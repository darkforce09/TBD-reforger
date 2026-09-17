//! Tests the spawn modules subject.

use super::*;
use serde_json::json;
use website_map_engine::data::scenario::spawn_modules::placement_is_exclusive;

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
fn both_position_and_zone_are_refused_in_the_panel() {
    let with_zone = with_field(&[wave()], 0, "zoneId", "z1").expect("zone strips xz");
    assert!(with_zone[0].get("x").is_none());
    assert!(with_zone[0].get("z").is_none());
    assert_eq!(with_zone[0]["zoneId"], "z1");
    validate(&block_from_modules(&with_zone).expect("block")).expect("xor");

    let back = with_field(&with_zone, 0, "x", "10").expect("x strips zone");
    assert!(back[0].get("zoneId").is_none());
    assert_eq!(back[0]["x"], 10.0);
    assert_eq!(back[0]["z"], 0.0);
    validate(&block_from_modules(&back).expect("block")).expect("position");
    assert!(!placement_is_exclusive(true, true));
}

#[test]
fn an_unknown_faction_is_refused() {
    let err = with_field(&[wave()], 0, "factionKey", "navy").expect_err("faction");
    assert!(err.contains("navy"), "{err}");
}

#[test]
fn over_cap_max_alive_is_refused() {
    let err = with_field(&[wave()], 0, "maxAlive", "99").expect_err("cap");
    assert!(err.contains("32"), "{err}");
}

#[test]
fn blank_optional_interval_is_stripped() {
    let next = with_field(&[wave()], 0, "intervalSeconds", "  ").expect("blank");
    assert!(next[0].get("intervalSeconds").is_none());
    validate(&block_from_modules(&next).expect("block")).expect("valid");
}

#[test]
fn the_pickers_offer_exactly_the_schema_vocabulary() {
    assert_eq!(KINDS, ["wave", "garrison"]);
    assert_eq!(FACTION_KEYS, ["blufor", "opfor", "indfor", "civ"]);
    for k in KINDS {
        assert_ne!(kind_label(k), "Unknown kind");
    }
    for k in FACTION_KEYS {
        assert_ne!(faction_label(k), "Unknown faction");
    }
}

#[test]
fn env_patch_sets_and_clears() {
    let set: Value =
        serde_json::from_str(&env_patch(block_from_modules(&[wave()]).as_ref())).expect("json");
    assert_eq!(set["spawnModules"][0]["kind"], "wave");
    let cleared: Value = serde_json::from_str(&env_patch(None)).expect("json");
    assert_eq!(cleared, json!({"spawnModules": null}));
}

#[test]
fn the_reader_chain_names_every_hop() {
    let hops: Vec<&str> = SPAWN_MODULES_READERS.iter().map(|(h, _)| *h).collect();
    assert_eq!(hops, ["compile", "flatten", "mod", "editor"]);
    for (hop, reader) in SPAWN_MODULES_READERS {
        assert!(reader.len() > 30, "{hop}'s reader is not named: {reader}");
    }
}
