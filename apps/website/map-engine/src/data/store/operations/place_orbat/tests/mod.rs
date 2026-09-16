//! Role: Module boundary for doc/operations/place_orbat/tests.
//! Position: `doc/operations/place_orbat/tests` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

use serde_json::Value;

fn layer(doc: &MissionDocCore) {
    doc.add_editor_layer("lyr", "Layer 1", None);
}

fn place(
    doc: &MissionDocCore,
    side: &str,
    slot: &str,
) -> Result<(String, String, String), PlaceOrbatError> {
    place_character_under_side(
        doc,
        side,
        slot,
        "lyr",
        "Rifleman",
        None,
        Some("asset/Rifleman.et".into()),
        100.0,
        200.0,
        0.0,
        0.0,
    )
}

fn small(doc: &MissionDocCore) -> Value {
    serde_json::from_str(&doc.small_maps_json()).expect("small_maps_json")
}

fn slots(doc: &MissionDocCore) -> Value {
    serde_json::from_str(&doc.slots_json()).expect("slots_json")
}

fn squad_ids(doc: &MissionDocCore, side: &str) -> Vec<String> {
    small(doc)["factionsById"][format!("faction-{side}")]["squadIds"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

#[allow(clippy::too_many_arguments)]
fn place_at(doc: &MissionDocCore, side: &str, slot: &str, x: f64, y: f64) -> String {
    let (_, sid, _) = place_character_under_side(
        doc,
        side,
        slot,
        "lyr",
        "Rifleman",
        None,
        Some("asset/Rifleman.et".into()),
        x,
        y,
        0.0,
        0.0,
    )
    .expect("place");
    sid
}

mod cases_1;
