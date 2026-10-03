//! Unit tests of the grouping clock and the depth cap math.

use super::*;
use std::sync::Arc;
use time_source::ManualClock;
use yrs::sync::Clock as _;

#[test]
fn hidden_prefix_math_drops_oldest_whole_group() {
    assert_eq!(hidden_prefix_after(200, 0), 0);
    assert_eq!(hidden_prefix_after(201, 0), 1);
    assert_eq!(hidden_prefix_after(205, 1), 5);
    assert_eq!(hidden_prefix_after(199, 5), 5);
}

#[test]
fn grouping_clock_never_reads_zero() {
    let host = Arc::new(ManualClock::new(0));
    let clock = GroupingClock::wrap(host.clone());
    assert_eq!(clock.now(), 1, "a host reading of 0 is floored to 1 ms");
    host.set(1_000);
    assert_eq!(clock.now(), 1_000);
}

#[test]
fn open_group_freezes_the_reading_until_the_outermost_end() {
    let host = Arc::new(ManualClock::new(1_000));
    let clock = GroupingClock::wrap(host.clone());
    clock.begin_group();
    clock.begin_group();
    host.advance(GESTURE_WINDOW_MS * 4);
    assert_eq!(clock.now(), 1_000, "an open group reads its anchor");
    assert!(
        !clock.end_group(),
        "the inner end leaves the outer group open"
    );
    assert_eq!(clock.now(), 1_000);
    assert!(clock.end_group(), "the outermost end reports the close");
    assert_eq!(clock.now(), 1_000 + GESTURE_WINDOW_MS * 4);
    assert!(!clock.end_group(), "an end with no open group is a no-op");
}
