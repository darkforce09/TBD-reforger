//! Database layer — Rust port of `internal/db` (pool + migration runner + MV refresh).
//!
//! The migration pipeline is a single frozen `migrations/0001_initial_schema.sql`
//! (the Go GORM-AutoMigrate + raw-SQL schema, proven byte-equal by gate G2). sqlx
//! embeds it at compile time via `migrate!`; future schema changes add new files.

use std::future::Future;
use std::time::Duration;

use sqlx::postgres::{PgPool, PgPoolOptions};
use tokio::task::JoinHandle;

/// Startup connection retry budget (mirrors `db.Open`: 10 attempts, linear backoff).
const CONNECT_ATTEMPTS: u32 = 10;

/// Env var for the background leaderboard MV refresh cadence (seconds).
///
/// Default [`DEFAULT_LEADERBOARD_REFRESH_SECS`] = 15 minutes — short enough that a quiet
/// ingest path cannot leave `leaderboard_totals` stuck at `WITH NO DATA` (or stale) for
/// long, long enough that concurrent refreshes do not thrash the DB under load.
pub const LEADERBOARD_REFRESH_INTERVAL_ENV: &str = "LEADERBOARD_REFRESH_INTERVAL_SECS";

/// Default scheduled refresh interval: 15 minutes.
pub const DEFAULT_LEADERBOARD_REFRESH_SECS: u64 = 15 * 60;

/// Connect to Postgres, tuning the pool and retrying the initial connection with
/// linear backoff (Postgres can briefly refuse connections just after reporting ready).
///
/// Mirrors `db.Open`: MaxOpen 25, ConnMaxLifetime 30m, ConnMaxIdleTime 5m.
pub async fn connect(database_url: &str) -> Result<PgPool, sqlx::Error> {
    let opts = PgPoolOptions::new()
        .max_connections(25)
        .idle_timeout(Duration::from_secs(5 * 60))
        .max_lifetime(Duration::from_secs(30 * 60))
        .acquire_timeout(Duration::from_secs(30));

    let mut last_err: Option<sqlx::Error> = None;
    for attempt in 1..=CONNECT_ATTEMPTS {
        match opts.clone().connect(database_url).await {
            Ok(pool) => return Ok(pool),
            Err(e) => {
                last_err = Some(e);
                tokio::time::sleep(Duration::from_millis(u64::from(attempt) * 250)).await;
            }
        }
    }
    Err(last_err.expect("loop runs at least once"))
}

/// Build a pool that connects lazily (on first use). Used by tests/harnesses that
/// exercise code paths not reaching the DB, without requiring a live server.
pub fn connect_lazy(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(25)
        .connect_lazy(database_url)
}

/// Run all pending migrations (embedded from `./migrations` at compile time).
///
/// Mirrors `db.Migrate` — the pre/AutoMigrate/post pipeline is collapsed into the
/// single frozen `0001_initial_schema.sql`.
pub async fn migrate(pool: &PgPool) -> Result<(), sqlx::migrate::MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

/// Refresh the `leaderboard_totals` materialized view. Call after match telemetry
/// ingest (debounced). Falls back to a non-concurrent refresh if the concurrent one
/// fails (e.g. the view has not been populated yet). Mirrors `db.RefreshLeaderboard`.
pub async fn refresh_leaderboard(pool: &PgPool) -> Result<(), sqlx::Error> {
    if sqlx::query("REFRESH MATERIALIZED VIEW CONCURRENTLY leaderboard_totals")
        .execute(pool)
        .await
        .is_err()
    {
        sqlx::query("REFRESH MATERIALIZED VIEW leaderboard_totals")
            .execute(pool)
            .await?;
    }
    Ok(())
}

/// Resolve the scheduled MV refresh interval from
/// [`LEADERBOARD_REFRESH_INTERVAL_ENV`], falling back to
/// [`DEFAULT_LEADERBOARD_REFRESH_SECS`]. Invalid / zero / negative values use the default.
pub fn leaderboard_refresh_interval() -> Duration {
    leaderboard_refresh_interval_from(
        std::env::var(LEADERBOARD_REFRESH_INTERVAL_ENV)
            .ok()
            .as_deref(),
    )
}

fn leaderboard_refresh_interval_from(raw: Option<&str>) -> Duration {
    match raw {
        Some(s) => match s.trim().parse::<u64>() {
            Ok(secs) if secs > 0 => Duration::from_secs(secs),
            _ => Duration::from_secs(DEFAULT_LEADERBOARD_REFRESH_SECS),
        },
        None => Duration::from_secs(DEFAULT_LEADERBOARD_REFRESH_SECS),
    }
}

/// Spawn the background leaderboard MV refresher: one immediate refresh (so a quiet
/// ingest path cannot leave `leaderboard_totals` at `WITH NO DATA`), then every
/// `interval` until the runtime stops. Failures are logged; the next tick retries.
///
/// Ingest callers of [`refresh_leaderboard`] are unchanged — this is a safety net.
pub fn start_leaderboard_refresh(pool: PgPool, interval: Duration) -> JoinHandle<()> {
    start_leaderboard_refresh_with(
        pool,
        interval,
        |p| async move { refresh_leaderboard(&p).await },
    )
}

