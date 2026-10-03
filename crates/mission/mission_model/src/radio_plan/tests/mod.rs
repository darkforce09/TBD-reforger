//! **Role:** Module boundary for `mission_model::radio_plan::tests`.
//! **Position:** `mission_model::radio_plan::tests` in the `mission_model` crate.
//! **Signals & state:** explicit data inputs; no UI or graphics state.
//! **Invariants:** preserve authored order, numeric precision, and wire representations.

use super::*;

use crate::authored_blocks::{AuthoredBlocks, DOCUMENT_OWNED_BLOCKS, is_authored_block};

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

mod cases_1;
