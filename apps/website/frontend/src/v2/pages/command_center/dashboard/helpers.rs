//! Field reads for the parts of the dashboard payload that are still untyped JSON.
//!
//! **Role:** two total accessors over a `serde_json::Value`, used by the panels whose slice of
//! the payload has no data-transfer type yet.
//! **Position:** shared by the hero banner, the deployment card and the intelligence feed.
//! **Signals & state:** none — both functions are pure.
//! **Invariants:** a missing key, a null and a value of the wrong type are all indistinguishable
//! from an empty value, so a panel never has to handle an error case.
#![allow(dead_code)]

use serde_json::Value;

/// The string at `k`, or an empty string when the key is absent or is not a string.
pub(super) fn vstr(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or_default().into()
}

/// The boolean at `k`, or `false` when the key is absent or is not a boolean.
pub(super) fn vbool(v: &Value, k: &str) -> bool {
    v.get(k).and_then(Value::as_bool).unwrap_or(false)
}
