use super::*;

const V1: &str =
    include_str!("../../../../../../../packages/tbd-schema/registry/loadout-export.sample.json");
const V2: &str =
    include_str!("../../../../../../../packages/tbd-schema/registry/loadout-export.v2.sample.json");

/// Value-level round-trip: parse → serialize → parse; the two JSON values must be EQUAL
/// (key order irrelevant; null-vs-absent must be preserved — the double-Option contract).
fn round_trips(fixture: &str) {
    let parsed: LoadoutExport = serde_json::from_str(fixture).expect("deserialize");
    let re = serde_json::to_string(&parsed).expect("serialize");
    let a: serde_json::Value = serde_json::from_str(fixture).unwrap();
    let b: serde_json::Value = serde_json::from_str(&re).unwrap();
    assert_eq!(a, b, "value round-trip drift");
}

#[test]
fn v1_sample_round_trips() {
    round_trips(V1);
    let LoadoutExport::V1(doc) = serde_json::from_str(V1).unwrap() else {
        panic!("v1 fixture parsed as wrong version");
    };
    assert!(doc.gear.primary.is_some() && doc.gear.helmet.is_none());
}

#[test]
fn v2_sample_round_trips() {
    round_trips(V2);
    let LoadoutExport::V2(doc) = serde_json::from_str(V2).unwrap() else {
        panic!("v2 fixture parsed as wrong version");
    };
    assert_eq!(doc.wear.len(), 8);
    assert!(doc.weapons.iter().any(|w| w.slot_index == 0));
    // The fixture's second weapon omits optic entirely — absent, not null.
    let grenade = doc.weapons.iter().find(|w| w.slot_index == 3).unwrap();
    assert!(grenade.optic.is_none() && grenade.attachments.is_none());
}
