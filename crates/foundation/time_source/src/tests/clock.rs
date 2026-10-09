//! Tests of the platform clock's plain-function reading: it is the platform clock's own reading,
//! in milliseconds since the Unix epoch.

use super::*;

#[test]
fn wall_clock_ms_is_the_platform_clock_reading() {
    let before = PlatformClock.now_unix_ms_f64();
    let read = wall_clock_ms();
    let after = PlatformClock.now_unix_ms_f64();
    assert!(
        (before..=after).contains(&read),
        "{before} <= {read} <= {after}"
    );
    assert_eq!(
        read.fract(),
        0.0,
        "the native platform clock reads whole milliseconds"
    );
    // 2026-01-01T00:00:00Z: a reading in seconds or microseconds would fall outside this window.
    assert!(read > 1_767_225_600_000.0, "{read}");
    assert!(read < 1.0e14, "{read}");
}

#[test]
fn wall_clock_ms_fills_a_function_pointer_slot() {
    let slot: fn() -> f64 = wall_clock_ms;
    assert!(slot() > 0.0);
}
