use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Positive seconds parse (whitespace trimmed); unset, zero, negative and garbage fall back
/// to the default interval.
#[test]
fn resync_interval_parses_positive_seconds_and_falls_back_otherwise() {
    let default = Duration::from_secs(DEFAULT_ROLE_RESYNC_SECS);
    let cases = [
        (None, default),
        (Some("30"), Duration::from_secs(30)),
        (Some(" 120 "), Duration::from_secs(120)),
        (Some("0"), default),
        (Some("-1"), default),
        (Some("nope"), default),
        (Some(""), default),
    ];
    for (raw, expected) in cases {
        assert_eq!(role_resync_interval_from(raw), expected, "{raw:?}");
    }
}

/// Perturbation: a stub resync is invoked on boot and again after each interval
/// tick, proving the scheduler path (not only admin POST) drives resync.
#[tokio::test]
async fn scheduler_invokes_resync_on_boot_and_interval() {
    let calls = Arc::new(AtomicUsize::new(0));
    let calls_c = calls.clone();

    // Lazy pool — never connects; the stub never touches SQL.
    let pool = api_database::connect_lazy("postgres://role-resync-test/unused").expect("lazy pool");

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
