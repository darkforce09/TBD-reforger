//! Discord role → web role sync — Rust port of `services/role_sync.go`.
//!
//! # T-428 — scheduled resync
//!
//! Architecture claims a nightly cron that re-resolves every user's web tier from
//! stored Discord role snapshots against current `discord_roles` mappings
//! (`docs/website/backend/architecture.md` §Cross-Cutting). Admin
//! `POST /admin/roles/sync` already calls [`resync_all_roles`]; this module also
//! arms a boot + interval safety net so a remapped role promotes users without a
//! manual curl. OAuth login still syncs in-request.

use std::future::Future;
use std::time::Duration;

use sqlx::PgPool;
use tokio::task::JoinHandle;

use crate::models::UserRole;

/// Env var for the background Discord → web role resync cadence (seconds).
///
/// Default [`DEFAULT_ROLE_RESYNC_SECS`] = 24 h — matches the architecture "nightly
/// cron" claim. Long enough not to thrash under remaps; short enough that a quiet
/// admin path cannot leave remapped tiers stale for more than a day.
pub const ROLE_RESYNC_INTERVAL_ENV: &str = "ROLE_RESYNC_INTERVAL_SECS";

/// Default scheduled resync interval: 24 hours (nightly).
pub const DEFAULT_ROLE_RESYNC_SECS: u64 = 24 * 60 * 60;

/// Reconcile a user's Discord role snowflakes into `user_discord_roles` (unmapped
/// ids are still stored so a later admin mapping + resync promotes them), then
/// resolve their web role. Defaults to enlisted when nothing maps.
pub async fn sync_roles(
    pool: &PgPool,
    discord_id: &str,
    role_ids: &[String],
) -> sqlx::Result<UserRole> {
    let mut tx = pool.begin().await?;
    sqlx::query("DELETE FROM user_discord_roles WHERE discord_id = $1")
        .bind(discord_id)
        .execute(&mut *tx)
        .await?;
    for rid in role_ids {
        sqlx::query(
            "INSERT INTO user_discord_roles (discord_id, discord_role_id, synced_at) \
             VALUES ($1, $2, now()) ON CONFLICT DO NOTHING",
        )
        .bind(discord_id)
        .bind(rid)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    resolve_role(pool, role_ids).await
}

/// Re-resolve every user's web role from their stored Discord roles against current
/// mappings (used after an admin remaps a role). Returns the number changed.
pub async fn resync_all_roles(pool: &PgPool) -> sqlx::Result<i64> {
    let users: Vec<(String, UserRole)> =
        sqlx::query_as("SELECT discord_id, role FROM users WHERE deleted_at IS NULL")
            .fetch_all(pool)
            .await?;
    let mut updated = 0i64;
    for (discord_id, current) in users {
        let role_ids: Vec<String> = sqlx::query_scalar(
            "SELECT discord_role_id FROM user_discord_roles WHERE discord_id = $1",
        )
        .bind(&discord_id)
        .fetch_all(pool)
        .await?;
        let role = resolve_role(pool, &role_ids).await?;
        if role != current {
            sqlx::query("UPDATE users SET role = $1, updated_at = now() WHERE discord_id = $2")
                .bind(role)
                .bind(&discord_id)
                .execute(pool)
                .await?;
            updated += 1;
        }
    }
    Ok(updated)
}

/// The highest-priority mapped web role among the given Discord role ids, or
/// enlisted if none are mapped.
pub async fn resolve_role(pool: &PgPool, role_ids: &[String]) -> sqlx::Result<UserRole> {
    if role_ids.is_empty() {
        return Ok(UserRole::Enlisted);
    }
    let mapped: Option<Option<UserRole>> = sqlx::query_scalar(
        "SELECT mapped_role FROM discord_roles \
         WHERE discord_role_id = ANY($1) AND mapped_role IS NOT NULL \
         ORDER BY priority DESC LIMIT 1",
    )
    .bind(role_ids)
    .fetch_optional(pool)
    .await?;
    Ok(mapped.flatten().unwrap_or(UserRole::Enlisted))
}

/// Resolve the scheduled resync interval from [`ROLE_RESYNC_INTERVAL_ENV`], falling
/// back to [`DEFAULT_ROLE_RESYNC_SECS`]. Invalid / zero / negative values use the default.
pub fn role_resync_interval() -> Duration {
    role_resync_interval_from(std::env::var(ROLE_RESYNC_INTERVAL_ENV).ok().as_deref())
}

fn role_resync_interval_from(raw: Option<&str>) -> Duration {
    match raw {
        Some(s) => match s.trim().parse::<u64>() {
            Ok(secs) if secs > 0 => Duration::from_secs(secs),
            _ => Duration::from_secs(DEFAULT_ROLE_RESYNC_SECS),
        },
        None => Duration::from_secs(DEFAULT_ROLE_RESYNC_SECS),
    }
}

/// Spawn the background Discord → web role resyncer: one immediate pass (so a remap
/// that landed while the API was down is applied on boot), then every `interval`
/// until the runtime stops. Failures are logged; the next tick retries.
///
/// Admin `POST /admin/roles/sync` is unchanged — this is the nightly safety net the
/// architecture doc claimed.
pub fn start_role_resync(pool: PgPool, interval: Duration) -> JoinHandle<()> {
    start_role_resync_with(
        pool,
        interval,
        |p| async move { resync_all_roles(&p).await },
    )
}

/// Testable core of [`start_role_resync`]: runs `resync` immediately, then on each
/// interval tick. The production path wires [`resync_all_roles`].
fn start_role_resync_with<F, Fut>(pool: PgPool, interval: Duration, resync: F) -> JoinHandle<()>
where
    F: Fn(PgPool) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = sqlx::Result<i64>> + Send + 'static,
{
    tokio::spawn(async move {
        run_resync(&pool, &resync).await;
        let mut ticker = tokio::time::interval(interval);
        // `interval` fires immediately on first `tick`; we already resynced above.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            run_resync(&pool, &resync).await;
        }
    })
}

