//! **Role:** Module boundary for `mission_model::objectives::win_conditions::tests`.
//! **Position:** `mission_model::objectives::win_conditions::tests` in the `mission_model` crate.
//! **Signals & state:** explicit data inputs; no UI or graphics state.
//! **Invariants:** preserve authored order, numeric precision, and wire representations.

use super::*;

use serde_json::json;

fn vip() -> Value {
    json!({"mode": "vip", "endOn": ["faction_eliminated"], "vipSlotId": "s-12"})
}

mod cases_1;
