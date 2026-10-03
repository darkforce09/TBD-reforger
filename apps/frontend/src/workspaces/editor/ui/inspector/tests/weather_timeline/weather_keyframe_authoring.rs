//! Tests the weather timeline subject.

use super::*;
use serde_json::json;

fn t0() -> Value {
    json!({"atMinutes": 0, "weatherPreset": "clear"})
}

fn t15() -> Value {
    json!({"atMinutes": 15, "weatherPreset": "overcast", "windDirDeg": 90.0})
}

#[test]
fn add_appends_fifteen_minutes_after_the_last() {
    let next = add_keyframe(&[t0()]).expect("add");
    assert_eq!(next.len(), 2);
    assert_eq!(next[1]["atMinutes"], 15);
    assert_eq!(next[1]["weatherPreset"], "clear");
    validate(&timeline_from_keyframes(&next).expect("block")).expect("valid");
}

#[test]
fn remove_drops_one_row_and_clearing_the_last_writes_null() {
    let next = remove_keyframe(&[t0()], 0);
    assert!(next.is_empty());
    let cleared: Value = serde_json::from_str(&env_patch(None)).expect("json");
    assert_eq!(cleared, json!({"weatherTimeline": null}));
}

#[test]
fn reorder_swaps_weather_and_keeps_times_increasing() {
    let a = t0();
    let b = t15();
    let up = move_keyframe(&[a.clone(), b.clone()], 1, -1);
    assert_eq!(up[0]["atMinutes"], 0);
    assert_eq!(up[0]["weatherPreset"], "overcast");
    assert_eq!(up[1]["atMinutes"], 15);
    assert_eq!(up[1]["weatherPreset"], "clear");
    validate(&timeline_from_keyframes(&up).expect("block")).expect("still increasing");
    let down = move_keyframe(&up, 0, 1);
    assert_eq!(down[0]["weatherPreset"], "clear");
    assert_eq!(down[1]["weatherPreset"], "overcast");
    assert_eq!(move_keyframe(&down, 0, -1), down);
    assert_eq!(move_keyframe(&down, 1, 1), down);
}

#[test]
fn equal_at_minutes_are_refused_in_the_panel() {
    let err = with_field(&[t0(), t15()], 1, "atMinutes", "0").expect_err("equal");
    assert!(err.contains("strictly increasing"), "{err}");
}

#[test]
fn out_of_order_at_minutes_are_refused_in_the_panel() {
    let err = with_field(&[t0(), t15()], 1, "atMinutes", "-1").expect_err("neg");
    assert!(err.contains("negative"), "{err}");
    let err = with_field(&[t0(), t15()], 0, "atMinutes", "20").expect_err("invert");
    assert!(err.contains("strictly increasing"), "{err}");
}

#[test]
fn an_unknown_preset_is_refused() {
    let err = with_field(&[t0()], 0, "weatherPreset", "hailstorm").expect_err("preset");
    assert!(err.contains("hailstorm"), "{err}");
}

#[test]
fn blank_optional_wind_and_fog_are_stripped() {
    let next = with_field(&[t15()], 0, "windDirDeg", "  ").expect("blank wind");
    assert!(next[0].get("windDirDeg").is_none());
    let next = with_field(&next, 0, "fog", "0.25").expect("fog");
    assert_eq!(next[0]["fog"], 0.25);
    let next = with_field(&next, 0, "fog", "").expect("blank fog");
    assert!(next[0].get("fog").is_none());
    validate(&timeline_from_keyframes(&next).expect("block")).expect("valid");
}

#[test]
fn the_pickers_offer_exactly_the_schema_vocabulary() {
    assert_eq!(
        WEATHER_PRESETS,
        ["clear", "overcast", "heavy_rain", "dense_fog"]
    );
    for p in WEATHER_PRESETS {
        assert_ne!(preset_label(p), "Unknown preset");
    }
}

#[test]
fn env_patch_sets_and_clears() {
    let set: Value =
        serde_json::from_str(&env_patch(timeline_from_keyframes(&[t0()]).as_ref())).expect("json");
    assert_eq!(
        set["weatherTimeline"]["keyframes"][0]["weatherPreset"],
        "clear"
    );
    let cleared: Value = serde_json::from_str(&env_patch(None)).expect("json");
    assert_eq!(cleared, json!({"weatherTimeline": null}));
}

#[test]
fn the_reader_chain_names_every_hop() {
    let hops: Vec<&str> = WEATHER_TIMELINE_READERS.iter().map(|(h, _)| *h).collect();
    assert_eq!(hops, ["compile", "flatten", "mod", "editor"]);
    for (hop, reader) in WEATHER_TIMELINE_READERS {
        assert!(reader.len() > 30, "{hop}'s reader is not named: {reader}");
    }
}
