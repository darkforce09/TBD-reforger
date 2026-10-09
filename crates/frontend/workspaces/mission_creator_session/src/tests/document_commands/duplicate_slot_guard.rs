//! The duplicate-slot refusal: every duplicate pair gets its own line naming both the squad
//! callsign and the slot id.

use mission_editing_commands::document_text::merge_report::duplicate_slot_id_report;

/// The refusal names the callsign and the id: "This mission has duplicate slot ids" is not
/// actionable; "squad 1-1 lists slot s1 twice" is.
#[test]
fn the_refusal_names_the_callsign_and_the_id() {
    let (head, rows) = duplicate_slot_id_report(&[
        ("1-1".to_string(), "s1".to_string()),
        ("2-4".to_string(), "s9".to_string()),
    ]);
    assert!(
        head.contains("refused") && head.contains('2'),
        "the headline must say the save was refused and how many problems there are: {head}"
    );
    assert_eq!(rows.len(), 2, "one line per duplicate");
    assert!(
        rows[0].contains("1-1") && rows[0].contains("s1"),
        "a line naming only one of the two is not actionable: {}",
        rows[0]
    );
    assert!(
        rows[1].contains("2-4") && rows[1].contains("s9"),
        "every pair gets its own line: {}",
        rows[1]
    );
}
