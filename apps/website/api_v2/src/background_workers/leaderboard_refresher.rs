//! Scheduled refresh of the `leaderboard_totals` materialized view.
//!
//! Ingest refreshes the view in-request; this worker is the safety net for the case where
//! ingest is quiet, which would otherwise leave the view at `WITH NO DATA` (or stale)
//! indefinitely.

use std::future::Future;
use std::time::Duration;

use sqlx::postgres::PgPool;
use tokio::task::JoinHandle;

use crate::core::database::leaderboard_refresh::refresh_leaderboard;

/// Env var for the background leaderboard MV refresh cadence (seconds).
///
/// Default [`DEFAULT_LEADERBOARD_REFRESH_SECS`] = 15 minutes — short enough that a quiet
/// ingest path cannot leave `leaderboard_totals` stuck for long, long enough that concurrent
/// refreshes do not thrash the DB under load.
pub const LEADERBOARD_REFRESH_INTERVAL_ENV: &str = "LEADERBOARD_REFRESH_INTERVAL_SECS";

/// Default scheduled refresh interval: 15 minutes.
pub const DEFAULT_LEADERBOARD_REFRESH_SECS: u64 = 15 * 60;

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
/// Ingest callers of [`refresh_leaderboard`] are unaffected — this is a safety net.
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
        // `interval` fires immediately on first `tick`; the boot refresh above already ran.
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
#[path = "tests/leaderboard_refresher.rs"]
mod tests;
