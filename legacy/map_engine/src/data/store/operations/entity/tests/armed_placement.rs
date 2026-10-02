//! Role: what an armed palette pick-up commits on a canvas release.
//! Position: `doc/operations/entity/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;
use crate::data::store::operations::cargo::{read_loadout, set_cargo_defaults};
use crate::data::store::operations::cargo_rules::CargoRow;
use crate::data::store::operations::entity::vehicle_rows;
use crate::data::store::operations::projections::slot_rows;
use std::collections::HashMap;

fn ensure_default_layer(core: &MissionDocCore) -> String {
    core.add_editor_layer("layer-a", "Alpha", None);
    "layer-a".to_string()
}

fn rifleman_payload() -> PlacePayload {
    PlacePayload {
        asset_id: "char.us.rifleman".to_string(),
        role: "Rifleman".to_string(),
    }
}

fn truck_payload() -> PlacePayload {
    PlacePayload {
        asset_id: "veh.us.truck".to_string(),
        role: "Truck".to_string(),
    }
}

fn commit(
    core: &MissionDocCore,
    armed: ArmedPlacement,
    side: &str,
    alt_empty: bool,
) -> Option<PlacementCommit> {
    commit_armed_placement(
        core,
        ArmedPlacementRequest {
            armed,
            side,
            x: 100.0,
            y: 200.0,
            crew_toggle: true,
            alt_empty,
        },
        &Cell::new(0),
        ensure_default_layer,
    )
}

/// The authored `crewed` flag of the one placed vehicle. Absent means crewed — the document only
/// records the flag when the crew is left off.
fn authored_crewed_flag(core: &MissionDocCore) -> Option<bool> {
    let id = vehicle_rows(core).first()?.id.clone();
    let maps: serde_json::Value = serde_json::from_str(&core.small_maps_json()).ok()?;
    maps.get("vehiclesById")?
        .get(&id)?
        .get("crewed")
        .and_then(serde_json::Value::as_bool)
}

#[test]
fn the_crew_rule_lets_the_modifier_empty_a_vehicle_but_never_fill_one() {
    assert!(vehicle_places_its_crew(true, false));
    assert!(!vehicle_places_its_crew(true, true));
    assert!(!vehicle_places_its_crew(false, false));
    assert!(!vehicle_places_its_crew(false, true));
}

#[test]
fn a_draw_in_flight_commits_nothing_on_a_release() {
    let core = MissionDocCore::new();
    assert_eq!(
        commit(&core, ArmedPlacement::ZoneDraw, "BLUFOR", false),
        None
    );
}

#[test]
fn a_placed_character_is_filed_selected_and_seeded_from_its_asset_defaults() {
    set_cargo_defaults(HashMap::from([(
        "char.us.rifleman".to_string(),
        vec![CargoRow {
            container: "backpack".to_string(),
            item: "bandage".to_string(),
            qty: 2,
        }],
    )]));
    let core = MissionDocCore::new();
    let placed = commit(
        &core,
        ArmedPlacement::Character(rifleman_payload()),
        "BLUFOR",
        false,
    )
    .expect("the side is a real one");

    let selected = placed.selection.expect("a placed character is selected");
    assert_eq!(selected.len(), 1);
    assert!(!placed.placed_vehicle);
    assert!(placed.stamped_composition.is_none());

    let rows = slot_rows(&core);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, selected[0]);
    assert!(
        read_loadout(&core, &selected[0]).is_some_and(|lo| lo.contains("bandage")),
        "the place seeds the cargo the asset is authored to carry"
    );
}

#[test]
fn a_character_the_document_refuses_places_nothing() {
    let core = MissionDocCore::new();
    assert_eq!(
        commit(
            &core,
            ArmedPlacement::Character(rifleman_payload()),
            "CIVILIAN",
            false
        ),
        None
    );
    assert!(slot_rows(&core).is_empty());
}

#[test]
fn a_placed_vehicle_flags_the_lane_rebind_and_leaves_the_slot_selection_alone() {
    let core = MissionDocCore::new();
    let placed = commit(
        &core,
        ArmedPlacement::Vehicle(truck_payload()),
        "BLUFOR",
        false,
    )
    .expect("the side is a real one");

    assert!(placed.placed_vehicle);
    assert_eq!(placed.selection, None);
    assert_eq!(vehicle_rows(&core).len(), 1);
}

#[test]
fn a_vehicle_placed_with_the_modifier_held_is_stamped_without_its_crew() {
    let crewed = MissionDocCore::new();
    commit(
        &crewed,
        ArmedPlacement::Vehicle(truck_payload()),
        "BLUFOR",
        false,
    )
    .expect("the side is a real one");
    assert_eq!(authored_crewed_flag(&crewed), None, "crewed is the default");

    let empty = MissionDocCore::new();
    commit(
        &empty,
        ArmedPlacement::Vehicle(truck_payload()),
        "BLUFOR",
        true,
    )
    .expect("the side is a real one");
    assert_eq!(authored_crewed_flag(&empty), Some(false));
}

#[test]
fn a_vehicle_the_document_refuses_places_nothing() {
    let core = MissionDocCore::new();
    assert_eq!(
        commit(
            &core,
            ArmedPlacement::Vehicle(truck_payload()),
            "CIVILIAN",
            false
        ),
        None
    );
    assert!(vehicle_rows(&core).is_empty());
}

#[test]
fn a_placed_marker_writes_the_briefing_row_and_selects_nothing() {
    let core = MissionDocCore::new();
    let placed = commit(
        &core,
        ArmedPlacement::Marker("objective".to_string()),
        "BLUFOR",
        false,
    )
    .expect("a marker is placed for whichever side is active");

    assert_eq!(placed.selection, None);
    assert!(!placed.placed_vehicle);
    assert_eq!(marker_rows_of(&core).len(), 1);
}

#[test]
fn a_composition_that_carries_no_entities_stamps_nothing() {
    let core = MissionDocCore::new();
    assert_eq!(
        commit(
            &core,
            ArmedPlacement::Composition("comp-nothing".to_string()),
            "BLUFOR",
            false
        ),
        None
    );
}

#[test]
fn an_object_with_no_asset_behind_it_is_refused() {
    let core = MissionDocCore::new();
    assert_eq!(
        commit(
            &core,
            ArmedPlacement::Object(PlacePayload {
                asset_id: String::new(),
                role: "Barrier".to_string(),
            }),
            "BLUFOR",
            false
        ),
        None
    );
}
