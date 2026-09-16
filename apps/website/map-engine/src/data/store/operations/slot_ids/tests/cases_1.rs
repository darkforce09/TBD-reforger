//! Role: Domain regression cases.
//! Position: `doc/operations/slot_ids/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn test_duplicate_slot_ids() {
    let doc = MissionDocCore::new();
    doc.add_squad("sq1", "f1", "Alpha", Some("1-1".to_string()));
    doc.add_slot("s1", "sq1", "l1", 0, "RFL", None, None, 0.0, 0.0, 0.0, 0.0);
    doc.add_slot("s1", "sq1", "l1", 1, "MED", None, None, 0.0, 0.0, 0.0, 0.0);

    assert!(duplicate_slot_ids(&doc).is_empty());
    doc.hydrate(
        &serde_json::json!({"editor": {
            "squads": [{"id":"sq1", "factionId":"f1", "name":"Alpha", "callsign":"1-1", "slotIds":["s1","s1"]}],
            "slots": [{"id":"s1", "squadId":"sq1", "role":"MED"}]
        }}).to_string(),
        "l1",
    );

    let dups = duplicate_slot_ids(&doc);
    assert!(!dups.is_empty());
    assert_eq!(dups[0], ("1-1".to_string(), "s1".to_string()));
}
