//! Role: Domain regression cases.
//! Position: `slot_line/tests` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn format_slot_line_primary_and_launcher() {
    let s = format_slot_line(
        1,
        "Squad Leader",
        None,
        Some("L85A3"),
        Some("GL"),
        None,
        false,
    );
    assert_eq!(s, "1: Squad Leader (L85A3 + GL)");
}

#[test]
fn format_slot_line_tag_med() {
    let s = format_slot_line(2, "Medic", None, Some("L85A3"), None, Some("MED"), false);
    assert_eq!(s, "2: Medic (L85A3) | MED");
}

#[test]
fn format_slot_line_is_leader() {
    let s = format_slot_line(
        1,
        "Squad Leader",
        Some("L85A3 · GL"),
        None,
        None,
        Some("MED"),
        true,
    );
    assert_eq!(s, "1: Squad Leader (L85A3 + GL) | MED | SL");
}

#[test]
fn format_slot_line_summary_dot_split() {
    let s = format_slot_line(
        3,
        "Rifleman (AT)",
        Some("L85A3 · NLAW"),
        None,
        None,
        None,
        false,
    );
    assert_eq!(s, "3: Rifleman (AT) (L85A3 + NLAW)");
}

#[test]
fn format_slot_line_no_weapons() {
    let s = format_slot_line(1, "Rifleman", None, None, None, None, false);
    assert_eq!(s, "1: Rifleman");
}
