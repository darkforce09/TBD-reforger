//! Role: Domain regression cases.
//! Position: `mission/validation/wire_safety/tests` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn byte_scan_equals_char_scan_over_the_schema_pattern() {
    for c in ['\u{0}', '\u{1f}', '\u{7f}', '\t', '\n', '\r'] {
        assert!(
            first_unsafe_byte(&c.to_string()).is_some(),
            "{c:?} must be caught"
        );
    }
    for s in ["ALPHA", "Ålpha — Brávo 中文 🎯", " ", "", "a.b~c:d"] {
        assert_eq!(first_unsafe_byte(s), None, "{s:?} must be wire-safe");
    }
}

#[test]
fn clean_payload_reports_nothing() {
    let p = json!({"editor": {
        "factions": [{"key": "blufor", "name": "US Army"}],
        "squads": [{"id": "sq1", "callsign": "Alpha", "slotIds": ["s1"]}],
        "slots": [{"id": "s1", "role": "SL"}],
    }});
    assert!(scan_editor_payload(&p).is_empty());
}

#[test]
fn tab_in_callsign_is_reported_with_location_and_cause() {
    let p = json!({"editor": {
        "squads": [{"id": "sq1", "callsign": "AL\tPHA"}],
    }});
    let d = scan_editor_payload(&p);
    assert_eq!(d.len(), 1, "{d:?}");
    assert!(d[0].starts_with("/editor/squads/0/callsign:"), "{d:?}");
    assert!(
        d[0].contains("\"AL\\tPHA\""),
        "value must be escaped: {d:?}"
    );
    assert!(d[0].contains("TAB (U+0009)"), "{d:?}");
    assert!(d[0].contains("slots[].groupCallsign"), "{d:?}");
}

#[test]
fn substituted_blanks_are_not_reported_but_the_rung_that_is_read_is() {
    let clean =
        json!({"editor": {"squads": [{"id": "sq\t1", "callsign": "Alpha", "name": "n\tm"}]}});
    assert!(scan_editor_payload(&clean).is_empty());

    let dirty = json!({"editor": {"squads": [{"id": "sq1", "callsign": "", "name": "n\tm"}]}});
    let d = scan_editor_payload(&dirty);
    assert_eq!(d.len(), 1, "{d:?}");
    assert!(d[0].starts_with("/editor/squads/0/name:"), "{d:?}");

    let blanks = json!({"editor": {"factions": [{"key": "blufor", "name": ""}], "slots": [{"id": "s1", "role": ""}]}});
    assert!(scan_editor_payload(&blanks).is_empty());
}

#[test]
fn identical_bad_values_collapse_to_one_row_with_a_count() {
    let slots: Vec<Value> = (0..2000)
        .map(|i| json!({"id": format!("s{i}"), "role": "SL\tX"}))
        .collect();
    let d = scan_editor_payload(&json!({"editor": {"slots": slots}}));
    assert_eq!(
        d.len(),
        1,
        "a bulk paste must not produce 2000 lines: {d:?}"
    );
    assert!(d[0].contains("and 1999 more with the same value"), "{d:?}");
}

#[test]
fn distinct_bad_values_are_capped_with_a_tail_line() {
    let slots: Vec<Value> = (0..MAX_REPORTED + 5)
        .map(|i| json!({"id": format!("s{i}"), "role": format!("SL\t{i}")}))
        .collect();
    let d = scan_editor_payload(&json!({"editor": {"slots": slots}}));
    assert_eq!(d.len(), MAX_REPORTED + 1);
    assert!(
        d[MAX_REPORTED].contains("5 further distinct value(s)"),
        "{d:?}"
    );
}

#[test]
fn slot_id_is_scanned_because_flatten_copies_it_verbatim_into_uid() {
    let p = json!({"editor": {"slots": [{"id": "s\u{7f}1", "role": "SL"}]}});
    let d = scan_editor_payload(&p);
    assert_eq!(d.len(), 1, "{d:?}");
    assert!(d[0].contains("DEL (U+007F)"), "{d:?}");
    assert!(d[0].contains("slots[].uid"), "{d:?}");
}

