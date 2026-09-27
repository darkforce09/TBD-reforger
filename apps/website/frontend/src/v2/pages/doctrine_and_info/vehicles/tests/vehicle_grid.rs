//! The vehicle index's grouping helper, over hand-built rows and the recorded list.

use super::faction_order;
use crate::v2::core::api::dto::vehicles::Vehicle;
use crate::v2::core::api::dto::DataEnvelope;

/// The recorded `GET /vehicle-database` answer the DOM oracle serves.
const RECORDED_LIST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/api/GET__vehicle-database.json"
));

fn row(name: &str, faction: &str) -> Vehicle {
    Vehicle {
        id: format!("id-{name}"),
        name: name.into(),
        faction: faction.into(),
        armor_type: "APC".into(),
        amphibious: String::new(),
        primary_threat: String::new(),
        profile_image_url: String::new(),
    }
}

#[test]
fn factions_preserve_first_seen_order() {
    let rows = vec![
        row("BTR-70", "USSR"),
        row("M113A3", "US Army"),
        row("UAZ-469", "USSR"),
    ];
    assert_eq!(
        faction_order(&rows),
        vec!["USSR".to_string(), "US Army".to_string()]
    );
}

#[test]
fn a_row_without_a_faction_forms_no_group() {
    let rows = vec![row("Unknown", ""), row("BTR-70", "USSR")];
    assert_eq!(faction_order(&rows), vec!["USSR".to_string()]);
}

#[test]
fn the_recorded_list_decodes_into_typed_rows_and_groups_in_first_seen_order() {
    let list: DataEnvelope<Vehicle> =
        serde_json::from_str(RECORDED_LIST).expect("the recorded list decodes into Vehicle rows");
    assert_eq!(list.data.len(), 6);
    assert_eq!(
        faction_order(&list.data),
        vec![
            "USSR".to_string(),
            "US Army".to_string(),
            "Civilian".to_string()
        ]
    );
    let sedan = list
        .data
        .iter()
        .find(|v| v.name == "S105 Sedan")
        .expect("the recorded list holds the sedan");
    assert!(
        sedan.amphibious.is_empty()
            && sedan.primary_threat.is_empty()
            && sedan.profile_image_url.is_empty(),
        "absent optional fields decode as empty"
    );
}