async fn run_resync<F, Fut>(pool: &PgPool, resync: &F)
where
    F: Fn(PgPool) -> Fut,
    Fut: Future<Output = sqlx::Result<i64>>,
{
    match resync(pool.clone()).await {
        Ok(n) => tracing::info!(updated = n, "discord role resync ok"),
        Err(e) => tracing::error!(error = %e, "discord role scheduled resync failed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn resync_interval_default_when_unset() {
        assert_eq!(
            role_resync_interval_from(None),
            Duration::from_secs(DEFAULT_ROLE_RESYNC_SECS)
        );
    }

    #[test]
    fn resync_interval_parses_positive_secs() {
        assert_eq!(
            role_resync_interval_from(Some("30")),
            Duration::from_secs(30)
        );
        assert_eq!(
            role_resync_interval_from(Some(" 120 ")),
            Duration::from_secs(120)
        );
    }

    #[test]
    fn resync_interval_rejects_zero_negative_garbage() {
        let def = Duration::from_secs(DEFAULT_ROLE_RESYNC_SECS);
        assert_eq!(role_resync_interval_from(Some("0")), def);
        assert_eq!(role_resync_interval_from(Some("-1")), def);
        assert_eq!(role_resync_interval_from(Some("nope")), def);
        assert_eq!(role_resync_interval_from(Some("")), def);
    }

    /// Perturbation: a stub resync is invoked on boot and again after each interval
    /// tick, proving the scheduler path (not only admin POST) drives resync.
    #[tokio::test]
    async fn scheduler_invokes_resync_on_boot_and_interval() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_c = calls.clone();

        // Lazy pool — never connects; the stub never touches SQL.
        let pool =
            crate::db::connect_lazy("postgres://t428-scheduler-test/unused").expect("lazy pool");

        let handle = start_role_resync_with(pool, Duration::from_millis(40), move |_p| {
            let calls = calls_c.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(0_i64)
            }
        });

        wait_until(
            || calls.load(Ordering::SeqCst) >= 1,
            Duration::from_millis(500),
        )
        .await;
        assert!(calls.load(Ordering::SeqCst) >= 1, "immediate boot resync");

        wait_until(
            || calls.load(Ordering::SeqCst) >= 2,
            Duration::from_millis(500),
        )
        .await;
        assert!(
            calls.load(Ordering::SeqCst) >= 2,
            "interval tick resync, got {}",
            calls.load(Ordering::SeqCst)
        );

        handle.abort();
        let _ = handle.await;
    }

    async fn wait_until(mut pred: impl FnMut() -> bool, budget: Duration) {
        let start = tokio::time::Instant::now();
        while !pred() {
            assert!(
                start.elapsed() < budget,
                "timed out waiting for scheduler resync"
            );
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }
}
