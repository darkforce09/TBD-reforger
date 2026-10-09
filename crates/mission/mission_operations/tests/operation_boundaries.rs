//! **Role:** exercise editor operations through the public headless boundary.
//! **Position:** an integration suite of `mission_operations`, over the mission document, the
//! payload compiler and the arrange vocabulary the authoring commands drive.
//! **Signals & state:** deterministic documents and explicit host-policy callbacks.
//! **Invariants:** cancellation is inert; edits retain authored precision and undo behavior.

use mission_document::MissionDocCore;
use serde_json::{Value, json};
use std::cell::Cell;

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
    assert!(!mission_operations::transform::align_selection(
        &core,
        formation_geometry::AlignEdge::Left,
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
    assert!(mission_operations::transform::align_selection(
        &core,
        formation_geometry::AlignEdge::Right,
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
    let buffer = mission_operations::cargo::copy_loadouts_from_selection(
        &core,
        vec!["slot".into(), "vehicle".into(), "missing".into()],
    );
    assert_eq!(buffer.len(), 1);
    let snapshot = buffer[0].loadout_json.clone();
    core.update_slot_loadout("slot", None);
    assert_eq!(buffer[0].loadout_json, snapshot);
    let writes = [
        mission_operations::cargo::LoadoutWrite {
            target_id: "missing".into(),
            source_id: Some("slot".into()),
            loadout_json: snapshot.clone(),
        },
        mission_operations::cargo::LoadoutWrite {
            target_id: "slot".into(),
            source_id: Some("slot".into()),
            loadout_json: snapshot.clone(),
        },
    ];
    assert_eq!(
        mission_operations::cargo::commit_loadout_writes(&core, &writes),
        1
    );
    assert_eq!(
        serde_json::from_str::<Value>(
            &mission_operations::cargo::read_loadout(&core, "slot").unwrap()
        )
        .unwrap(),
        serde_json::from_str::<Value>(&snapshot.unwrap()).unwrap()
    );
    assert_eq!(
        mission_operations::cargo::commit_loadout_writes(&core, &[]),
        0
    );
}

#[test]
fn clipboard_paste_preserves_unknown_fields_and_authored_elevation() {
    let core = document();
    let rows: Value = serde_json::from_str(&core.slots_json()).unwrap();
    let mut copied = rows["slot"].clone();
    copied["futureExtension"] = json!({"ordered":[3,1,2],"enabled":true});
    let ids = mission_operations::entity::paste_at_cursor(
        &core,
        vec![copied],
        "layer",
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

const ROOFTOP_Z: f64 = 37.3;

#[test]
fn a_pasted_slot_keeps_the_authored_z_of_the_slot_it_was_copied_from() {
    let doc = MissionDocCore::new();
    doc.add_editor_layer("lyr", "Default", None);
    doc.add_slot(
        "src", "sq1", "lyr", 0, "Rifleman", None, None, 100.0, 200.0, ROOFTOP_Z, 90.0,
    );

    let rows: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("valid json");
    let clip = rows["src"].clone();
    let num = |k: &str| {
        clip["position"][k]
            .as_f64()
            .unwrap_or_else(|| panic!("clipboard row must carry position.{k}: {clip}"))
    };
    assert_eq!(
        num("z"),
        ROOFTOP_Z,
        "precondition: the clipboard row carries the authored z, so the paste has it in hand \
         without a second document read"
    );

    doc.paste_slots(
        vec!["copy".into()],
        vec!["sq1".into()],
        vec!["lyr".into()],
        vec![num("x")],
        vec![num("y")],
        vec![num("rotation")],
        vec![num("z")],
        vec!["Rifleman".into()],
        vec![String::new()],
        vec![String::new()],
        vec!["stand".into()],
        vec![String::new()],
        vec![String::new()],
        Some(400.0),
        Some(500.0),
        12800.0,
        12800.0,
    );

    let after: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("valid json");
    assert_eq!(
        after["copy"]["position"]["z"].as_f64(),
        Some(ROOFTOP_Z),
        "the copy must land at the elevation it was copied from, exactly — no terrain-follow and \
         no f32 round trip. Slots after paste: {after}"
    );

    assert_eq!(after["src"]["position"]["z"].as_f64(), Some(ROOFTOP_Z));

    doc.add_slot(
        "flat", "sq1", "lyr", 1, "Rifleman", None, None, 10.0, 20.0, 0.0, 0.0,
    );
    doc.paste_slots(
        vec!["flat_copy".into()],
        vec!["sq1".into()],
        vec!["lyr".into()],
        vec![10.0],
        vec![20.0],
        vec![0.0],
        vec![0.0],
        vec!["Rifleman".into()],
        vec![String::new()],
        vec![String::new()],
        vec!["stand".into()],
        vec![String::new()],
        vec![String::new()],
        Some(50.0),
        Some(60.0),
        12800.0,
        12800.0,
    );
    let after: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("valid json");
    assert_eq!(
        after["flat_copy"]["position"]["z"].as_f64(),
        Some(0.0),
        "carrying the source z through must not invent an elevation for a flat-map paste"
    );
}

#[test]
fn a_multi_slot_paste_gives_each_copy_its_own_source_elevation() {
    let doc = MissionDocCore::new();
    doc.add_editor_layer("lyr", "Default", None);
    let zs = vec![ROOFTOP_Z, 0.0, -4.75];
    doc.paste_slots(
        vec!["a".into(), "b".into(), "c".into()],
        vec!["sq1".into(); 3],
        vec!["lyr".into(); 3],
        vec![10.0, 20.0, 30.0],
        vec![10.0, 20.0, 30.0],
        vec![0.0, 0.0, 0.0],
        zs.clone(),
        vec!["Rifleman".into(); 3],
        vec![String::new(); 3],
        vec![String::new(); 3],
        vec!["stand".into(); 3],
        vec![String::new(); 3],
        vec![String::new(); 3],
        Some(100.0),
        Some(100.0),
        12800.0,
        12800.0,
    );
    let after: serde_json::Value = serde_json::from_str(&doc.slots_json()).expect("valid json");
    for (id, z) in ["a", "b", "c"].iter().zip(zs) {
        assert_eq!(
            after[*id]["position"]["z"].as_f64(),
            Some(z),
            "slot {id} must get index-aligned z {z}; slots were: {after}"
        );
    }
}
