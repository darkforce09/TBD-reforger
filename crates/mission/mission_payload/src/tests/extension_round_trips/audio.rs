//! Round trips of the audio block through the payload compiler.
//!
//! **Role:** proves the compiler promotes an authored `audio` block out of the environment bag
//! onto the payload root and omits the key when nothing is authored; where the block rides the
//! extension carrier, the carrier reads the compiled block back.
//! **Position:** a payload test over the extension block registry of the mission model.
//! **Signals & state:** none; pure functions over JSON fixtures.
//! **Invariants:** the fixtures match the extension's own test fixtures byte for byte.

use super::*;

fn two_and_one() -> Value {
    json!({
        "emitters": [
            {
                "id": "ae-gen",
                "x": 100.0,
                "z": 200.0,
                "sound": "SOUND_HINT",
                "radiusM": 25.0,
                "loop": true
            },
            {
                "id": "ae-shot",
                "x": 300.0,
                "z": 400.0,
                "y": 12.5,
                "sound": "SOUND_FEVER",
                "radiusM": 8.0,
                "loop": false,
                "triggerId": "tr-door"
            }
        ],
        "musicCues": [
            {
                "id": "mc-start",
                "event": "mission_start",
                "track": "SOUND_HINT"
            }
        ]
    })
}

fn compile_env_with_audio(block: &Value) -> Value {
    compile_payload(
        &json!({
            "meta": {
                "terrain": "everon",
                "environment": { "weather": "clear", "audio": block }
            }
        })
        .to_string(),
        "{}",
        false,
    )
}

#[test]
fn a_two_emitter_mission_copies_to_the_payload_root() {
    let block = two_and_one();
    let p = compile_env_with_audio(&block);
    assert_eq!(
        p["audio"], block,
        "AUTHORED_BLOCKS must promote audio out of the env bag: {p:#}"
    );
    assert_eq!(p["audio"]["emitters"].as_array().expect("array").len(), 2);
    assert_eq!(p["audio"]["musicCues"].as_array().expect("array").len(), 1);

    let (carried, refusals) = ExtensionBlocks::from_payload(&p);
    assert!(refusals.is_empty(), "{refusals:?}");
    assert_eq!(carried.get("audio"), Some(&block));
}

#[test]
fn an_unauthored_payload_still_omits_the_audio_key() {
    let p = compile_payload(
        &json!({"meta": {"terrain": "everon", "environment": {"weather": "clear"}}}).to_string(),
        "{}",
        false,
    );
    assert!(
        p.get("audio").is_none(),
        "parity: no audio authored ⇒ no audio key: {p:#}"
    );
    let (carried, refusals) = ExtensionBlocks::from_payload(&p);
    assert!(refusals.is_empty(), "{refusals:?}");
    assert!(carried.get("audio").is_none());
}
