//! **Role:** Domain regression cases.
//! **Position:** `mission_model::slot_line::tests::cases_1` in the `mission_model` crate.
//! **Signals & state:** explicit data inputs; no UI or graphics state.
//! **Invariants:** preserve authored order, numeric precision, and wire representations.

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
