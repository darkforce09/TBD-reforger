//! Role: exercise editor operations through the public headless boundary.
//! Position: mission-core integration tests.
//! Signals & state: deterministic documents and explicit host-policy callbacks.
//! Invariants: cancellation is inert; edits retain authored precision and undo behavior.

#![cfg(feature = "store")]

use serde_json::{Value, json};
use std::cell::Cell;
use website_map_engine::data::store::{MissionDocCore, operations};

fn document() -> MissionDocCore {
    let core = MissionDocCore::with_client_id(17);
    core.add_editor_layer("layer", "Default", None);
    core.add_slot(
        "slot", "squad", "layer", 0, "Rifleman", None, None, 100.0, 200.0, 37.3, 90.0,
    );
    core.add_vehicle(
        "vehicle",
        "car.et",
        Some(400.0),
        Some(500.0),
        Some(42.7),
        Some(33.0),
    );
    core
}

#[test]
fn refused_transform_leaves_document_and_history_unchanged() {
    let core = document();
    let slots: Value = serde_json::from_str(&core.slots_json()).unwrap();
    let other: Value = serde_json::from_str(&core.small_maps_json()).unwrap();
    let depth = core.undo_depth();
    let calls = Cell::new(0);
    assert!(!operations::transform::align_selection(
        &core,
        operations::placement::AlignEdge::Left,
        vec!["slot".into(), "vehicle".into()],
        |count, action| {
            assert_eq!((count, action), (2, "align"));
            calls.set(calls.get() + 1);
            false
        },
    ));
    assert_eq!(calls.get(), 1);
    assert_eq!(
        serde_json::from_str::<Value>(&core.slots_json()).unwrap(),
        slots
    );
    assert_eq!(
        serde_json::from_str::<Value>(&core.small_maps_json()).unwrap(),
        other
    );
    assert_eq!(core.undo_depth(), depth);
}

#[test]
fn mixed_transform_preserves_elevation_heading_and_group_undo() {
    let mut core = document();
    core.begin_group();
    core.end_group();
    let slots: Value = serde_json::from_str(&core.slots_json()).unwrap();
    let other: Value = serde_json::from_str(&core.small_maps_json()).unwrap();
    let depth = core.undo_depth();
    core.begin_group();
    assert!(operations::transform::align_selection(
        &core,
        operations::placement::AlignEdge::Right,
        vec!["slot".into(), "vehicle".into()],
        |_, _| true,
    ));
    core.end_group();
    let after: Value = serde_json::from_str(&core.slots_json()).unwrap();
    assert_eq!(
        after["slot"]["position"],
        json!({"x":400,"y":200,"z":37.3,"rotation":90})
    );
    let vehicle: Value = serde_json::from_str(&core.small_maps_json()).unwrap();
    assert_eq!(vehicle["vehiclesById"]["vehicle"]["position"]["z"], 42.7);
    assert_eq!(
        vehicle["vehiclesById"]["vehicle"]["position"]["rotation"].as_f64(),
        Some(33.0)
    );
    assert_eq!(core.undo_depth(), depth + 1);
    assert!(core.undo());
    assert_eq!(
        serde_json::from_str::<Value>(&core.slots_json()).unwrap(),
        slots
    );
    assert_eq!(
        serde_json::from_str::<Value>(&core.small_maps_json()).unwrap(),
        other
    );
}

#[test]
fn copied_loadout_is_a_snapshot_and_commits_count_only_existing_targets() {
    let core = document();
    core.update_slot_loadout(
        "slot",
        Some(json!({"primary":"rifle.et","cargo":[{"id":"mag","count":2}]}).to_string()),
    );
    let buffer = operations::cargo::copy_loadouts_from_selection(
        &core,
        vec!["slot".into(), "vehicle".into(), "missing".into()],
    );
    assert_eq!(buffer.len(), 1);
    let snapshot = buffer[0].loadout_json.clone();
    core.update_slot_loadout("slot", None);
    assert_eq!(buffer[0].loadout_json, snapshot);
    let writes = [
        operations::cargo::LoadoutWrite {
            target_id: "missing".into(),
            source_id: Some("slot".into()),
            loadout_json: snapshot.clone(),
        },
        operations::cargo::LoadoutWrite {
            target_id: "slot".into(),
            source_id: Some("slot".into()),
            loadout_json: snapshot.clone(),
        },
    ];
    assert_eq!(operations::cargo::commit_loadout_writes(&core, &writes), 1);
    assert_eq!(
        serde_json::from_str::<Value>(&operations::cargo::read_loadout(&core, "slot").unwrap())
            .unwrap(),
        serde_json::from_str::<Value>(&snapshot.unwrap()).unwrap()
    );
    assert_eq!(operations::cargo::commit_loadout_writes(&core, &[]), 0);
}

#[test]
fn clipboard_paste_preserves_unknown_fields_and_authored_elevation() {
    let core = document();
    let rows: Value = serde_json::from_str(&core.slots_json()).unwrap();
    let mut copied = rows["slot"].clone();
    copied["futureExtension"] = json!({"ordered":[3,1,2],"enabled":true});
    let ids = operations::entity::paste_at_cursor(
        &core,
        vec![copied],
        "layer".into(),
        &Cell::new(0),
        Some(900.0),
        Some(800.0),
    );
    assert_eq!(ids.len(), 1);
    assert_ne!(ids[0], "slot");
    let after: Value = serde_json::from_str(&core.slots_json()).unwrap();
    assert_eq!(
        after[&ids[0]]["futureExtension"],
        json!({"ordered":[3,1,2],"enabled":true})
    );
    assert_eq!(after[&ids[0]]["position"]["z"], 37.3);
    assert_eq!(after["slot"], rows["slot"]);
}
