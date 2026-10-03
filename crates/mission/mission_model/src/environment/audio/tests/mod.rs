//! **Role:** Module boundary for `mission_model::environment::audio::tests`.
//! **Position:** `mission_model::environment::audio::tests` in the `mission_model` crate.
//! **Signals & state:** explicit data inputs; no UI or graphics state.
//! **Invariants:** preserve authored order, numeric precision, and wire representations.

use super::*;

use crate::authored_blocks::{copy_authored_blocks, is_authored_block};

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

mod cases_1;
