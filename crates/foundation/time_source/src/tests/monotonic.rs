//! Tests of the native monotonic source: it starts near zero, never runs backwards, and measures
//! a sleep.

use std::time::Duration;

use super::*;

#[test]
fn never_runs_backwards() {
    let mut previous = monotonic_ms();
    assert!(previous >= 0.0);
    for _ in 0..10_000 {
        let next = monotonic_ms();
        assert!(next >= previous, "{next} < {previous}");
        previous = next;
    }
}

#[test]
fn measures_elapsed_time() {
    let start = monotonic_ms();
    std::thread::sleep(Duration::from_millis(20));
    let elapsed = monotonic_ms() - start;
    assert!(elapsed >= 20.0, "{elapsed}");
    assert!(elapsed < 60_000.0, "{elapsed}");
}

#[test]
fn fits_a_function_pointer_clock_slot() {
    let slot: fn() -> f64 = monotonic_ms;
    assert!(slot() >= 0.0);
}
