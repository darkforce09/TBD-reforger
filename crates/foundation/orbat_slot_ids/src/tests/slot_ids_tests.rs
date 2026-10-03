//! Pins the wire form of [`crate::SlotUid`] and [`crate::SlotId`]: each is a bare JSON string, so
//! the editor's slot rows, the compiled document and the squad links read and write the same bytes
//! as the plain strings they replaced.

use super::{SlotId, SlotUid};

#[test]
fn slot_id_serialises_as_the_bare_string_it_wraps() {
    let id = SlotUid::new("alpha-1-lead");
    let json = serde_json::to_string(&id).expect("slot uid serialises");
    assert_eq!(json, "\"alpha-1-lead\"");
}

#[test]
fn slot_id_reads_back_from_the_bare_string() {
    let id: SlotUid = serde_json::from_str("\"alpha-1-lead\"").expect("slot uid deserialises");
    assert_eq!(id.as_str(), "alpha-1-lead");
}

#[test]
fn derived_wire_slot_id_round_trips_as_the_bare_string() {
    let id = SlotId::new("blufor:Alpha 1:Squad Leader:0");
    let json = serde_json::to_string(&id).expect("wire slot id serialises");
    assert_eq!(json, "\"blufor:Alpha 1:Squad Leader:0\"");
    let back: SlotId = serde_json::from_str(&json).expect("wire slot id deserialises");
    assert_eq!(back, id);
}
