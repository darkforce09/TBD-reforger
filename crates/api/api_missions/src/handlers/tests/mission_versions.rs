//! Save-boundary test for the version handlers: the cargo-capacity refusal the catalogued
//! validator performs.

use super::*;
use mission_wire_safety::CargoPhys;

/// Save refuses over-capacity cargo when the caller supplies phys attrs; an empty catalog stays
/// silent (never invent a limit). The numbers mirror the editor's own check: 4×60 cm³ into a
/// 200 cm³ vest.
#[test]
fn over_capacity_cargo_is_refused_at_save_with_catalog() {
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
    let bad = r#"{"schemaVersion":1,"editor":{"factions":[],"squads":[],"editorLayers":[],
            "slots":[{"id":"s1","role":"RFL","loadout":{"version":2,
              "wear":{"vest":"vest_rn"},"weapons":[],
              "cargo":[{"container":"vest","item":"mag","qty":4}]}}]}}"#;
    let err = validate_payload_with_catalog(bad, &catalog).expect_err("must refuse");
    assert_eq!(err.status, StatusCode::BAD_REQUEST);
    let details = err
        .details
        .as_ref()
        .and_then(|d| d.as_array())
        .expect("details");
    assert!(
        details.iter().any(|d| {
            d.as_str()
                .is_some_and(|s| s.contains("240 / 200 cm³") && s.contains("Plate Carrier"))
        }),
        "Save details must name the over-capacity finding: {details:?}"
    );

    let ok = r#"{"schemaVersion":1,"editor":{"factions":[],"squads":[],"editorLayers":[],
            "slots":[{"id":"s1","role":"RFL","loadout":{"version":2,
              "wear":{"vest":"vest_rn"},"weapons":[],
              "cargo":[{"container":"vest","item":"mag","qty":3}]}}]}}"#;
    assert!(
        validate_payload_with_catalog(ok, &catalog).is_ok(),
        "under-capacity must save"
    );
    assert!(
        validate_payload_with_catalog(bad, &CargoPhysCatalog::new()).is_ok(),
        "empty catalog must not invent a limit"
    );
}
