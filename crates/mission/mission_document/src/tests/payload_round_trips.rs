//! The payload compiler over a document the CRDT store hydrated.
//!
//! **Role:** proves authored briefing prose survives a hydrate into the document core and a
//! recompile from its maps, byte for byte.
//! **Position:** a store test: it needs both the payload compiler and [`super::MissionDocCore`],
//! so it lives with the store, which links both.
//! **Signals & state:** one document core per test.
//! **Invariants:** the briefing fixture matches the payload tests' fixture byte for byte.

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
