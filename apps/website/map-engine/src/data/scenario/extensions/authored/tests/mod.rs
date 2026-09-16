//! Role: Module boundary for mission/extensions/authored/tests.
//! Position: `mission/extensions/authored/tests` in the map engine's headless mission data domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::*;

use serde_json::json;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Host {
    win_conditions: Value,
    #[serde(flatten)]
    extensions: ExtensionBlocks,
}

fn host(extensions: ExtensionBlocks) -> Value {
    let h = Host {
        win_conditions: json!({"mode": "attrition", "endOn": ["time_limit"]}),
        extensions,
    };
    serde_json::from_str(&serde_json::to_string(&h).expect("serialises")).expect("parses")
}

mod cases_1;
