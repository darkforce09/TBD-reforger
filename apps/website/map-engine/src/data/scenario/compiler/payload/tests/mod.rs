//! Role: Module boundary for mission/compiler/payload/tests.
//! Position: `mission/compiler/payload/tests` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

fn small_maps() -> String {
    json!({
        "meta": Value::Null,
        "factionsById": {
            "fa": { "key": "BLUFOR", "squadIds": ["s1"] },
            "fb": { "key": "OPFOR",  "squadIds": ["s2"] }
        },
        "squadsById": {
            "s1": { "id": "s1", "callsign": "Alpha", "name": "1st", "slotIds": ["z2", "z1"] },
            "s2": { "id": "s2", "callsign": "Bravo", "name": "2nd", "slotIds": ["z3"] }
        },
        "loadoutsById": {},
        "itemsById": {},
        "objectivesById": {},
        "vehiclesById": {},
        "entitiesById": {},
        "markersById": {},
        "editorLayersById": {}
    })
    .to_string()
}

fn slots() -> String {
    json!({
        "z1": { "id": "z1", "index": 5, "role": "SL",       "tag": "CMD" },
        "z2": { "id": "z2", "index": 1, "role": "Rifleman", "tag": "" },
        "z3": { "id": "z3", "index": 0, "role": "MED",      "tag": "MED" }
    })
    .to_string()
}

const SITUATION: &str =
    "Soviet airborne hold the Levie crossing.\n\nTwo BMPs were seen at the eastern abutment.";

const MISSION: &str = "Seize Levie Bridge.\nHold it to the time limit.";

const EXECUTION: &str = "Alpha advances from the western treeline.\n\n\
                             - MG support from Hill 214\n- Sappers follow on foot";

fn authored_briefing() -> Value {
    json!({ "situation": SITUATION, "mission": MISSION, "execution": EXECUTION })
}

mod cases_1;
