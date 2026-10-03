//! Round trips for the ballistics-catalog shapes: the public list, one catalog document decoded
//! by the map engine's type, and the administrator upload's report.

use super::*;
use crate::foundation::transport::dto::ballistics_catalogs::{
    BallisticsCatalog, BallisticsCatalogList, CatalogUploadReport, ShellRole,
};

fn summary(version: u32) -> Value {
    json!({
        "catalog_id": "vanilla-mortars",
        "catalog_version": version,
        "title": "Vanilla mortars",
        "game_build": "1.4.0.53",
        "export_generation_id": "5A3F0C9B1E2D4A7C",
        "catalog_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
        "uploaded_at": "2000-01-01T00:00:00Z"
    })
}

/// Every key of every listed version is a named field, and the versions keep their order.
#[test]
fn the_catalog_list_claims_every_key() {
    let wire = json!({"data": [summary(1), summary(2)]}).to_string();
    assert_golden::<BallisticsCatalogList>(&wire, &[]);
    let list: BallisticsCatalogList = serde_json::from_str(&wire).unwrap();
    let versions: Vec<u32> = list.data.iter().map(|row| row.catalog_version).collect();
    assert_eq!(versions, [1, 2]);
}

/// A summary carrying a key the contract does not define fails the read.
#[test]
fn a_summary_with_an_unknown_key_fails_the_read() {
    let mut row = summary(1);
    row["catalog_document"] = json!({});
    let wire = json!({ "data": [row] });
    assert!(serde_json::from_value::<BallisticsCatalogList>(wire).is_err());
}

/// A catalog document decodes through the map engine's type and writes back every key.
#[test]
fn a_catalog_document_round_trips_through_the_map_engine_type() {
    let weapon = json!({
        "weapon_id": "m252", "display_name": "M252", "prefab_guid": "5A3F0C9B1E2D4A7E",
        "caliber_mm": 81.0, "mils_per_circle": 6400, "elevation_min_deg": 45.0,
        "elevation_max_deg": 85.0, "muzzle_init_speed_coef": 1.0,
        "dispersion_diameter_m": 20.0, "dispersion_range_m": 1000.0, "shell_ids": ["m821-he"]
    });
    let shell = json!({
        "shell_id": "m821-he", "display_name": "M821 HE", "prefab_guid": "5A3F0C9B1E2D4A7F",
        "role": "he", "init_speed_m_s": 70.0, "init_speed_variation": 0.01, "mass_kg": 4.1,
        "air_drag": 0.0003, "side_air_drag_scale": 1.0, "wind_influence_multiplier": 1.0,
        "dispersion_multiplier": 1.0, "time_to_live_s": 60.0, "standard_dispersion_m": 30.0,
        "charges": [{"rings": 0, "init_speed_coef": 1.0, "is_default": true}]
    });
    let resource = json!({
        "guid": "5A3F0C9B1E2D4A7D", "resource_name": "Mortar_M252.et",
        "sha256": "1111111111111111111111111111111111111111111111111111111111111111"
    });
    let wire = json!({
        "schema_version": 1,
        "catalog_id": "vanilla-mortars",
        "catalog_version": 1,
        "title": "Vanilla mortars",
        "game_build": "1.4.0.53",
        "export_generation_id": "5A3F0C9B1E2D4A7C",
        "gravity_m_s2": 9.81,
        "gravity_source": "oracle",
        "resources": [resource],
        "weapons": [weapon],
        "shells": [shell]
    })
    .to_string();
    assert_golden::<BallisticsCatalog>(&wire, &[]);
    let catalog = BallisticsCatalog::from_json_slice(wire.as_bytes()).unwrap();
    assert_eq!(catalog.shells[0].role, ShellRole::He);
    assert!(catalog.shells[0].time_fuze.is_none());
}

/// Both upload outcomes: an accepted report with no failures, and a refused one naming each case.
#[test]
fn the_upload_report_claims_every_key_in_both_outcomes() {
    let accepted = json!({
        "accepted": true,
        "cases": 42,
        "failures": [],
        "forward_samples_not_judged": 7
    })
    .to_string();
    assert_golden::<CatalogUploadReport>(&accepted, &[]);

    let refused = json!({
        "accepted": false,
        "cases": 42,
        "failures": [{"case_id": "native/m821-he/rings-0/row-3",
                      "reason": "time of flight differs by 0.2 s"}],
        "forward_samples_not_judged": 7
    })
    .to_string();
    assert_golden::<CatalogUploadReport>(&refused, &[]);
    let report: CatalogUploadReport = serde_json::from_str(&refused).unwrap();
    assert!(!report.accepted);
    assert_eq!(report.failures.len(), 1);
    assert_eq!(report.forward_samples_not_judged, 7);
}

/// A report without its count of unjudged forward samples fails the read: the key is required.
#[test]
fn an_upload_report_without_its_unjudged_sample_count_fails_the_read() {
    let wire = json!({"accepted": true, "cases": 42, "failures": []});
    assert!(serde_json::from_value::<CatalogUploadReport>(wire).is_err());
}
