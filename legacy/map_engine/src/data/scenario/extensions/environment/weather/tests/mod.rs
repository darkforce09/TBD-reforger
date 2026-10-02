//! Role: Module boundary for mission/extensions/environment/weather/tests.
//! Position: `mission/extensions/environment/weather/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

use crate::data::scenario::compile::compile_payload;

use crate::data::scenario::extensions::{ExtensionBlocks, copy_authored_blocks, is_authored_block};

use serde_json::json;

fn three_keyframes() -> Value {
    json!({
        "keyframes": [
            {"atMinutes": 0, "weatherPreset": "clear"},
            {"atMinutes": 15, "weatherPreset": "overcast", "windDirDeg": 90.0},
            {"atMinutes": 40, "weatherPreset": "heavy_rain", "fog": 0.4}
        ]
    })
}

fn compile_env_with_timeline(timeline: &Value) -> Value {
    compile_payload(
        &json!({
            "meta": {
                "terrain": "everon",
                "environment": { "weather": "clear", "weatherTimeline": timeline }
            }
        })
        .to_string(),
        "{}",
        false,
    )
}

mod cases_1;
