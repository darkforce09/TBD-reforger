//! Role: which ids the server reconciliation runs for, and which it skips.
//! Position: `editing/persist/tests` in the map engine.
//! Signals & state: explicit strings built in the test body.
//! Invariants: exactly the canonical 36-byte hexadecimal form passes. A local-only name must not,
//! because passing would send the editor down a network path whose only answer is a 404.

use super::*;

#[test]
fn the_canonical_form_is_a_server_id() {
    assert!(is_uuid("3f2504e0-4f89-11d3-9a0c-0305e82c3301"));
    assert!(is_uuid("FFFFFFFF-FFFF-FFFF-FFFF-FFFFFFFFFFFF"));
    assert!(is_uuid("00000000-0000-0000-0000-000000000000"));
}

#[test]
fn the_local_only_names_the_editor_opens_are_not_server_ids() {
    for local in ["smoke", "draft", "", "new-mission"] {
        assert!(!is_uuid(local), "{local:?} must stay local");
    }
}

#[test]
fn length_and_hyphen_positions_are_both_load_bearing() {
    // One byte short, one byte long, and the right length with a hyphen in the wrong place.
    assert!(!is_uuid("3f2504e0-4f89-11d3-9a0c-0305e82c330"));
    assert!(!is_uuid("3f2504e0-4f89-11d3-9a0c-0305e82c33011"));
    assert!(!is_uuid("3f2504e0-4f89-11d3-9a0c0-305e82c3301"));
    assert!(!is_uuid("3f2504e04f89-11d3-9a0c-0305e82c33011"));
}

#[test]
fn a_non_hexadecimal_byte_anywhere_disqualifies_the_id() {
    assert!(!is_uuid("3f2504e0-4f89-11d3-9a0c-0305e82c330g"));
    assert!(!is_uuid("zf2504e0-4f89-11d3-9a0c-0305e82c3301"));
    assert!(!is_uuid("3f2504e0-4f89-11d3-9a0c-0305e82c330 "));
}

/// A multi-byte character must not be sliced apart or counted as one byte of the shape.
#[test]
fn a_multi_byte_character_is_not_a_hexadecimal_digit() {
    assert!(!is_uuid("3f2504e0-4f89-11d3-9a0c-0305e82c33é"));
}
