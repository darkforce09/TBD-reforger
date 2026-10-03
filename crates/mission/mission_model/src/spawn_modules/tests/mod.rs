//! **Role:** Module boundary for `mission_model::spawn_modules::tests`.
//! **Position:** `mission_model::spawn_modules::tests` in the `mission_model` crate.
//! **Signals & state:** explicit data inputs; no UI or graphics state.
//! **Invariants:** preserve authored order, numeric precision, and wire representations.

use super::*;

use crate::authored_blocks::{copy_authored_blocks, is_authored_block};

use serde_json::json;

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

mod cases_1;
