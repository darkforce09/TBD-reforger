//! Round trips of the weather timeline through the payload compiler.
//!
//! **Role:** proves the compiler promotes an authored `weatherTimeline` block out of the environment bag
//! onto the payload root and omits the key when nothing is authored; where the block rides the
//! extension carrier, the carrier reads the compiled block back.
//! **Position:** a payload test over the extension block registry of the mission model.
//! **Signals & state:** none; pure functions over JSON fixtures.
//! **Invariants:** the fixtures match the extension's own test fixtures byte for byte.

use super::*;

fn three_keyframes() -> Value {
    json!({
        "keyframes": [
            {"atMinutes": 0, "weatherPreset": "clear"},
            {"atMinutes": 15, "weatherPreset": "overcast", "windDirDeg": 90.0},
            {"atMinutes": 40, "weatherPreset": "heavy_rain", "fog": 0.4}
        ]
    })
}

fn compile_env_with_timeline(timeline: &Value) -> Value {
    compile_payload(
        &json!({
            "meta": {
                "terrain": "everon",
                "environment": { "weather": "clear", "weatherTimeline": timeline }
            }
        })
        .to_string(),
        "{}",
        false,
    )
}

#[test]
fn a_three_keyframe_mission_copies_to_the_payload_root() {
    let timeline = three_keyframes();
    let p = compile_env_with_timeline(&timeline);
    assert_eq!(
        p["weatherTimeline"], timeline,
        "AUTHORED_BLOCKS must promote weatherTimeline out of the env bag: {p:#}"
    );
    assert_eq!(
        p["weatherTimeline"]["keyframes"]
            .as_array()
            .expect("array")
            .len(),
        3
    );

    let (carried, refusals) = ExtensionBlocks::from_payload(&p);
    assert!(refusals.is_empty(), "{refusals:?}");
    assert_eq!(carried.get("weatherTimeline"), Some(&timeline));
}

#[test]
fn an_unauthored_payload_still_omits_the_weather_timeline_key() {
    let p = compile_payload(
        &json!({"meta": {"terrain": "everon", "environment": {"weather": "clear"}}}).to_string(),
        "{}",
        false,
    );
    assert!(
        p.get("weatherTimeline").is_none(),
        "parity: no weatherTimeline authored ⇒ no weatherTimeline key: {p:#}"
    );
    let (carried, refusals) = ExtensionBlocks::from_payload(&p);
    assert!(refusals.is_empty(), "{refusals:?}");
    assert!(carried.get("weatherTimeline").is_none());
}
