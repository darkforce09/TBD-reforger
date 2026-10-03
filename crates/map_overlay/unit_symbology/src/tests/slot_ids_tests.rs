//! Pins the wire form of [`crate::slot_ids::SlotId`]: a bare JSON string, the same form the
//! mission model's slot id uses, so both read and write the same bytes.

use super::SlotId;

#[test]
fn slot_id_serialises_as_the_bare_string_it_wraps() {
    let id = SlotId::new("alpha-1-lead");
    let json = serde_json::to_string(&id).expect("slot id serialises");
    assert_eq!(json, "\"alpha-1-lead\"");
}

#[test]
fn slot_id_reads_back_from_the_bare_string() {
    let id: SlotId = serde_json::from_str("\"alpha-1-lead\"").expect("slot id deserialises");
    assert_eq!(id.as_str(), "alpha-1-lead");
}
