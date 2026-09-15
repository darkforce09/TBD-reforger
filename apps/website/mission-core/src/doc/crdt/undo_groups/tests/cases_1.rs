//! Role: Domain regression cases.
//! Position: `doc/crdt/undo_groups/tests` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn three_position_ops_within_50ms_are_one_undo_group() {
    let clock = ManualClock::new(1_000);
    let mut doc = slot_doc(clock.clone());
    doc.move_entities(vec!["s0".to_string()], 10.0, 0.0, vec![0.0]);
    clock.advance(25);
    doc.move_entities(vec!["s0".to_string()], 10.0, 0.0, vec![0.0]);
    clock.advance(25);
    doc.move_entities(vec!["s0".to_string()], 10.0, 0.0, vec![0.0]);
    assert_eq!(
        doc.undo_depth(),
        1,
        "T-937.2: three position ops within 50ms are ONE undo group; got {}",
        doc.undo_depth()
    );
    assert!(doc.undo());
    assert!((x_of(&doc) - 100.0).abs() < f32::EPSILON);
    assert!(!doc.can_undo());
}

#[test]
fn edits_separated_by_more_than_the_window_are_two_groups() {
    let clock = ManualClock::new(1_000);
    let doc = slot_doc(clock.clone());
    doc.move_entities(vec!["s0".to_string()], 10.0, 0.0, vec![0.0]);
    clock.advance(GESTURE_WINDOW_MS);
    doc.move_entities(vec!["s0".to_string()], 10.0, 0.0, vec![0.0]);
    assert_eq!(
        doc.undo_depth(),
        2,
        "two edits ≥ {GESTURE_WINDOW_MS} ms apart must not merge"
    );
}

#[test]
fn explicit_group_wins_over_the_window() {
    let clock = ManualClock::new(1_000);
    let mut doc = slot_doc(clock.clone());
    doc.begin_group();
    doc.move_entities(vec!["s0".to_string()], 10.0, 0.0, vec![0.0]);
    clock.advance(GESTURE_WINDOW_MS + 50);
    doc.move_entities(vec!["s0".to_string()], 10.0, 0.0, vec![0.0]);
    doc.end_group();
    assert_eq!(
        doc.undo_depth(),
        1,
        "begin_group/end_group must merge even across the window"
    );
    assert!(doc.undo());
    assert!((x_of(&doc) - 100.0).abs() < f32::EPSILON);
}

#[test]
fn end_group_splits_from_the_next_gesture() {
    let clock = ManualClock::new(1_000);
    let mut doc = slot_doc(clock.clone());
    doc.begin_group();
    doc.move_entities(vec!["s0".to_string()], 10.0, 0.0, vec![0.0]);
    doc.end_group();
    doc.move_entities(vec!["s0".to_string()], 10.0, 0.0, vec![0.0]);
    assert_eq!(
        doc.undo_depth(),
        2,
        "the op after end_group must be a new stack item"
    );
}

#[test]
fn depth_cap_drops_oldest_of_201_groups() {
    let mut doc = MissionDocCore::new();
    doc.set_origin_init(true);
    doc.add_slot(
        "s0", "sq", "lyr", 0, "Rifleman", None, None, 100.0, 200.0, 0.0, 0.0,
    );
    doc.set_origin_init(false);
    for _ in 0..201 {
        doc.move_entities(vec!["s0".to_string()], 10.0, 0.0, vec![0.0]);
    }
    assert_eq!(
        doc.undo_depth(),
        MAX_UNDO_GROUPS,
        "201 groups must leave {MAX_UNDO_GROUPS} undoable"
    );
    for _ in 0..MAX_UNDO_GROUPS {
        assert!(doc.undo());
    }
    assert!(!doc.can_undo(), "earliest group is gone — not a 201st undo");
    assert!(
        (x_of(&doc) - 110.0).abs() < f32::EPSILON,
        "first move remains applied after 200 undos; got {}",
        x_of(&doc)
    );
}

#[test]
fn hidden_prefix_math_drops_oldest_whole_group() {
    assert_eq!(hidden_prefix_after(200, 0), 0);
    assert_eq!(hidden_prefix_after(201, 0), 1);
    assert_eq!(hidden_prefix_after(205, 1), 5);
    assert_eq!(hidden_prefix_after(199, 5), 5);
}
