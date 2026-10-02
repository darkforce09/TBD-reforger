//! Tests of the native wall clock: it reads the operating system clock to the millisecond and is
//! the platform clock natively.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use super::*;
use crate::clock::PlatformClock;

fn system_time_ms() -> u64 {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap()
}

#[test]
fn reads_the_operating_system_clock_in_milliseconds() {
    let before = system_time_ms();
    let read = SystemClock.now_unix_ms();
    let after = system_time_ms();
    assert!(
        (before..=after).contains(&read),
        "{before} <= {read} <= {after}"
    );
    // 2026-01-01T00:00:00Z: a reading in seconds or microseconds would fall outside this window.
    assert!(read > 1_767_225_600_000, "{read}");
    assert!(read < 32_503_680_000_000, "{read}");
}

#[test]
fn the_float_reading_is_the_integer_reading() {
    let clock = ManualReading(1_783_208_618_437);
    assert_eq!(clock.now_unix_ms_f64(), 1_783_208_618_437.0);
    let float = SystemClock.now_unix_ms_f64();
    assert_eq!(float.fract(), 0.0, "whole milliseconds: {float}");
}

#[test]
fn is_the_platform_clock_natively_and_object_safe() {
    let platform: PlatformClock = SystemClock;
    let shared: Arc<dyn Clock> = Arc::new(platform);
    let before = system_time_ms();
    assert!(shared.now_unix_ms() >= before);
    assert!(PlatformClock.now_unix_ms() >= before);
}

/// A clock with a fixed reading, to exercise the provided `f64` method.
struct ManualReading(u64);

impl Clock for ManualReading {
    fn now_unix_ms(&self) -> u64 {
        self.0
    }
}
