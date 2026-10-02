//! Role: Module boundary for mission/extensions/environment/audio/tests.
//! Position: `mission/extensions/environment/audio/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

use crate::data::scenario::compile::compile_payload;

use crate::data::scenario::extensions::{ExtensionBlocks, copy_authored_blocks, is_authored_block};

use serde_json::json;

fn two_and_one() -> Value {
    json!({
        "emitters": [
            {
                "id": "ae-gen",
                "x": 100.0,
                "z": 200.0,
                "sound": "SOUND_HINT",
                "radiusM": 25.0,
                "loop": true
            },
            {
                "id": "ae-shot",
                "x": 300.0,
                "z": 400.0,
                "y": 12.5,
                "sound": "SOUND_FEVER",
                "radiusM": 8.0,
                "loop": false,
                "triggerId": "tr-door"
            }
        ],
        "musicCues": [
            {
                "id": "mc-start",
                "event": "mission_start",
                "track": "SOUND_HINT"
            }
        ]
    })
}

fn compile_env_with_audio(block: &Value) -> Value {
    compile_payload(
        &json!({
            "meta": {
                "terrain": "everon",
                "environment": { "weather": "clear", "audio": block }
            }
        })
        .to_string(),
        "{}",
        false,
    )
}

mod cases_1;
