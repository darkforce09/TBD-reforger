//! The row list after a write: placement of a saved row, removal of a deleted one, and the row the
//! dossier shows, and the faction grouping of the index.

use super::{place_saved_vehicle, remove_vehicle, selection_after_removal, shown_vehicle};
use frontend_api_dtos::vehicles::Vehicle;

fn row(id: &str, name: &str) -> Vehicle {
    Vehicle {
        id: id.into(),
        name: name.into(),
        faction: "USSR".into(),
        armor_type: "APC".into(),
        amphibious: String::new(),
        primary_threat: String::new(),
        profile_image_url: String::new(),
    }
}

fn names(rows: &[Vehicle]) -> Vec<&str> {
    rows.iter().map(|row| row.name.as_str()).collect()
}

#[test]
fn a_new_row_takes_its_place_by_name() {
    let mut rows = vec![row("1", "BTR-70"), row("3", "UAZ-469")];
    place_saved_vehicle(&mut rows, row("2", "M113A3"));
    assert_eq!(names(&rows), vec!["BTR-70", "M113A3", "UAZ-469"]);
    place_saved_vehicle(&mut rows, row("4", "ZIL-131"));
    assert_eq!(names(&rows), vec!["BTR-70", "M113A3", "UAZ-469", "ZIL-131"]);
    place_saved_vehicle(&mut rows, row("5", "AAV-7"));
    assert_eq!(
        names(&rows),
        vec!["AAV-7", "BTR-70", "M113A3", "UAZ-469", "ZIL-131"]
    );
}

#[test]
fn a_saved_row_replaces_its_stored_copy_and_moves_by_its_new_name() {
    let mut rows = vec![row("1", "BTR-70"), row("2", "M113A3"), row("3", "UAZ-469")];
    place_saved_vehicle(&mut rows, row("1", "ZIL-131"));
    assert_eq!(names(&rows), vec!["M113A3", "UAZ-469", "ZIL-131"]);
    assert_eq!(rows.iter().filter(|r| r.id == "1").count(), 1);
}

#[test]
fn an_unchanged_name_keeps_the_rows_place() {
    let mut rows = vec![row("1", "BTR-70"), row("2", "M113A3"), row("3", "UAZ-469")];
    let mut edited = row("2", "M113A3");
    edited.primary_threat = "RPG".into();
    place_saved_vehicle(&mut rows, edited);
    assert_eq!(names(&rows), vec!["BTR-70", "M113A3", "UAZ-469"]);
    assert_eq!(rows[1].primary_threat, "RPG");
}

#[test]
fn equal_names_are_ordered_by_id() {
    let mut rows = vec![row("a", "BTR-70"), row("c", "BTR-70")];
    place_saved_vehicle(&mut rows, row("b", "BTR-70"));
    let ids: Vec<&str> = rows.iter().map(|r| r.id.as_str()).collect();
    assert_eq!(ids, vec!["a", "b", "c"]);
}

#[test]
fn a_deleted_row_leaves_and_the_selection_falls_back_to_the_first() {
    let mut rows = vec![row("1", "BTR-70"), row("2", "M113A3"), row("3", "UAZ-469")];
    remove_vehicle(&mut rows, "2");
    assert_eq!(names(&rows), vec!["BTR-70", "UAZ-469"]);
    assert_eq!(selection_after_removal(&rows, "2"), "1");
    assert_eq!(selection_after_removal(&rows, "3"), "3");
    remove_vehicle(&mut rows, "1");
    remove_vehicle(&mut rows, "3");
    assert_eq!(selection_after_removal(&rows, "3"), "");
}

#[test]
fn the_dossier_shows_the_selected_row_or_the_first() {
    let rows = vec![row("1", "BTR-70"), row("2", "M113A3")];
    assert_eq!(shown_vehicle(&rows, "2").map(|r| r.id.as_str()), Some("2"));
    assert_eq!(
        shown_vehicle(&rows, "gone").map(|r| r.id.as_str()),
        Some("1")
    );
    assert_eq!(shown_vehicle(&[], "1"), None);
}

/// The index groups the recorded list by faction in first-seen order, and a row without a faction
/// forms no group.
#[test]
fn the_index_groups_factions_in_first_seen_order() {
    use crate::vehicles::vehicle_grid::faction_order;
    use frontend_api_dtos::DataEnvelope;
    let list: DataEnvelope<Vehicle> = serde_json::from_str(
        frontend_test_support::fixtures::golden!("GET__vehicle-database.json"),
    )
    .expect("the recorded list decodes into Vehicle rows");
    assert_eq!(
        faction_order(&list.data),
        vec![
            "USSR".to_string(),
            "US Army".to_string(),
            "Civilian".to_string()
        ]
    );
    let mut unassigned = row("unknown", "Unknown");
    unassigned.faction = String::new();
    assert_eq!(
        faction_order(&[unassigned, row("b", "BTR-70")]),
        vec!["USSR".to_string()]
    );
}
