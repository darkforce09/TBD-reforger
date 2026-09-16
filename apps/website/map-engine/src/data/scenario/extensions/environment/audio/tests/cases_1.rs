//! Role: Domain regression cases.
//! Position: `mission/extensions/environment/audio/tests` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn a_two_emitter_one_cue_block_parses() {
    let got = parse(&two_and_one()).expect("parses");
    assert_eq!(got.emitters.len(), 2);
    assert_eq!(got.emitters[0].id, "ae-gen");
    assert_eq!(got.emitters[0].radius_m, 25.0);
    assert!(got.emitters[0].loop_sound);
    assert!(got.emitters[0].y.is_none());
    assert_eq!(got.emitters[1].y, Some(12.5));
    assert_eq!(got.emitters[1].trigger_id.as_deref(), Some("tr-door"));
    assert_eq!(got.music_cues.len(), 1);
    assert_eq!(got.music_cues[0].event, "mission_start");
}

#[test]
fn radius_zero_is_refused() {
    let err = parse(&json!({
        "emitters": [{
            "id": "ae-zero",
            "x": 1.0,
            "z": 2.0,
            "sound": "SOUND_HINT",
            "radiusM": 0.0,
            "loop": false
        }],
        "musicCues": []
    }))
    .expect_err("radius 0 must be refused");
    assert!(err.contains("above zero"), "{err}");
    assert!(err.contains("0"), "{err}");
    assert!(
        !radius_above_zero(0.0),
        "the predicate itself must refuse radius 0"
    );
    assert!(radius_above_zero(0.1));
    validate(&json!({
        "emitters": [{
            "id": "ae-zero",
            "x": 1.0,
            "z": 2.0,
            "sound": "SOUND_HINT",
            "radiusM": 0.0,
            "loop": false
        }],
        "musicCues": []
    }))
    .expect_err("radius 0");
}

#[test]
fn a_negative_radius_is_refused() {
    let err = parse(&json!({
        "emitters": [{
            "id": "ae-neg",
            "x": 1.0,
            "z": 2.0,
            "sound": "SOUND_HINT",
            "radiusM": -4.0,
            "loop": false
        }],
        "musicCues": []
    }))
    .expect_err("negative");
    assert!(err.contains("above zero"), "{err}");
}

#[test]
fn an_unknown_event_is_refused() {
    let err = parse(&json!({
        "emitters": [],
        "musicCues": [{
            "id": "mc-bad",
            "event": "round_pause",
            "track": "SOUND_HINT"
        }]
    }))
    .expect_err("unknown event");
    assert!(err.contains("round_pause"), "{err}");
    assert!(err.contains("mission_start"), "{err}");
}

#[test]
fn a_duplicate_id_is_refused() {
    let err = parse(&json!({
        "emitters": [{
            "id": "same",
            "x": 1.0,
            "z": 2.0,
            "sound": "SOUND_HINT",
            "radiusM": 5.0,
            "loop": false
        }],
        "musicCues": [{
            "id": "same",
            "event": "mission_end",
            "track": "SOUND_HINT"
        }]
    }))
    .expect_err("duplicate");
    assert!(err.contains("unique"), "{err}");
}

#[test]
fn an_empty_block_is_refused() {
    let err = parse(&json!({"emitters": [], "musicCues": []})).expect_err("empty");
    assert!(err.contains("empty"), "{err}");
}

#[test]
fn an_unknown_key_is_refused() {
    let err = parse(&json!({
        "emitters": [{
            "id": "ae-1",
            "x": 1.0,
            "z": 2.0,
            "sound": "SOUND_HINT",
            "radiusM": 5.0,
            "loop": false,
            "volume": 2
        }],
        "musicCues": []
    }))
    .expect_err("unknown key");
    assert!(err.contains("volume"), "{err}");
}

#[test]
fn audio_is_registered_on_the_carrier() {
    assert!(
        is_authored_block("audio"),
        "T-936.5's row must be in AUTHORED_BLOCKS or the carrier never emits it"
    );
    assert!(is_authored_block("spawnModules"));
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

#[test]
fn copy_authored_blocks_carries_audio_and_leaves_weather() {
    let block = two_and_one();
    let env = json!({"weather": "clear", "audio": block});
    let mut dst = serde_json::Map::new();
    let copied = copy_authored_blocks(&env, &mut dst);
    assert!(copied.contains(&"audio"), "{copied:?}");
    assert_eq!(dst["audio"], block);
    assert!(
        !dst.contains_key("weather"),
        "the bag's own keys stay in the bag"
    );
}
