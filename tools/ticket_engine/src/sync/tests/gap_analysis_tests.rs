use super::*;

use serde_json::json;
use std::collections::BTreeMap;

/// Pins with no gap implementations, so a lookup that misses the checkmark falls through to
/// the `—` placeholder.
fn no_pins() -> CorpusPins {
    CorpusPins {
        game_mod_programme_ticket: String::new(),
        never_minted: vec![],
        gap_implementations: BTreeMap::new(),
    }
}

/// The ticket-column value the gap-row notes produce against an empty registry.
fn ticket_for_notes(notes: &str) -> String {
    lookup_ticket_for_gap(
        &no_pins(),
        &json!({ "tickets": [] }),
        "EDEN-ROW-001",
        "TBD-ROW-001",
        notes,
    )
}

#[test]
fn a_checkmark_captures_every_digit_of_a_four_digit_id() {
    assert_eq!(ticket_for_notes("✅ T-1000"), "T-1000");
    assert_eq!(ticket_for_notes("✅T-1000 shipped with the wave"), "T-1000");
    assert_eq!(ticket_for_notes("✅ T-12345"), "T-12345");
}

#[test]
fn a_checkmark_still_captures_a_three_digit_id() {
    assert_eq!(ticket_for_notes("✅ T-649"), "T-649");
    assert_eq!(ticket_for_notes("✅ T-649 inverted the guard"), "T-649");
}

#[test]
fn a_checkmark_captures_the_parent_of_a_dotted_child() {
    assert_eq!(ticket_for_notes("✅ T-649.2 follow-on"), "T-649");
    assert_eq!(ticket_for_notes("✅ T-1000.3"), "T-1000");
}

#[test]
fn notes_without_a_leading_checkmark_id_fall_through() {
    assert_eq!(ticket_for_notes("✅ T-12"), "—");
    assert_eq!(ticket_for_notes("T-649 ✅"), "—");
}
