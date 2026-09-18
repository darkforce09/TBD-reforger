use super::*;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::core::database::connect_lazy;

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
    let pool = connect_lazy("postgres://leaderboard-scheduler/unused").expect("lazy pool");

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
