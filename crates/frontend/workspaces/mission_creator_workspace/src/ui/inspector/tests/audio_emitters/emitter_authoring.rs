//! Tests the audio emitters subject.

use super::*;
use serde_json::json;

fn ae() -> Value {
    json!({"id": "ae-1", "x": 1.0, "z": 2.0, "sound": "SOUND_HINT", "radiusM": 10.0, "loop": true})
}

#[test]
fn add_appends_a_valid_emitter() {
    let next = add_emitter(&[], &[]).expect("add");
    assert_eq!(next.len(), 1);
    validate(&block_from_parts(&next, &[]).expect("block")).expect("valid");
}

#[test]
fn remove_drops_and_clearing_the_last_writes_null() {
    let next = remove_at(&[ae()], 0);
    assert!(next.is_empty());
    let cleared: Value = serde_json::from_str(&env_patch(None)).expect("json");
    assert_eq!(cleared, json!({"audio": null}));
}

#[test]
fn last_marker_fills_xz() {
    let next = apply_marker_xz(&[ae()], &[], 0, 6400.0, 1200.0).expect("xz");
    assert_eq!(next[0]["x"], 6400.0);
    assert_eq!(next[0]["z"], 1200.0);
    assert_eq!(
        xz_from_last_marker(&[(1.0, 2.0), (9.0, 8.0)]),
        Some((9.0, 8.0))
    );
    assert!(xz_from_last_marker(&[]).is_none());
}

#[test]
fn env_patch_sets_and_clears() {
    let set: Value =
        serde_json::from_str(&env_patch(block_from_parts(&[ae()], &[]).as_ref())).expect("json");
    assert_eq!(set["audio"]["emitters"][0]["id"], "ae-1");
    let cleared: Value = serde_json::from_str(&env_patch(None)).expect("json");
    assert_eq!(cleared, json!({"audio": null}));
}
