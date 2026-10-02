//! Role: Module boundary for mission/extensions/radio/tests.
//! Position: `mission/extensions/radio/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

use crate::data::scenario::compile::compile_payload;

use crate::data::scenario::extensions::{
    AuthoredBlocks, DOCUMENT_OWNED_BLOCKS, copy_authored_blocks, is_authored_block,
};

use serde_json::json;

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

mod cases_1;
