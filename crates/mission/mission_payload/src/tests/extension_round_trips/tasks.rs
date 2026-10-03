//! Round trips of the task hierarchy through the payload compiler.
//!
//! **Role:** proves the compiler promotes an authored `tasks` block out of the environment bag
//! onto the payload root and omits the key when nothing is authored; where the block rides the
//! extension carrier, the carrier reads the compiled block back.
//! **Position:** a payload test over the extension block registry of the mission model.
//! **Signals & state:** none; pure functions over JSON fixtures.
//! **Invariants:** the fixtures match the extension's own test fixtures byte for byte.

use super::*;

fn three_tiers() -> Value {
    json!([
        {
            "id": "t-pri",
            "title": "Seize the hill",
            "tier": "primary",
            "state": "assigned",
            "triggerId": "trg-hill",
            "markerId": "attack"
        },
        {
            "id": "t-sec",
            "title": "Find the cache",
            "tier": "secondary",
            "state": "assigned",
            "triggerId": "trg-cache"
        },
        {
            "id": "t-opt",
            "title": "Radio check",
            "tier": "optional",
            "state": "assigned",
            "description": "No trigger — stays assigned."
        }
    ])
}

fn compile_env_with_tasks(tasks: &Value) -> Value {
    compile_payload(
        &json!({
            "meta": {
                "terrain": "everon",
                "environment": { "weather": "clear", "tasks": tasks }
            }
        })
        .to_string(),
        "{}",
        false,
    )
}

#[test]
fn a_three_tier_mission_copies_to_the_payload_root() {
    let tasks = three_tiers();
    let p = compile_env_with_tasks(&tasks);
    assert_eq!(
        p["tasks"], tasks,
        "AUTHORED_BLOCKS must promote tasks out of the env bag: {p:#}"
    );
    assert_eq!(p["tasks"].as_array().expect("array").len(), 3);
    assert_eq!(p["tasks"][0]["tier"], "primary");
    assert_eq!(p["tasks"][1]["tier"], "secondary");
    assert_eq!(p["tasks"][2]["tier"], "optional");

    let (carried, refusals) = ExtensionBlocks::from_payload(&p);
    assert!(refusals.is_empty(), "{refusals:?}");
    assert_eq!(carried.get("tasks"), Some(&tasks));
}

#[test]
fn an_unauthored_payload_still_omits_the_tasks_key() {
    let p = compile_payload(
        &json!({"meta": {"terrain": "everon", "environment": {"weather": "clear"}}}).to_string(),
        "{}",
        false,
    );
    assert!(
        p.get("tasks").is_none(),
        "parity: no tasks authored ⇒ no tasks key: {p:#}"
    );
    let (carried, refusals) = ExtensionBlocks::from_payload(&p);
    assert!(refusals.is_empty(), "{refusals:?}");
    assert!(carried.get("tasks").is_none());
}
