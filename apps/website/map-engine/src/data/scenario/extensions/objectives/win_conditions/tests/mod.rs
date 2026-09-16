//! Role: Module boundary for mission/extensions/objectives/win_conditions/tests.
//! Position: `mission/extensions/objectives/win_conditions/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

use serde_json::json;

fn vip() -> Value {
    json!({"mode": "vip", "endOn": ["faction_eliminated"], "vipSlotId": "s-12"})
}

mod cases_1;
