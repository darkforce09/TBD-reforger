//! **Role:** Domain regression cases.
//! **Position:** `mission_model::environment::weather::tests::cases_1` in the `mission_model` crate.
//! **Signals & state:** explicit data inputs; no UI or graphics state.
//! **Invariants:** preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn a_three_keyframe_block_parses() {
    let got = parse(&three_keyframes()).expect("parses");
    assert_eq!(got.keyframes.len(), 3);
    assert_eq!(got.keyframes[0].at_minutes, 0);
    assert_eq!(got.keyframes[0].weather_preset, "clear");
    assert!(got.keyframes[0].wind_dir_deg.is_none());
    assert!(got.keyframes[0].fog.is_none());
    assert_eq!(got.keyframes[1].wind_dir_deg, Some(90.0));
    assert_eq!(got.keyframes[2].fog, Some(0.4));
    assert_eq!(got.keyframes[2].weather_preset, "heavy_rain");
}

#[test]
fn equal_at_minutes_are_refused() {
    let err = parse(&json!({
        "keyframes": [
            {"atMinutes": 10, "weatherPreset": "clear"},
            {"atMinutes": 10, "weatherPreset": "overcast"}
        ]
    }))
    .expect_err("equal atMinutes must be refused");
    assert!(err.to_string().contains("strictly increasing"), "{err}");
    assert!(err.to_string().contains("10"), "{err}");
    assert!(
        !minutes_strictly_increase(10, 10),
        "the predicate itself must refuse equal offsets"
    );
    validate(&json!({
        "keyframes": [
            {"atMinutes": 10, "weatherPreset": "clear"},
            {"atMinutes": 10, "weatherPreset": "overcast"}
        ]
    }))
    .expect_err("equal atMinutes");
}

#[test]
fn out_of_order_at_minutes_are_refused() {
    let err = parse(&json!({
        "keyframes": [
            {"atMinutes": 20, "weatherPreset": "clear"},
            {"atMinutes": 5, "weatherPreset": "overcast"}
        ]
    }))
    .expect_err("descending");
    assert!(err.to_string().contains("strictly increasing"), "{err}");
    assert!(minutes_strictly_increase(5, 20));
    assert!(!minutes_strictly_increase(20, 5));
}

#[test]
fn fog_outside_unit_interval_is_refused() {
    let err = parse(&json!({
        "keyframes": [{"atMinutes": 0, "weatherPreset": "clear", "fog": 1.5}]
    }))
    .expect_err("fog");
    assert!(err.to_string().contains("fog"), "{err}");
    assert!(err.to_string().contains("0..=1"), "{err}");
}

#[test]
fn copy_authored_blocks_carries_weather_timeline_and_leaves_weather() {
    let timeline = three_keyframes();
    let env = json!({"weather": "clear", "weatherTimeline": timeline});
    let mut dst = serde_json::Map::new();
    let copied = copy_authored_blocks(&env, &mut dst);
    assert_eq!(copied, ["weatherTimeline"]);
    assert_eq!(dst["weatherTimeline"], timeline);
    assert!(
        !dst.contains_key("weather"),
        "the bag's own keys stay in the bag"
    );
}
