//! Scheduled Discord → web role resync.
//!
//! Re-resolves every user's web tier from their stored Discord role snapshots against the
//! current `discord_roles` mappings, so a remapped role promotes users without an admin
//! calling `POST /admin/roles/sync`. OAuth login still syncs in-request; this is the safety
//! net for the quiet path.

use std::future::Future;
use std::time::Duration;

use sqlx::PgPool;
use tokio::task::JoinHandle;

use crate::identity_and_access::services::discord_role_sync::resync_all_roles;

/// Env var for the background Discord → web role resync cadence (seconds).
///
/// Default [`DEFAULT_ROLE_RESYNC_SECS`] = 24 h. Long enough not to thrash under remaps;
/// short enough that a quiet admin path cannot leave remapped tiers stale for more than a day.
pub const ROLE_RESYNC_INTERVAL_ENV: &str = "ROLE_RESYNC_INTERVAL_SECS";

/// Default scheduled resync interval: 24 hours (nightly).
pub const DEFAULT_ROLE_RESYNC_SECS: u64 = 24 * 60 * 60;

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

/// Spawn the resyncer: one immediate pass (so a remap that landed while the API was down is
/// applied on boot), then every `interval` until the runtime stops. Failures are logged; the
/// next tick retries.
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
        // `interval` fires immediately on first `tick`; the boot resync above already ran.
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
#[path = "tests/discord_role_synchronizer.rs"]
mod tests;
