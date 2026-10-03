//! Tests of the world-layer switches: their defaults and their stored JSON spelling.

use super::*;

#[test]
fn every_world_layer_starts_on_except_props() {
    let defaults = WorldLayerPrefs::default();
    assert!(
        !defaults.props,
        "props are off until the user asks for them"
    );
    let on = [
        defaults.roads,
        defaults.buildings,
        defaults.forest,
        defaults.trees,
        defaults.contours,
        defaults.sea,
        defaults.fences,
        defaults.airfield,
        defaults.heights,
        defaults.town_labels,
        defaults.road_names,
    ];
    assert!(on.iter().all(|layer| *layer), "every other layer starts on");
}

#[test]
fn the_label_switches_are_stored_in_camel_case() {
    let stored = serde_json::to_value(WorldLayerPrefs::default()).expect("the switches serialise");
    let object = stored
        .as_object()
        .expect("the switches serialise as one object");
    assert!(object.contains_key("townLabels") && object.contains_key("roadNames"));
    assert!(!object.contains_key("town_labels") && !object.contains_key("road_names"));
    assert_eq!(object.len(), 12, "one key per switch");
    let round_trip: WorldLayerPrefs =
        serde_json::from_value(stored).expect("the stored spelling reads back");
    assert_eq!(round_trip, WorldLayerPrefs::default());
}