/// Testable core of [`start_leaderboard_refresh`]: runs `refresh` immediately, then on
/// each interval tick. The production path wires [`refresh_leaderboard`].
fn start_leaderboard_refresh_with<F, Fut>(
    pool: PgPool,
    interval: Duration,
    refresh: F,
) -> JoinHandle<()>
where
    F: Fn(PgPool) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), sqlx::Error>> + Send + 'static,
{
    tokio::spawn(async move {
        run_refresh(&pool, &refresh).await;
        let mut ticker = tokio::time::interval(interval);
        // `interval` fires immediately on first `tick`; we already refreshed above.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            run_refresh(&pool, &refresh).await;
        }
    })
}

async fn run_refresh<F, Fut>(pool: &PgPool, refresh: &F)
where
    F: Fn(PgPool) -> Fut,
    Fut: Future<Output = Result<(), sqlx::Error>>,
{
    match refresh(pool.clone()).await {
        Ok(()) => tracing::debug!("leaderboard MV refresh ok"),
        Err(e) => tracing::error!(error = %e, "leaderboard MV scheduled refresh failed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn refresh_interval_default_when_unset() {
        assert_eq!(
            leaderboard_refresh_interval_from(None),
            Duration::from_secs(DEFAULT_LEADERBOARD_REFRESH_SECS)
        );
    }

    #[test]
    fn refresh_interval_parses_positive_secs() {
        assert_eq!(
            leaderboard_refresh_interval_from(Some("30")),
            Duration::from_secs(30)
        );
        assert_eq!(
            leaderboard_refresh_interval_from(Some(" 120 ")),
            Duration::from_secs(120)
        );
    }

    #[test]
    fn refresh_interval_rejects_zero_negative_garbage() {
        let def = Duration::from_secs(DEFAULT_LEADERBOARD_REFRESH_SECS);
        assert_eq!(leaderboard_refresh_interval_from(Some("0")), def);
        assert_eq!(leaderboard_refresh_interval_from(Some("-1")), def);
        assert_eq!(leaderboard_refresh_interval_from(Some("nope")), def);
        assert_eq!(leaderboard_refresh_interval_from(Some("")), def);
    }

    /// Perturbation: a stub refresh is invoked on boot and again after each interval tick,
    /// proving the scheduler path (not only ingest) drives refresh.
    ///
    /// Uses a real short interval (no `tokio` `test-util` feature on this crate) and polls
    /// until the expected call counts land.
    #[tokio::test]
    async fn scheduler_invokes_refresh_on_boot_and_interval() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_c = calls.clone();

        // Lazy pool — never connects; the stub never touches SQL.
        let pool = connect_lazy("postgres://t261-scheduler-test/unused").expect("lazy pool");

        let handle = start_leaderboard_refresh_with(pool, Duration::from_millis(40), move |_p| {
            let calls = calls_c.clone();
            async move {
                calls.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        });

        // Boot refresh runs before the first ticker wait.
        wait_until(
            || calls.load(Ordering::SeqCst) >= 1,
            Duration::from_millis(500),
        )
        .await;
        assert!(calls.load(Ordering::SeqCst) >= 1, "immediate boot refresh");

        // At least one interval tick after boot.
        wait_until(
            || calls.load(Ordering::SeqCst) >= 2,
            Duration::from_millis(500),
        )
        .await;
        assert!(
            calls.load(Ordering::SeqCst) >= 2,
            "interval tick refresh, got {}",
            calls.load(Ordering::SeqCst)
        );

        handle.abort();
        let _ = handle.await;
    }

    /// T-940.5 acceptance — `TBD_DB_POOL_MAX_CONNECTIONS=3` must yield a pool whose max is 3 and
    /// unset must yield 25. `connect` performs its first connection eagerly, so this needs a live
    /// database and reads `TEST_DATABASE_URL` exactly like every DB-backed suite under `tests/`
    /// (unset = the harness's skip path; the gate and `db test-it` both provide a database). It
    /// only opens pools and reads their options — no migration, no writes — so it needs no
    /// scratch-name guard.
    #[tokio::test]
    async fn pool_max_connections_follows_env_override() {
        let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
            eprintln!("skip: TEST_DATABASE_URL unset");
            return;
        };
        // SAFETY: this is the only test in the binary that touches `TBD_DB_POOL_*`; both writes
        // happen on this thread, every reader is Rust `std::env` (internally locked), and nothing
        // in the process calls `getenv` from C during the window.
        unsafe { std::env::set_var("TBD_DB_POOL_MAX_CONNECTIONS", "3") };
        let overridden = connect(&url).await;
        unsafe { std::env::remove_var("TBD_DB_POOL_MAX_CONNECTIONS") };
        let overridden = overridden.expect("connect with TBD_DB_POOL_MAX_CONNECTIONS=3");
        let unset = connect(&url)
            .await
            .expect("connect with TBD_DB_POOL_MAX_CONNECTIONS unset");
        assert_eq!(
            overridden.options().get_max_connections(),
            3,
            "TBD_DB_POOL_MAX_CONNECTIONS=3 must yield a pool max of 3"
        );
        assert_eq!(
            unset.options().get_max_connections(),
            25,
            "unset TBD_DB_POOL_MAX_CONNECTIONS must yield the default 25"
        );
        overridden.close().await;
        unset.close().await;
    }

    async fn wait_until(mut pred: impl FnMut() -> bool, budget: Duration) {
        let start = tokio::time::Instant::now();
        while !pred() {
            assert!(
                start.elapsed() < budget,
                "timed out waiting for scheduler refresh"
            );
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }
}
