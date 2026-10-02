//! Role: the arm gate, read across both authoring modes.
//! Position: `doc/operations/entity/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn objects_mode_accepts_objects_and_refuses_what_is_placed_on_a_side() {
    assert!(placement_is_armable(ArmedPlacementKind::Object, true));
    assert!(!placement_is_armable(ArmedPlacementKind::Character, true));
    assert!(!placement_is_armable(ArmedPlacementKind::Vehicle, true));
}

#[test]
fn a_side_mode_accepts_characters_and_vehicles_and_refuses_objects() {
    assert!(placement_is_armable(ArmedPlacementKind::Character, false));
    assert!(placement_is_armable(ArmedPlacementKind::Vehicle, false));
    assert!(!placement_is_armable(ArmedPlacementKind::Object, false));
}

#[test]
fn compositions_and_markers_belong_to_neither_mode_and_a_zone_is_never_armed() {
    for objects_mode in [true, false] {
        assert!(placement_is_armable(
            ArmedPlacementKind::Composition,
            objects_mode
        ));
        assert!(placement_is_armable(
            ArmedPlacementKind::Marker,
            objects_mode
        ));
        assert!(!placement_is_armable(
            ArmedPlacementKind::Zone,
            objects_mode
        ));
    }
}
