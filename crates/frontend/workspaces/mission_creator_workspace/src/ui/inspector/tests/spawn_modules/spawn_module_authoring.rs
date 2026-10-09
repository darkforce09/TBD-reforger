//! Tests the spawn modules subject.

use super::*;
use serde_json::json;

fn wave() -> Value {
    json!({
        "id": "sm-wave",
        "kind": "wave",
        "factionKey": "opfor",
        "groupTemplate": "Group_Base",
        "x": 1.0,
        "z": 2.0,
        "count": 2,
        "intervalSeconds": 30.0,
        "maxAlive": 4
    })
}

#[test]
fn add_appends_a_valid_default_wave() {
    let next = add_module(&[]).expect("add");
    assert_eq!(next.len(), 1);
    assert_eq!(next[0]["kind"], "wave");
    assert_eq!(next[0]["id"], "sm-1");
    validate(&block_from_modules(&next).expect("block")).expect("valid");
}

#[test]
fn remove_drops_one_row_and_clearing_the_last_writes_null() {
    let next = remove_module(&[wave()], 0);
    assert!(next.is_empty());
    let cleared: Value = serde_json::from_str(&env_patch(None)).expect("json");
    assert_eq!(cleared, json!({"spawnModules": null}));
}

#[test]
fn blank_optional_interval_is_stripped() {
    let next = with_field(&[wave()], 0, "intervalSeconds", "  ").expect("blank");
    assert!(next[0].get("intervalSeconds").is_none());
    validate(&block_from_modules(&next).expect("block")).expect("valid");
}

#[test]
fn env_patch_sets_and_clears() {
    let set: Value =
        serde_json::from_str(&env_patch(block_from_modules(&[wave()]).as_ref())).expect("json");
    assert_eq!(set["spawnModules"][0]["kind"], "wave");
    let cleared: Value = serde_json::from_str(&env_patch(None)).expect("json");
    assert_eq!(cleared, json!({"spawnModules": null}));
}
