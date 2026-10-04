//! Shape checks for the vehicle write bodies, which no captured fixture carries: the three states
//! of a patch field, and the full body a create or a replace sends.

use serde_json::json;

use super::*;

#[test]
fn an_empty_patch_sends_an_empty_object() {
    assert_eq!(
        serde_json::to_string(&VehiclePatch::default()).unwrap(),
        "{}"
    );
}

#[test]
fn a_patch_sends_null_to_clear_and_a_value_to_set_and_omits_the_rest() {
    let patch = VehiclePatch {
        name: Some("BTR-80".into()),
        amphibious: Some(None),
        profile_image_url: Some(Some("/uploads/btr80.png".into())),
        ..VehiclePatch::default()
    };
    assert_eq!(
        serde_json::to_value(&patch).unwrap(),
        json!({
            "name": "BTR-80",
            "amphibious": null,
            "profile_image_url": "/uploads/btr80.png"
        })
    );
}

#[test]
fn a_patch_reads_an_absent_key_a_null_and_a_value_as_three_states() {
    let patch: VehiclePatch =
        serde_json::from_str(r#"{"amphibious":null,"primary_threat":"ATGM"}"#).unwrap();
    assert_eq!(
        patch.profile_image_url, None,
        "absent leaves the field as stored"
    );
    assert_eq!(patch.amphibious, Some(None), "null clears the field");
    assert_eq!(
        patch.primary_threat,
        Some(Some("ATGM".to_string())),
        "a value sets the field"
    );
    assert_eq!(patch.name, None);
}

#[test]
fn every_patch_state_writes_back_as_it_was_read() {
    for body in [
        r#"{}"#,
        r#"{"amphibious":null}"#,
        r#"{"amphibious":"Yes"}"#,
        r#"{"name":"M113A3","faction":"US Army","armor_type":"Light Armour","amphibious":"No","primary_threat":null,"profile_image_url":""}"#,
    ] {
        let patch: VehiclePatch = serde_json::from_str(body).unwrap();
        assert_eq!(serde_json::to_string(&patch).unwrap(), body);
    }
}

#[test]
fn a_patch_refuses_a_key_the_contract_does_not_name() {
    assert!(serde_json::from_str::<VehiclePatch>(r#"{"deleted_at":null}"#).is_err());
}

#[test]
fn a_write_sends_every_field_and_empty_optionals_as_empty_strings() {
    let write = VehicleWrite {
        name: "S105 Sedan".into(),
        faction: "Civilian".into(),
        armor_type: "Unarmoured".into(),
        amphibious: String::new(),
        primary_threat: String::new(),
        profile_image_url: String::new(),
    };
    assert_eq!(
        serde_json::to_value(&write).unwrap(),
        json!({
            "name": "S105 Sedan",
            "faction": "Civilian",
            "armor_type": "Unarmoured",
            "amphibious": "",
            "primary_threat": "",
            "profile_image_url": ""
        })
    );
}
