use super::*;
use website_map_engine::data::scenario::wire_safety::CargoPhys;

#[test]
fn editor_schema_compiles_and_accepts_minimal_payload() {
    // A minimal valid editor payload (schemaVersion int + editor block).
    let ok =
        br#"{"schemaVersion":1,"editor":{"factions":[],"squads":[],"slots":[],"editorLayers":[]}}"#;
    let details = validate_mission_editor_payload(ok).expect("compiles");
    assert!(details.is_empty(), "expected valid, got {details:?}");
}

#[test]
fn invalid_json_reports_detail_not_error() {
    let details = validate_mission_editor_payload(b"not json").expect("compiles");
    assert_eq!(details, vec!["payload is not valid JSON".to_string()]);
}

/// The save-time catch for control characters, on the channel `create_version` already answers
/// 400 with. Without it the same payload validates CLEAN here and fails later at `/compiled`,
/// as a 500 the author never sees.
#[test]
fn control_character_in_an_authored_slot_string_is_a_save_time_finding() {
    let bad = br#"{"schemaVersion":1,"editor":{"factions":[],
            "squads":[{"id":"sq1","callsign":"AL\tPHA","slotIds":["s1"]}],
            "slots":[{"id":"s1","role":"SL"}],"editorLayers":[]}}"#;
    let details = validate_mission_editor_payload(bad).expect("compiles");
    assert_eq!(details.len(), 1, "{details:?}");
    assert!(
        details[0].starts_with("/editor/squads/0/callsign:"),
        "{details:?}"
    );
    assert!(details[0].contains("TAB (U+0009)"), "{details:?}");

    // The schema half of the same call is untouched: a payload that is merely wire-safe still
    // has to satisfy `mission-editor-payload.schema.json`.
    let ok = br#"{"schemaVersion":1,"editor":{"factions":[],
            "squads":[{"id":"sq1","callsign":"ALPHA","slotIds":["s1"]}],
            "slots":[{"id":"s1","role":"SL"}],"editorLayers":[]}}"#;
    assert!(
        validate_mission_editor_payload(ok)
            .expect("compiles")
            .is_empty()
    );
}

/// Over-capacity cargo joins the same `details` channel as wire-safety, when the caller supplies
/// a phys catalog. Without a catalog the walk is silent (never invent).
#[test]
fn over_capacity_cargo_is_a_save_time_finding_with_catalog() {
    let mut catalog = CargoPhysCatalog::new();
    catalog.insert(
        "mag".into(),
        CargoPhys {
            display_name: "Mag".into(),
            weight_kg: Some(0.5),
            volume_cm3: Some(60.0),
            ..CargoPhys::default()
        },
    );
    catalog.insert(
        "vest_rn".into(),
        CargoPhys {
            display_name: "Plate Carrier".into(),
            max_weight_kg: Some(5.0),
            max_volume_cm3: Some(200.0),
            ..CargoPhys::default()
        },
    );
    // 4 × 60 = 240 > 200 cm³.
    let bad = br#"{"schemaVersion":1,"editor":{"factions":[],"squads":[],"editorLayers":[],
            "slots":[{"id":"s1","role":"RFL","loadout":{"version":2,
              "wear":{"vest":"vest_rn"},"weapons":[],
              "cargo":[{"container":"vest","item":"mag","qty":4}]}}]}}"#;
    let details = validate_mission_editor_payload_with_catalog(bad, &catalog).expect("compiles");
    assert_eq!(details.len(), 1, "{details:?}");
    assert!(
        details[0].starts_with("/editor/slots/0/loadout/wear/vest:"),
        "{details:?}"
    );
    assert!(details[0].contains("240 / 200 cm³"), "{details:?}");

    // One magazine fewer: under capacity → clean.
    let ok = br#"{"schemaVersion":1,"editor":{"factions":[],"squads":[],"editorLayers":[],
            "slots":[{"id":"s1","role":"RFL","loadout":{"version":2,
              "wear":{"vest":"vest_rn"},"weapons":[],
              "cargo":[{"container":"vest","item":"mag","qty":3}]}}]}}"#;
    assert!(
        validate_mission_editor_payload_with_catalog(ok, &catalog)
            .expect("compiles")
            .is_empty(),
        "under-capacity must stay clean"
    );

    // Same over-capacity bytes, empty catalog → silent (signature-compatible entry point).
    assert!(
        validate_mission_editor_payload(bad)
            .expect("compiles")
            .is_empty(),
        "empty catalog must not invent a limit"
    );
}

#[test]
fn mission_document_schema_compiles() {
    // Empty object violates the required keys → non-empty details, but compiles.
    let details = validate_mission_document(b"{}").expect("compiles");
    assert!(!details.is_empty(), "empty doc should be schema-invalid");
}

/// A document that is schema-shaped can still be refused solely on raw byte size. Pads
/// `meta.author` (no maxLength) past `MISSION_FILE_MAX_BYTES`; the size finding must fire before
/// (or instead of) any schema finding, and must name the mod constant.
#[test]
fn oversized_mission_document_is_rejected_on_byte_ceiling() {
    let mut doc: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../../../packages/tbd-schema/golden-missions/last-stand-at-montfort.json"
    ))
    .expect("golden");
    // Keep the document structurally valid so a missing size check would pass schema alone.
    let pad = "x".repeat(MISSION_FILE_MAX_BYTES);
    doc["meta"]["author"] = serde_json::Value::String(pad);
    let raw = serde_json::to_vec(&doc).expect("serialize");
    assert!(
        raw.len() > MISSION_FILE_MAX_BYTES,
        "pad must push past the ceiling (got {} B)",
        raw.len()
    );

    let details = validate_mission_document(&raw).expect("compiles");
    assert_eq!(details.len(), 1, "{details:?}");
    assert!(
        details[0].starts_with("/: document exceeds MISSION_FILE_MAX_BYTES"),
        "{details:?}"
    );
    assert!(
        details[0].contains(&MISSION_FILE_MAX_BYTES.to_string()),
        "{details:?}"
    );
}

#[test]
fn schema_x_tbd_mission_file_max_bytes_matches_mod_constant() {
    let schema: serde_json::Value =
        serde_json::from_str(MISSION_SCHEMA).expect("mission.schema.json");
    let pinned = schema["x-tbd-missionFileMaxBytes"]
        .as_u64()
        .expect("x-tbd-missionFileMaxBytes must be present on mission.schema.json");
    assert_eq!(
        pinned as usize, MISSION_FILE_MAX_BYTES,
        "schema keyword drifted from validate_mission_document / TBD_MissionLoader"
    );
}
