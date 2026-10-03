//! Round trips of the tactical graphics through the payload compiler.
//!
//! **Role:** proves the compiler promotes an authored `tacticalGraphics` block out of the environment bag
//! onto the payload root and omits the key when nothing is authored; where the block rides the
//! extension carrier, the carrier reads the compiled block back.
//! **Position:** a payload test over the extension block registry of the mission model.
//! **Signals & state:** none; pure functions over JSON fixtures.
//! **Invariants:** the fixtures match the extension's own test fixtures byte for byte.

use super::*;

fn phase_line() -> Value {
    json!({
        "id": "tg-phase",
        "kind": "phase_line",
        "points": [[1000.0, 2000.0], [1400.0, 2100.0]],
        "label": "PL BLUE",
        "sideKey": "blufor",
        "style": {"color": "#3388ff", "alpha": 0.8}
    })
}

fn boundary() -> Value {
    json!({
        "id": "tg-bound",
        "kind": "boundary",
        "points": [[900.0, 1800.0], [1200.0, 1900.0], [1500.0, 2400.0]]
    })
}

fn axis_of_advance() -> Value {
    json!({
        "id": "tg-axis",
        "kind": "axis_of_advance",
        "points": [[800.0, 1200.0], [1600.0, 2600.0]],
        "label": "AXIS SABRE"
    })
}

fn curved_arrow() -> Value {
    json!({
        "id": "tg-arrow",
        "kind": "curved_arrow",
        "points": [[700.0, 1100.0], [1100.0, 1500.0], [1700.0, 1400.0]],
        "style": {"brush": "solid", "color": "#ff2222", "widthM": 24.0}
    })
}

fn one_of_each() -> Value {
    json!([phase_line(), boundary(), axis_of_advance(), curved_arrow()])
}

fn compile_env_with_graphics(block: &Value) -> Value {
    compile_payload(
        &json!({
            "meta": {
                "terrain": "everon",
                "environment": { "weather": "clear", "tacticalGraphics": block }
            }
        })
        .to_string(),
        "{}",
        false,
    )
}

#[test]
fn tactical_graphics_registered_here_must_also_be_readable_by_flatten() {
    assert_eq!(
        AUTHORED_BLOCKS.len(),
        7,
        "the seven T-936 blocks; a row added without a flatten field is a silent drop"
    );
    let p = compile_env_with_graphics(&one_of_each());
    let (carried, refusals) = ExtensionBlocks::from_payload(&p);
    assert!(refusals.is_empty(), "{refusals:?}");
    assert_eq!(carried.get("tacticalGraphics"), Some(&one_of_each()));
}

#[test]
fn a_mission_with_one_of_each_kind_copies_to_the_payload_root() {
    let block = one_of_each();
    let p = compile_env_with_graphics(&block);
    assert_eq!(
        p["tacticalGraphics"], block,
        "AUTHORED_BLOCKS must promote tacticalGraphics out of the env bag: {p:#}"
    );
    assert_eq!(p["tacticalGraphics"].as_array().expect("array").len(), 4);
    assert_eq!(
        p["environment"]["tacticalGraphics"], block,
        "the bag reaches the wire unchanged — that is the reload path"
    );
}

#[test]
fn an_unauthored_payload_still_omits_the_tactical_graphics_key() {
    let p = compile_payload(
        &json!({"meta": {"terrain": "everon", "environment": {"weather": "clear"}}}).to_string(),
        "{}",
        false,
    );
    assert!(
        p.get("tacticalGraphics").is_none(),
        "parity: no graphics authored ⇒ no tacticalGraphics key: {p:#}"
    );
    let (carried, refusals) = ExtensionBlocks::from_payload(&p);
    assert!(refusals.is_empty(), "{refusals:?}");
    assert!(carried.get("tacticalGraphics").is_none());
}

#[test]
fn a_malformed_block_is_refused_at_the_carrier_with_a_readable_clause() {
    let p = compile_env_with_graphics(&json!([{
        "id": "tg-short",
        "kind": "phase_line",
        "points": [[1.0, 2.0]]
    }]));
    let (carried, refusals) = ExtensionBlocks::from_payload(&p);
    assert!(carried.get("tacticalGraphics").is_none());
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(refusals[0].0, "tacticalGraphics");
    assert!(refusals[0].1.contains("at least 2"), "{:?}", refusals[0]);
}
