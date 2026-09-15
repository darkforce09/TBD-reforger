//! Role: Module boundary for mission/extensions/objectives/tasks/tests.
//! Position: `mission/extensions/objectives/tasks/tests` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

use crate::mission::compile::compile_payload;

use crate::mission::extensions::{ExtensionBlocks, copy_authored_blocks, is_authored_block};

use serde_json::json;

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

fn timed(start: i64, window: i64) -> Value {
    json!([{
        "id": "t1",
        "title": "A",
        "tier": "primary",
        "state": "assigned",
        "schedule": {"startAfterS": start, "windowS": window}
    }])
}

mod cases_1;
