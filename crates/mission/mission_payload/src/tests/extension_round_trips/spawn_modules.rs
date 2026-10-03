//! Round trips of the spawn modules through the payload compiler.
//!
//! **Role:** proves the compiler promotes an authored `spawnModules` block out of the environment bag
//! onto the payload root and omits the key when nothing is authored; where the block rides the
//! extension carrier, the carrier reads the compiled block back.
//! **Position:** a payload test over the extension block registry of the mission model.
//! **Signals & state:** none; pure functions over JSON fixtures.
//! **Invariants:** the fixtures match the extension's own test fixtures byte for byte.

use super::*;

fn wave() -> Value {
    json!({
        "id": "sm-wave",
        "kind": "wave",
        "factionKey": "opfor",
        "groupTemplate": "{000CD338713F2B5A}Prefabs/AI/Groups/Group_Base.et",
        "x": 1200.0,
        "z": 3400.0,
        "count": 2,
        "intervalSeconds": 45.0,
        "maxAlive": 4
    })
}

fn garrison() -> Value {
    json!({
        "id": "sm-gar",
        "kind": "garrison",
        "factionKey": "blufor",
        "groupTemplate": "{000CD338713F2B5A}Prefabs/AI/Groups/Group_Base.et",
        "zoneId": "z_spawn_blufor",
        "count": 1
    })
}

fn wave_and_garrison() -> Value {
    json!([wave(), garrison()])
}

fn compile_env_with_modules(block: &Value) -> Value {
    compile_payload(
        &json!({
            "meta": {
                "terrain": "everon",
                "environment": { "weather": "clear", "spawnModules": block }
            }
        })
        .to_string(),
        "{}",
        false,
    )
}

#[test]
fn a_wave_and_garrison_mission_copies_to_the_payload_root() {
    let block = wave_and_garrison();
    let p = compile_env_with_modules(&block);
    assert_eq!(
        p["spawnModules"], block,
        "AUTHORED_BLOCKS must promote spawnModules out of the env bag: {p:#}"
    );
    assert_eq!(p["spawnModules"].as_array().expect("array").len(), 2);
    assert_eq!(p["spawnModules"][0]["kind"], "wave");
    assert_eq!(p["spawnModules"][1]["kind"], "garrison");

    let (carried, refusals) = ExtensionBlocks::from_payload(&p);
    assert!(refusals.is_empty(), "{refusals:?}");
    assert_eq!(carried.get("spawnModules"), Some(&block));
}

#[test]
fn an_unauthored_payload_still_omits_the_spawn_modules_key() {
    let p = compile_payload(
        &json!({"meta": {"terrain": "everon", "environment": {"weather": "clear"}}}).to_string(),
        "{}",
        false,
    );
    assert!(
        p.get("spawnModules").is_none(),
        "parity: no spawnModules authored ⇒ no spawnModules key: {p:#}"
    );
    let (carried, refusals) = ExtensionBlocks::from_payload(&p);
    assert!(refusals.is_empty(), "{refusals:?}");
    assert!(carried.get("spawnModules").is_none());
}
