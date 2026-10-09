//! The payload compiler over a document the CRDT store hydrated or a writer authored.
//!
//! **Role:** proves authored briefing prose survives a hydrate into the document core and a
//! recompile from its maps, byte for byte, and that the vehicle rows `add_vehicle`,
//! `attach_vehicle` and `set_vehicle_cargo` write compile to the shape the vehicle module of
//! `mission_compiler` reads: `id`, `resourceName`, the four-axis `position`, `squadId` and `cargo`.
//! **Position:** a store test: it needs both the payload compiler and [`super::MissionDocCore`],
//! so it lives with the store, which links both.
//! **Signals & state:** one document core per test.
//! **Invariants:** the briefing fixture matches the payload tests' fixture byte for byte; an
//! unplaced, unattached vehicle carries neither `position` nor `squadId`.

use super::MissionDocCore;
use mission_payload::compile_payload;
use serde_json::{Value, json};

const SITUATION: &str =
    "Soviet airborne hold the Levie crossing.\n\nTwo BMPs were seen at the eastern abutment.";

const MISSION: &str = "Seize Levie Bridge.\nHold it to the time limit.";

const EXECUTION: &str = "Alpha advances from the western treeline.\n\n\
                             - MG support from Hill 214\n- Sappers follow on foot";

fn authored_briefing() -> Value {
    json!({ "situation": SITUATION, "mission": MISSION, "execution": EXECUTION })
}

#[test]
fn briefing_prose_round_trips_through_the_document_core() {
    let payload = json!({
        "schemaVersion": 1,
        "map": { "terrain": "everon" },
        "editor": {
            "factions": [
                {
                    "id": "fa", "key": "BLUFOR", "name": "US Army", "squadIds": ["s1"],
                    "briefing": authored_briefing()
                },
                { "id": "fb", "key": "OPFOR", "name": "Soviet VDV", "squadIds": [] }
            ],
            "squads": [
                { "id": "s1", "factionId": "fa", "callsign": "Alpha", "name": "1st",
                  "slotIds": ["z1"] }
            ],
            "slots": [
                { "id": "z1", "squadId": "s1", "index": 0, "role": "SL",
                  "position": { "x": 4839.2, "y": 6620.8, "z": 0.0, "rotation": 270.0 } }
            ],
            "editorLayers": []
        }
    })
    .to_string();

    let doc = MissionDocCore::new();
    doc.hydrate(&payload, "layer-1");

    let small = doc.small_maps_json();
    let parsed: Value = serde_json::from_str(&small).expect("small_maps_json is JSON");
    assert_eq!(
        parsed["factionsById"]["fa"]["briefing"],
        authored_briefing()
    );

    let recompiled = compile_payload(&small, &doc.slots_json(), false);
    assert_eq!(
        recompiled["editor"]["factions"][0]["briefing"],
        authored_briefing()
    );

    let situation = recompiled["editor"]["factions"][0]["briefing"]["situation"]
        .as_str()
        .expect("situation is a string");
    assert_eq!(situation, SITUATION);
    assert!(situation.contains("\n\n"), "{situation:?}");

    assert_eq!(recompiled["editor"]["slots"].as_array().unwrap().len(), 1);
    assert_eq!(recompiled["map"]["terrain"], json!("everon"));
}

fn vehicles_from_writer_json_roundtrip() -> serde_json::Value {
    let doc = MissionDocCore::new();
    doc.add_faction("f1", "BLUFOR", "US Army");
    doc.add_squad("sq1", "f1", "Alpha 1-1", Some("Alpha".into()));
    doc.add_vehicle(
        "v1",
        "{F6B23D17D5067C11}Prefabs/Vehicles/Wheeled/M151A2/M151A2_M2HB.et",
        Some(100.5),
        Some(200.5),
        Some(3.0),
        Some(45.0),
    );
    doc.attach_vehicle("sq1", "v1");
    doc.set_vehicle_cargo("v1", &[("res://ammo".into(), 4)]);
    doc.add_vehicle(
        "v2",
        "{ABCDEF0123456789}Prefabs/Vehicles/Wheeled/UAZ/UAZ469.et",
        None,
        None,
        None,
        None,
    );

    let payload = compile_payload(&doc.small_maps_json(), &doc.slots_json(), false);
    let bytes = serde_json::to_vec(&payload).expect("payload serializes");
    serde_json::from_slice(&bytes).expect("payload deserializes")
}

#[test]
fn the_vehicle_row_still_has_the_shape_this_module_reads() {
    let authored = vehicles_from_writer_json_roundtrip();
    let vehicles = authored["vehicles"]
        .as_array()
        .expect("compile_payload must emit a vehicles array from add_vehicle output");
    assert_eq!(
        vehicles.len(),
        2,
        "writer authored two vehicles (placed+attached v1, bare v2)"
    );

    let by_id = |id: &str| -> &serde_json::Value {
        vehicles
            .iter()
            .find(|v| v.get("id").and_then(serde_json::Value::as_str) == Some(id))
            .unwrap_or_else(|| panic!("missing vehicle id={id} in writer round-trip"))
    };

    for id in ["v1", "v2"] {
        let v = by_id(id);
        for required in ["id", "resourceName"] {
            assert!(
                v.get(required)
                    .and_then(serde_json::Value::as_str)
                    .is_some(),
                "vehicles[{id}].{required} must be a non-null string — the floor \
                     add_vehicle writes. If the writing slice renamed or retyped it, these two \
                     halves now disagree and nothing else will say so."
            );
        }
    }

    let placed = by_id("v1");
    let pos = placed["position"]
        .as_object()
        .expect("vehicles[v1].position is an object (store.rs `position_any`)");
    let mut axes: Vec<&str> = pos.keys().map(String::as_str).collect();
    axes.sort_unstable();
    assert_eq!(
        axes,
        ["rotation", "x", "y", "z"],
        "the vehicle position shape changed; `position_any` writes exactly these four"
    );
    assert!(
        pos.values().all(serde_json::Value::is_number),
        "every vehicle position axis is a number"
    );
    assert_eq!(placed["squadId"], "sq1");

    let bare = by_id("v2");
    assert!(
        bare.get("position").is_none() && bare.get("squadId").is_none(),
        "an unplaced, unattached vehicle carries neither key — a reader that requires \
             either would silently drop it"
    );

    let cargo = placed
        .get("cargo")
        .and_then(serde_json::Value::as_array)
        .expect("set_vehicle_cargo + compile_payload must preserve vehicles[].cargo");
    assert_eq!(cargo.len(), 1, "one cargo row authored");
    assert_eq!(cargo[0]["item"], "res://ammo");
    assert_eq!(cargo[0]["qty"], 4);
    assert!(
        placed
            .get("resourceName")
            .and_then(serde_json::Value::as_str)
            .is_some(),
        "floor resourceName must survive beside the cargo extra"
    );
    assert!(
        placed.get("position").and_then(|p| p.get("x")).is_some(),
        "floor position.x must survive beside the cargo extra"
    );
}
