//! **Role:** Module boundary for `mission_model::environment::weather::tests`.
//! **Position:** `mission_model::environment::weather::tests` in the `mission_model` crate.
//! **Signals & state:** explicit data inputs; no UI or graphics state.
//! **Invariants:** preserve authored order, numeric precision, and wire representations.

use super::*;

use crate::authored_blocks::copy_authored_blocks;

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

mod cases_1;
