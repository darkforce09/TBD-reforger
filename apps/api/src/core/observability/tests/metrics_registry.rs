//! Accumulation invariants that need no HTTP: the in-flight guard and the cardinality cap.

use std::sync::Arc;
use std::time::Duration;

use super::Registry;

/// The in-flight gauge must come back down — including when a handler panics, which
/// unwinds straight through the metrics middleware.
#[test]
fn in_flight_guard_decrements_on_unwind() {
    let reg = Arc::new(Registry::new());
    assert_eq!(reg.in_flight(), 0);
    let r = std::panic::catch_unwind({
        let reg = reg.clone();
        move || {
            let _g = reg.enter();
            assert_eq!(reg.in_flight(), 1);
            panic!("handler exploded");
        }
    });
    assert!(r.is_err());
    assert_eq!(reg.in_flight(), 0, "gauge leaked across a panic");
}

/// The cardinality backstop actually drops, and says so.
#[test]
fn cardinality_cap_drops_and_counts() {
    let reg = Registry::new();
    for i in 0..(Registry::MAX_SERIES + 5) {
        reg.record("GET", &format!("/r{i}"), 200, Duration::from_millis(1));
    }
    assert_eq!(
        reg.requests_total("GET", "/r0", 200),
        1,
        "early series kept"
    );
    let over = format!("/r{}", Registry::MAX_SERIES + 1);
    assert_eq!(
        reg.requests_total("GET", &over, 200),
        0,
        "series past the cap must not exist"
    );
    assert!(
        reg.dropped_series() >= 5,
        "drops went uncounted: {}",
        reg.dropped_series()
    );
}
