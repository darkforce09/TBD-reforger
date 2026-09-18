use super::*;

use std::sync::atomic::{AtomicUsize, Ordering};

use chrono::Utc;
use uuid::Uuid;

use crate::core::database::connect_lazy;
use crate::core::realtime_hub::server_status_topic::publish_server_status;
use crate::models::ServerStatus;

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

/// Perturbation RED: when publish is a no-op, a subscriber receives nothing. Dropping this
/// assertion (or restoring a publish) is what makes the GREEN test below load-bearing.
#[tokio::test]
async fn perturbation_no_publish_delivers_zero_frames() {
    let hub = Arc::new(Hub::new());
    let id = Uuid::parse_str("00000000-0000-4000-d000-000000000099").unwrap();
    let topic = format!("server:{id}");
    let mut rx = hub.subscribe(&topic);

    // Stub tick that *intentionally* does not call publish_server_status.
    let pool = connect_lazy("postgres://publisher-perturb/unused").expect("lazy pool");
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

    let pool = connect_lazy("postgres://publisher-scheduler/unused").expect("lazy pool");
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
