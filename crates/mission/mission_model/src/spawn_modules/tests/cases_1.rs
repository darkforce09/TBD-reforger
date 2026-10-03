//! **Role:** Domain regression cases.
//! **Position:** `mission_model::spawn_modules::tests::cases_1` in the `mission_model` crate.
//! **Signals & state:** explicit data inputs; no UI or graphics state.
//! **Invariants:** preserve authored order, numeric precision, and wire representations.

use super::*;

#[test]
fn a_wave_and_garrison_block_parses() {
    let got = parse(&wave_and_garrison()).expect("parses");
    assert_eq!(got.len(), 2);
    assert_eq!(got[0].kind, "wave");
    assert_eq!(got[0].faction_key, "opfor");
    assert_eq!(got[0].x, Some(1200.0));
    assert_eq!(got[0].interval_seconds, Some(45.0));
    assert_eq!(got[0].max_alive, Some(4));
    assert_eq!(got[1].kind, "garrison");
    assert_eq!(
        got[1].zone_id.as_ref().map(|id| id.as_str()),
        Some("z_spawn_blufor")
    );
    assert!(got[1].x.is_none());
}

#[test]
fn both_position_and_zone_are_refused() {
    assert!(
        !placement_is_exclusive(true, true),
        "the predicate itself must refuse both"
    );
    assert!(placement_is_exclusive(true, false));
    assert!(placement_is_exclusive(false, true));
    assert!(!placement_is_exclusive(false, false));

    let err = parse(&json!([{
        "id": "sm-both",
        "kind": "garrison",
        "factionKey": "blufor",
        "groupTemplate": "Group_Base",
        "x": 1.0,
        "z": 2.0,
        "zoneId": "z1",
        "count": 1
    }]))
    .expect_err("both");
    assert!(err.to_string().contains("zoneId"), "{err}");
    assert!(err.to_string().contains("never both"), "{err}");
}

#[test]
fn neither_position_nor_zone_is_refused() {
    let err = parse(&json!([{
        "id": "sm-none",
        "kind": "wave",
        "factionKey": "opfor",
        "groupTemplate": "Group_Base",
        "count": 1
    }]))
    .expect_err("neither");
    assert!(
        err.to_string().contains("never neither") || err.to_string().contains("zoneId"),
        "{err}"
    );
}

#[test]
fn incomplete_position_is_refused() {
    let err = parse(&json!([{
        "id": "sm-x",
        "kind": "wave",
        "factionKey": "opfor",
        "groupTemplate": "Group_Base",
        "x": 1.0,
        "count": 1
    }]))
    .expect_err("x only");
    assert!(err.to_string().contains("together"), "{err}");
}

#[test]
fn an_unknown_faction_is_refused() {
    let err = parse(&json!([{
        "id": "sm-navy",
        "kind": "wave",
        "factionKey": "navy",
        "groupTemplate": "Group_Base",
        "x": 1.0,
        "z": 2.0,
        "count": 1
    }]))
    .expect_err("faction");
    assert!(err.to_string().contains("navy"), "{err}");
    assert!(err.to_string().contains("blufor"), "{err}");
}

#[test]
fn an_unknown_kind_is_refused() {
    let err = parse(&json!([{
        "id": "sm-patrol",
        "kind": "patrol",
        "factionKey": "blufor",
        "groupTemplate": "Group_Base",
        "x": 1.0,
        "z": 2.0,
        "count": 1
    }]))
    .expect_err("kind");
    assert!(err.to_string().contains("patrol"), "{err}");
    assert!(err.to_string().contains("wave"), "{err}");
}

#[test]
fn zero_and_over_cap_counts_are_refused() {
    let err = parse(&json!([{
        "id": "sm-zero",
        "kind": "wave",
        "factionKey": "opfor",
        "groupTemplate": "Group_Base",
        "x": 1.0,
        "z": 2.0,
        "count": 0
    }]))
    .expect_err("zero");
    assert!(err.to_string().contains("1..="), "{err}");

    let err = parse(&json!([{
        "id": "sm-cap",
        "kind": "wave",
        "factionKey": "opfor",
        "groupTemplate": "Group_Base",
        "x": 1.0,
        "z": 2.0,
        "count": 1,
        "maxAlive": 99
    }]))
    .expect_err("cap");
    assert!(err.to_string().contains("32"), "{err}");
}

#[test]
fn a_non_positive_interval_is_refused() {
    let err = parse(&json!([{
        "id": "sm-int",
        "kind": "wave",
        "factionKey": "opfor",
        "groupTemplate": "Group_Base",
        "x": 1.0,
        "z": 2.0,
        "count": 1,
        "intervalSeconds": 0
    }]))
    .expect_err("interval");
    assert!(err.to_string().contains("above zero"), "{err}");
}

#[test]
fn an_unknown_key_is_refused() {
    let err = parse(&json!([{
        "id": "sm-extra",
        "kind": "garrison",
        "factionKey": "blufor",
        "groupTemplate": "Group_Base",
        "zoneId": "z1",
        "count": 1,
        "behaviour": "defend"
    }]))
    .expect_err("unknown");
    assert!(err.to_string().contains("behaviour"), "{err}");
}

#[test]
fn empty_and_non_array_are_refused() {
    let err = parse(&json!([])).expect_err("empty");
    assert!(err.to_string().contains("empty"), "{err}");
    let err = parse(&json!({"id": "sm-1"})).expect_err("object");
    assert!(err.to_string().contains("array"), "{err}");
}

#[test]
fn a_duplicate_id_is_refused() {
    let err = parse(&json!([
        {
            "id": "same",
            "kind": "wave",
            "factionKey": "opfor",
            "groupTemplate": "A",
            "x": 1.0,
            "z": 2.0,
            "count": 1
        },
        {
            "id": "same",
            "kind": "garrison",
            "factionKey": "blufor",
            "groupTemplate": "B",
            "zoneId": "z1",
            "count": 1
        }
    ]))
    .expect_err("dup");
    assert!(err.to_string().contains("unique"), "{err}");
}

#[test]
fn spawn_modules_is_registered_on_the_carrier() {
    assert!(
        is_authored_block("spawnModules"),
        "T-936.6's row must be in AUTHORED_BLOCKS or the carrier never emits it"
    );
    assert!(
        !crate::authored_blocks::DOCUMENT_OWNED_BLOCKS.contains(&"spawnModules"),
        "spawnModules is optional — it rides ExtensionBlocks"
    );
    assert_eq!(KINDS, ["wave", "garrison"]);
    assert_eq!(FACTION_KEYS, ["blufor", "opfor", "indfor", "civ"]);
    assert_eq!(MAX_ALIVE, 32);
}

#[test]
fn an_unlisted_environment_key_is_not_promoted() {
    let env = json!({"weather": "clear", "notAnAuthoredBlock": []});
    let mut dst = serde_json::Map::new();
    let copied = copy_authored_blocks(&env, &mut dst);
    assert!(!copied.contains(&"notAnAuthoredBlock"), "{copied:?}");
    assert!(
        !dst.contains_key("notAnAuthoredBlock"),
        "an unlisted key stays parked: {dst:?}"
    );
}
