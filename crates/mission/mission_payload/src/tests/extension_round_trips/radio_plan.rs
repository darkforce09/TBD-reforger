//! Round trips of the radio plan through the payload compiler.
//!
//! **Role:** proves the compiler promotes an authored `radioPlan` block out of the environment bag
//! onto the payload root and omits the key when nothing is authored; where the block rides the
//! extension carrier, the carrier reads the compiled block back.
//! **Position:** a payload test over the extension block registry of the mission model.
//! **Signals & state:** none; pure functions over JSON fixtures.
//! **Invariants:** the fixtures match the extension's own test fixtures byte for byte.

use super::*;

fn one_net() -> Value {
    json!({
        "nets": [{
            "id": "net:blufor_cmd",
            "label": "Command",
            "freqMHz": 30.0,
            "faction": "blufor",
            "range": "long"
        }]
    })
}

fn compile_env_with_plan(plan: &Value) -> Value {
    compile_payload(
        &json!({
            "meta": {
                "terrain": "everon",
                "environment": { "weather": "clear", "radioPlan": plan }
            }
        })
        .to_string(),
        "{}",
        false,
    )
}

#[test]
fn a_mission_copies_radio_plan_to_the_payload_root() {
    let plan = one_net();
    let p = compile_env_with_plan(&plan);
    assert_eq!(
        p["radioPlan"], plan,
        "AUTHORED_BLOCKS must promote radioPlan out of the env bag: {p:#}"
    );
    let mut dst = serde_json::Map::new();
    let copied = copy_authored_blocks(&json!({"radioPlan": plan}), &mut dst);
    assert_eq!(copied, ["radioPlan"]);
}

#[test]
fn an_unauthored_payload_still_omits_the_radio_plan_key() {
    let p = compile_payload(
        &json!({"meta": {"terrain": "everon", "environment": {"weather": "clear"}}}).to_string(),
        "{}",
        false,
    );
    assert!(
        p.get("radioPlan").is_none(),
        "parity: no radioPlan authored ⇒ no radioPlan key on the payload: {p:#}"
    );
}
