//! Role: Module boundary for mission/extensions/tactical_graphics/tests.
//! Position: `mission/extensions/tactical_graphics/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

use crate::data::scenario::compile::compile_payload;

use crate::data::scenario::extensions::{
    AUTHORED_BLOCKS, DOCUMENT_OWNED_BLOCKS, ExtensionBlocks, copy_authored_blocks,
    is_authored_block,
};

use serde_json::json;

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

mod cases_1;
