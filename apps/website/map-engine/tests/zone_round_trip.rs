//! Role: zone round trip.
//! Position: `apps/website/map-engine/tests` in the map engine.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

#![cfg(feature = "store")]

use serde_json::Value;
use website_map_engine::data::scenario::compile::compile_payload;
use website_map_engine::data::store::MissionDocCore;

const DEFAULT_LAYER: &str = "layer-1";

fn authored() -> MissionDocCore {
    let core = MissionDocCore::new();
    core.add_circle_zone("z1", "boundary", 1234.5, 6789.2, 250.7);
    core.set_zone_label("z1", Some("Area of Operations"));
    core.set_zone_faction("z1", Some("blufor"));
    core.set_zone_rules("z1", Some(r#"{"graceSeconds":45,"penalty":"kill"}"#));

    core.add_polygon_zone(
        "z2",
        "objective_capture",
        &[100.0, 200.0, 340.5, 210.25, 275.0, 480.75, 120.0, 460.0],
    );
    core.set_zone_label("z2", Some("Hilltop"));
    core.set_zone_rules("z2", Some(r#"{"captureSeconds":180,"contestable":false}"#));
    core
}

fn save_then_reload(core: &MissionDocCore) -> Value {
    let payload = compile_payload(&core.small_maps_json(), &core.slots_json(), false);
    let fresh = MissionDocCore::new();
    fresh.hydrate(&payload.to_string(), DEFAULT_LAYER);
    serde_json::from_str(&fresh.zones_json()).expect("zones_json is JSON")
}

#[test]
fn zone_geometry_survives_save_and_reload() {
    let core = authored();
    let before: Value = serde_json::from_str(&core.zones_json()).expect("zones_json is JSON");
    let after = save_then_reload(&core);

    assert_eq!(
        core.zone_count(),
        2,
        "the tool must have authored two zones"
    );
    assert!(
        before.get("z1").is_some() && before.get("z2").is_some(),
        "precondition: both zones exist before the save"
    );

    let c = after
        .get("z1")
        .expect("the circle zone must survive the round trip");
    assert_eq!(c["type"], "boundary");
    assert_eq!(c["label"], "Area of Operations");
    assert_eq!(c["faction"], "blufor");
    let circle = &c["shape"]["circle"];
    assert_eq!(
        circle["x"].as_f64(),
        Some(1234.5),
        "centre x must survive whole"
    );
    assert_eq!(circle["z"].as_f64(), Some(6789.2), "centre z, not y");
    assert_eq!(
        circle["r"].as_f64(),
        Some(250.7),
        "the radius is the geometry — a dropped r is the T-211 bug class"
    );
    assert!(
        c["shape"].get("polygon").is_none(),
        "$defs/shape is a oneOf: a circle row carrying a polygon key is schema-INVALID"
    );

    assert_eq!(c["rules"]["graceSeconds"].as_f64(), Some(45.0));
    assert_eq!(c["rules"]["penalty"], "kill");

    let p = after
        .get("z2")
        .expect("the polygon zone must survive the round trip");
    assert_eq!(p["type"], "objective_capture");
    assert_eq!(p["label"], "Hilltop");
    let ring = p["shape"]["polygon"]
        .as_array()
        .expect("the ring must survive as an array");
    assert_eq!(ring.len(), 4, "every vertex, not just the first");
    let flat: Vec<f64> = ring
        .iter()
        .flat_map(|v| v.as_array().expect("a [x,z] pair").iter())
        .map(|n| n.as_f64().expect("numeric vertex"))
        .collect();
    assert_eq!(
        flat,
        vec![100.0, 200.0, 340.5, 210.25, 275.0, 480.75, 120.0, 460.0],
        "the ring must come back in order and unrounded — a reordered or truncated ring is a \
             different play area"
    );
    assert!(
        p["shape"].get("circle").is_none(),
        "$defs/shape oneOf, from the other side"
    );
    assert_eq!(p["rules"]["captureSeconds"].as_f64(), Some(180.0));
    assert_eq!(p["rules"]["contestable"], false);

    assert_eq!(
        before, after,
        "the reloaded zones map must equal the authored one exactly"
    );
}

#[test]
fn zones_reach_the_payload_root_through_the_projection() {
    let core = authored();
    let small: Value = serde_json::from_str(&core.small_maps_json()).expect("JSON");

    assert!(
        small["zonesById"]["z1"].is_object(),
        "small_maps_json must carry the canonical by-id map"
    );

    let parked = small["payloadExtras"]["zones"]
        .as_array()
        .expect("the projection must park an ordered array");
    assert_eq!(parked.len(), 2);

    let payload = compile_payload(&core.small_maps_json(), &core.slots_json(), false);
    let root = payload["zones"]
        .as_array()
        .expect("compile_payload must promote payloadExtras.zones to the payload ROOT");
    assert_eq!(
        root.len(),
        2,
        "both zones must reach the root array flatten reads"
    );
    assert_eq!(
        root, parked,
        "promotion must not reshape or reorder the rows"
    );
}

#[test]
fn deleting_every_zone_survives_the_round_trip_too() {
    let core = authored();
    core.remove_zone("z1");
    core.remove_zone("z2");
    assert_eq!(core.zone_count(), 0);

    let after = save_then_reload(&core);
    assert_eq!(
        after.as_object().map(serde_json::Map::len),
        Some(0),
        "a cleared play area must stay cleared across a save and reload"
    );
}
