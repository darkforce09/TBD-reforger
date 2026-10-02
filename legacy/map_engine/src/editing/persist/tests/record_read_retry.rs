//! Role: the retry schedule's exact delays.
//! Position: `editing/persist/tests` in the map engine.
//! Signals & state: none.
//! Invariants: the delays double and the schedule starts at the first attempt, so three attempts
//! span 560 ms in total — the window a save must not write inside.

use super::*;

#[test]
fn the_delay_doubles_on_every_attempt() {
    assert_eq!(backoff_before_attempt_ms(1), 80);
    assert_eq!(backoff_before_attempt_ms(2), 160);
    assert_eq!(backoff_before_attempt_ms(3), 320);
}

#[test]
fn three_attempts_span_the_documented_window() {
    let total: i32 = (1..=3).map(backoff_before_attempt_ms).sum();
    assert_eq!(total, 560);
}

/// A zero attempt is not a schedule position, and it must not shift by a negative amount.
#[test]
fn a_zero_attempt_falls_back_to_the_first_delay() {
    assert_eq!(backoff_before_attempt_ms(0), 80);
}