#[test]
fn missing_editor_block_is_not_an_error() {
    assert!(scan_editor_payload(&json!({})).is_empty());
    assert!(scan_editor_payload(&json!({"editor": 7})).is_empty());
}

#[test]
fn over_capacity_cargo_is_a_finding() {
    let cat = catalog_fixture();

    let p = slot_with_cargo(
        json!({"vest": "vest_rn", "backpack": "pack_rn"}),
        json!([
            {"container": "vest", "item": "mag", "qty": 4},
            {"container": "backpack", "item": "mag", "qty": 4}
        ]),
    );
    let d = scan_cargo_capacity(&p, &cat);
    assert_eq!(d.len(), 1, "{d:?}");
    assert!(
        d[0].starts_with("/editor/slots/0/loadout/wear/vest:"),
        "{d:?}"
    );
    assert!(d[0].contains("240 / 200 cm³"), "{d:?}");
    assert!(d[0].contains("Plate Carrier"), "{d:?}");
    assert!(
        !d[0].contains("kg"),
        "weight under limit must stay quiet: {d:?}"
    );
    assert!(d[0].contains(CARGO_CAPACITY_CAVEAT), "{d:?}");
}

#[test]
fn under_capacity_cargo_is_ok() {
    let cat = catalog_fixture();
    let p = slot_with_cargo(
        json!({"vest": "vest_rn"}),
        json!([{"container": "vest", "item": "mag", "qty": 3}]),
    );
    assert!(scan_cargo_capacity(&p, &cat).is_empty());
}

#[test]
fn empty_catalog_never_invents_a_limit() {
    let p = slot_with_cargo(
        json!({"vest": "vest_rn"}),
        json!([{"container": "vest", "item": "mag", "qty": 40}]),
    );
    assert!(scan_cargo_capacity(&p, &CargoPhysCatalog::new()).is_empty());
}

#[test]
fn no_garment_or_uncatalogued_capacity_is_silent() {
    let cat = catalog_fixture();
    let heavy = json!([{"container": "vest", "item": "mag", "qty": 40}]);

    assert!(scan_cargo_capacity(&slot_with_cargo(json!({}), heavy.clone()), &cat).is_empty());

    let mut cat2 = catalog_fixture();
    cat2.insert(
        "plain_rn".into(),
        CargoPhys {
            display_name: "Uncatalogued Vest".into(),
            ..CargoPhys::default()
        },
    );
    assert!(
        scan_cargo_capacity(&slot_with_cargo(json!({"vest": "plain_rn"}), heavy), &cat2).is_empty()
    );
}

#[test]
fn armored_vest_fault_keys_on_the_wear_row() {
    let mut cat = CargoPhysCatalog::new();
    cat.insert(
        "brick".into(),
        CargoPhys {
            display_name: "Brick".into(),
            weight_kg: Some(4.0),
            volume_cm3: Some(300.0),
            ..CargoPhys::default()
        },
    );
    cat.insert(
        "av_rn".into(),
        CargoPhys {
            display_name: "Armored Vest".into(),
            max_weight_kg: Some(5.0),
            max_volume_cm3: Some(200.0),
            ..CargoPhys::default()
        },
    );
    let p = slot_with_cargo(
        json!({"armoredVest": "av_rn"}),
        json!([{"container": "vest", "item": "brick", "qty": 2}]),
    );
    let d = scan_cargo_capacity(&p, &cat);
    assert_eq!(d.len(), 1, "{d:?}");
    assert!(
        d[0].starts_with("/editor/slots/0/loadout/wear/armoredVest:"),
        "{d:?}"
    );
    assert!(d[0].contains("8.0 / 5 kg"), "{d:?}");
    assert!(d[0].contains("600 / 200 cm³"), "{d:?}");
}
