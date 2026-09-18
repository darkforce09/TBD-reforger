//! In-process publish/subscribe hub — Rust port of `internal/realtime`.
//!
//! Fans server-status updates out to SSE clients. Single-instance only (same caveat
//! as the Go original — scale-out would back this with Postgres LISTEN/NOTIFY or
//! Redis). Backed by `tokio::sync::broadcast`: a bounded ring buffer per topic gives
//! Go's "buffer 16, non-blocking, drop slow subscribers" behavior for free, and a
//! dropped receiver auto-unsubscribes (no explicit cancel needed).
//!
//! # T-272 — scheduled republish
//!
//! Ingest (`POST /ingest/server-status`) is the only *writer* of live rows, but nothing
//! in-repo calls it (no game-server bridge yet). Without a second producer, SSE clients
//! connect, flip `connected=true`, and receive zero frames forever after the optional
//! one-shot snapshot. [`start_server_status_publisher`] closes that loop: boot + interval
//! poll of `server_statuses` → [`publish_server_status`], same payload shape as ingest.
//! Ingest still publishes in-request; this is the safety net.

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use sqlx::PgPool;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

use crate::models::ServerStatus;

/// Per-topic ring-buffer capacity (matches Go's `make(chan []byte, 16)`).
const TOPIC_BUFFER: usize = 16;

/// Env var for the background server-status → SSE republish cadence (seconds).
///
/// Default [`DEFAULT_SERVER_STATUS_PUBLISH_SECS`] = 10 s — frequent enough that Server
/// Intel's live panel updates without an external ingest heartbeat, long enough that an
/// empty `server_statuses` table does not thrash the DB.
pub const SERVER_STATUS_PUBLISH_INTERVAL_ENV: &str = "SERVER_STATUS_PUBLISH_INTERVAL_SECS";

/// Default scheduled republish interval: 10 seconds.
pub const DEFAULT_SERVER_STATUS_PUBLISH_SECS: u64 = 10;

/// SQL that both the SSE snapshot and the scheduled publisher use — one shape, one cast.
const SELECT_SERVER_STATUSES: &str = "SELECT server_id, is_online, player_count, max_players, \
     server_fps::float8 AS server_fps, uptime_seconds, current_match_id, \
     COALESCE(ingame_time, '') AS ingame_time, COALESCE(ingame_weather, '') AS ingame_weather, \
     COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at \
     FROM server_statuses";

/// Fans messages out to subscribers grouped by topic.
pub struct Hub {
    topics: Mutex<HashMap<String, broadcast::Sender<Vec<u8>>>>,
}

impl Hub {
    /// Create an empty hub.
    pub fn new() -> Self {
        Self {
            topics: Mutex::new(HashMap::new()),
        }
    }

    /// Subscribe to a topic. The returned receiver auto-unsubscribes when dropped
    /// (the SSE handler holds it for the life of the connection).
    pub fn subscribe(&self, topic: &str) -> broadcast::Receiver<Vec<u8>> {
        let mut topics = self.topics.lock().expect("hub lock");
        topics
            .entry(topic.to_string())
            .or_insert_with(|| broadcast::channel(TOPIC_BUFFER).0)
            .subscribe()
    }

    /// Deliver `msg` to all current subscribers of `topic`. Non-blocking: a slow
    /// subscriber whose buffer is full lags and drops rather than blocking the
    /// publisher. When no subscribers remain, the topic is pruned (mirrors Go's
    /// empty-topic delete on the last unsubscribe).
    pub fn publish(&self, topic: &str, msg: Vec<u8>) {
        let mut topics = self.topics.lock().expect("hub lock");
        if let Some(tx) = topics.get(topic)
            && tx.send(msg).is_err()
        {
            topics.remove(topic);
        }
    }
}

impl Default for Hub {
    fn default() -> Self {
        Self::new()
    }
}

/// Serialize `status` and fan it out on `server:{id}` — the exact bytes ingest and the
/// scheduled publisher both put on the wire (and that `sse.rs` / `decode_server_status_frame`
/// consume).
pub fn publish_server_status(hub: &Hub, status: &ServerStatus) {
    if let Ok(payload) = serde_json::to_vec(status) {
        hub.publish(&format!("server:{}", status.server_id), payload);
    }
}

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

/// Load every `server_statuses` row and publish each to its SSE topic.
///
/// Failures are returned to the caller (the scheduler logs and retries next tick). An empty
/// table is success with zero publishes — there is nothing to fan out until ingest (or a
/// seed) writes a row.
pub async fn publish_all_server_statuses(pool: &PgPool, hub: &Hub) -> Result<usize, sqlx::Error> {
    let rows: Vec<ServerStatus> = sqlx::query_as(SELECT_SERVER_STATUSES)
        .fetch_all(pool)
        .await?;
    let n = rows.len();
    for status in &rows {
        publish_server_status(hub, status);
    }
    Ok(n)
}

/// Spawn the background server-status → SSE republisher: one immediate poll (so a quiet
/// ingest path still delivers frames to connected clients), then every `interval` until
/// the runtime stops. Failures are logged; the next tick retries.
///
/// Ingest callers of [`publish_server_status`] are unchanged — this is the safety net that
/// closes the SSE loop without an external game-server bridge.
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
        // `interval` fires immediately on first `tick`; we already published above.
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
mod tests {
    use super::*;
    use chrono::Utc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use uuid::Uuid;

