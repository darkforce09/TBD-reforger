//! Field reads for the parts of the dashboard payload that are still untyped JSON.
//!
//! **Role:** a total string accessor over a `serde_json::Value`, used by the panels whose slice of
//! the payload has no data-transfer type yet.
//! **Position:** shared by the hero banner and the deployment card.
//! **Signals & state:** none — the function is pure.
//! **Invariants:** a missing key, a null and a value of the wrong type are all indistinguishable
//! from an empty value, so a panel never has to handle an error case.

#[cfg(target_arch = "wasm32")]
use serde_json::Value;

/// The string at `k`, or an empty string when the key is absent or is not a string.
#[cfg(target_arch = "wasm32")]
pub(super) fn vstr(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or_default().into()
}
