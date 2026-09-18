//! Scheduled republish of `server_statuses` rows onto their SSE topics.
//!
//! Ingest (`POST /ingest/server-status`) is the only writer of live rows and publishes
//! in-request. Without a second producer, an SSE client that connects while ingest is quiet
//! flips to `connected` and then receives nothing after the optional one-shot snapshot. This
//! worker closes that loop: boot poll plus interval poll of `server_statuses`, same payload
//! shape as ingest.

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use sqlx::PgPool;
use tokio::task::JoinHandle;

use crate::core::realtime_hub::Hub;
use crate::server_infrastructure::services::status_broadcast::publish_all_server_statuses;

/// Env var for the background server-status → SSE republish cadence (seconds).
///
/// Default [`DEFAULT_SERVER_STATUS_PUBLISH_SECS`] = 10 s — frequent enough that the live
/// server panel updates without an external ingest heartbeat, long enough that an empty
/// `server_statuses` table does not thrash the DB.
pub const SERVER_STATUS_PUBLISH_INTERVAL_ENV: &str = "SERVER_STATUS_PUBLISH_INTERVAL_SECS";

/// Default scheduled republish interval: 10 seconds.
pub const DEFAULT_SERVER_STATUS_PUBLISH_SECS: u64 = 10;

/// Resolve the scheduled republish interval from
/// [`SERVER_STATUS_PUBLISH_INTERVAL_ENV`], falling back to
/// [`DEFAULT_SERVER_STATUS_PUBLISH_SECS`]. Invalid / zero / negative values use the default.
pub fn server_status_publish_interval() -> Duration {
    server_status_publish_interval_from(
        std::env::var(SERVER_STATUS_PUBLISH_INTERVAL_ENV)
            .ok()
            .as_deref(),
    )
}

fn server_status_publish_interval_from(raw: Option<&str>) -> Duration {
    match raw {
        Some(s) => match s.trim().parse::<u64>() {
            Ok(secs) if secs > 0 => Duration::from_secs(secs),
            _ => Duration::from_secs(DEFAULT_SERVER_STATUS_PUBLISH_SECS),
        },
        None => Duration::from_secs(DEFAULT_SERVER_STATUS_PUBLISH_SECS),
    }
}

/// Spawn the background server-status → SSE republisher: one immediate poll (so a quiet
/// ingest path still delivers frames to connected clients), then every `interval` until
/// the runtime stops. Failures are logged; the next tick retries.
///
/// Ingest's in-request publishes are unaffected — this is the safety net that closes the SSE
/// loop without an external game-server bridge.
pub fn start_server_status_publisher(
    pool: PgPool,
    hub: Arc<Hub>,
    interval: Duration,
) -> JoinHandle<()> {
    start_server_status_publisher_with(pool, hub, interval, |p, h| async move {
        publish_all_server_statuses(&p, &h).await.map(|_| ())
    })
}

/// Testable core of [`start_server_status_publisher`]: runs `tick` immediately, then on
/// each interval. The production path wires [`publish_all_server_statuses`].
fn start_server_status_publisher_with<F, Fut>(
    pool: PgPool,
    hub: Arc<Hub>,
    interval: Duration,
    tick: F,
) -> JoinHandle<()>
where
    F: Fn(PgPool, Arc<Hub>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), sqlx::Error>> + Send + 'static,
{
    tokio::spawn(async move {
        run_publish_tick(&pool, &hub, &tick).await;
        let mut ticker = tokio::time::interval(interval);
        // `interval` fires immediately on first `tick`; the boot publish above already ran.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            run_publish_tick(&pool, &hub, &tick).await;
        }
    })
}

async fn run_publish_tick<F, Fut>(pool: &PgPool, hub: &Arc<Hub>, tick: &F)
where
    F: Fn(PgPool, Arc<Hub>) -> Fut,
    Fut: Future<Output = Result<(), sqlx::Error>>,
{
    match tick(pool.clone(), Arc::clone(hub)).await {
        Ok(()) => tracing::debug!("server-status SSE republish ok"),
        Err(e) => tracing::error!(error = %e, "server-status SSE scheduled republish failed"),
    }
}

#[cfg(test)]
#[path = "tests/server_status_publisher.rs"]
mod tests;