    fn sample_status(id: Uuid) -> ServerStatus {
        ServerStatus {
            server_id: id,
            is_online: true,
            player_count: 12,
            max_players: 64,
            server_fps: 58.7,
            uptime_seconds: 100,
            current_match_id: None,
            ingame_time: "06:42".into(),
            ingame_weather: "overcast".into(),
            updated_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn publish_delivers() {
        let h = Hub::new();
        let mut rx = h.subscribe("topic-a");
        h.publish("topic-a", b"hello".to_vec());
        assert_eq!(rx.recv().await.unwrap(), b"hello");
    }

    #[tokio::test]
    async fn topic_isolation() {
        let h = Hub::new();
        let mut rx = h.subscribe("topic-a");
        h.publish("topic-b", b"nope".to_vec());
        assert!(rx.try_recv().is_err(), "no cross-topic delivery");
    }

    #[tokio::test]
    async fn unsubscribe_stops_delivery() {
        let h = Hub::new();
        let rx = h.subscribe("topic-a");
        drop(rx); // cancel
        h.publish("topic-a", b"x".to_vec()); // must not panic
    }

    #[test]
    fn publish_interval_default_when_unset() {
        assert_eq!(
            server_status_publish_interval_from(None),
            Duration::from_secs(DEFAULT_SERVER_STATUS_PUBLISH_SECS)
        );
    }

    #[test]
    fn publish_interval_parses_positive_secs() {
        assert_eq!(
            server_status_publish_interval_from(Some("5")),
            Duration::from_secs(5)
        );
        assert_eq!(
            server_status_publish_interval_from(Some(" 30 ")),
            Duration::from_secs(30)
        );
    }

    #[test]
    fn publish_interval_rejects_zero_negative_garbage() {
        let def = Duration::from_secs(DEFAULT_SERVER_STATUS_PUBLISH_SECS);
        assert_eq!(server_status_publish_interval_from(Some("0")), def);
        assert_eq!(server_status_publish_interval_from(Some("-1")), def);
        assert_eq!(server_status_publish_interval_from(Some("nope")), def);
        assert_eq!(server_status_publish_interval_from(Some("")), def);
    }

    /// Payload shape pin: the helper must put the same JSON on the wire that ingest used to
    /// build inline — topic `server:{uuid}` and a body `ServerStatus` deserializes.
    #[tokio::test]
    async fn publish_server_status_matches_ingest_shape() {
        let hub = Hub::new();
        let id = Uuid::parse_str("00000000-0000-4000-d000-000000000001").unwrap();
        let status = sample_status(id);
        let mut rx = hub.subscribe(&format!("server:{id}"));
        publish_server_status(&hub, &status);
        let bytes = rx.recv().await.expect("frame delivered");
        let back: ServerStatus =
            serde_json::from_slice(&bytes).expect("payload must be ServerStatus JSON");
        assert_eq!(back.server_id, id);
        assert!(back.is_online);
        assert_eq!(back.player_count, 12);
        assert_eq!(back.server_fps, 58.7);
        assert_eq!(back.ingame_time, "06:42");
    }

    /// Perturbation RED: when publish is a no-op, a subscriber receives nothing. Dropping
    /// this assertion (or restoring a publish) is what makes the GREEN test below load-bearing.
    #[tokio::test]
    async fn perturbation_no_publish_delivers_zero_frames() {
        let hub = Arc::new(Hub::new());
        let id = Uuid::parse_str("00000000-0000-4000-d000-000000000099").unwrap();
        let topic = format!("server:{id}");
        let mut rx = hub.subscribe(&topic);

        // Stub tick that *intentionally* does not call publish_server_status.
        let pool = crate::db::connect_lazy("postgres://t272-perturb/unused").expect("lazy pool");
        let handle = start_server_status_publisher_with(
            pool,
            Arc::clone(&hub),
            Duration::from_millis(40),
            |_p, _h| async move { Ok(()) },
        );

        tokio::time::sleep(Duration::from_millis(120)).await;
        assert!(
            rx.try_recv().is_err(),
            "perturbation: stub that skips publish must deliver zero frames"
        );

        handle.abort();
        let _ = handle.await;
    }

    /// Scheduler path (not only ingest) drives publish: boot + at least one interval tick.
    #[tokio::test]
    async fn scheduler_publishes_on_boot_and_interval() {
        let calls = Arc::new(AtomicUsize::new(0));
        let calls_c = calls.clone();
        let hub = Arc::new(Hub::new());
        let id = Uuid::parse_str("00000000-0000-4000-d000-000000000042").unwrap();
        let topic = format!("server:{id}");
        let mut rx = hub.subscribe(&topic);

        let pool = crate::db::connect_lazy("postgres://t272-scheduler/unused").expect("lazy pool");
        let handle = start_server_status_publisher_with(
            pool,
            Arc::clone(&hub),
            Duration::from_millis(40),
            move |_p, h| {
                let calls = calls_c.clone();
                let status = sample_status(id);
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    publish_server_status(&h, &status);
                    Ok(())
                }
            },
        );

        wait_until(
            || calls.load(Ordering::SeqCst) >= 1,
            Duration::from_millis(500),
        )
        .await;
        let first = rx.recv().await.expect("boot publish frame");
        let back: ServerStatus = serde_json::from_slice(&first).unwrap();
        assert_eq!(back.server_id, id);

        wait_until(
            || calls.load(Ordering::SeqCst) >= 2,
            Duration::from_millis(500),
        )
        .await;
        let second = rx.recv().await.expect("interval publish frame");
        assert!(!second.is_empty(), "interval tick must deliver a frame");

        handle.abort();
        let _ = handle.await;
    }

    async fn wait_until(mut pred: impl FnMut() -> bool, budget: Duration) {
        let start = tokio::time::Instant::now();
        while !pred() {
            assert!(
                start.elapsed() < budget,
                "timed out waiting for server-status publisher"
            );
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    }
}
