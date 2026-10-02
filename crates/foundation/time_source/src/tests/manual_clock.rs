//! Tests of the test clock: it stands still, and moves exactly as set and advanced, through a
//! shared `Arc` as well.

use std::sync::Arc;

use super::*;

#[test]
fn stands_still_until_moved() {
    let clock = ManualClock::new(1_000);
    assert_eq!(clock.now_unix_ms(), 1_000);
    assert_eq!(clock.now_unix_ms(), 1_000);
    assert_eq!(clock.now_unix_ms_f64(), 1_000.0);
    assert_eq!(ManualClock::default().now_unix_ms(), 0);
}

#[test]
fn moves_exactly_as_set_and_advanced() {
    let clock = ManualClock::new(1_000);
    clock.advance(300);
    assert_eq!(clock.now_unix_ms(), 1_300);
    clock.advance(1);
    assert_eq!(clock.now_unix_ms(), 1_301);
    clock.set(50);
    assert_eq!(clock.now_unix_ms(), 50, "set moves the clock backwards too");
    clock.set(u64::MAX);
    clock.advance(2);
    assert_eq!(
        clock.now_unix_ms(),
        1,
        "advance wraps like the atomic add it is"
    );
}

#[test]
fn a_shared_clock_moves_for_every_holder() {
    let clock = Arc::new(ManualClock::new(10));
    let reader: Arc<dyn Clock> = clock.clone();
    clock.advance(5);
    assert_eq!(reader.now_unix_ms(), 15);
    let mover = Arc::clone(&clock);
    std::thread::spawn(move || mover.set(99)).join().unwrap();
    assert_eq!(reader.now_unix_ms(), 99);
}
