//! Field reads for the vehicle payload, which is still untyped JSON.
//!
//! **Role:** one total accessor over a `serde_json::Value` row, used wherever the page reads a
//! vehicle's fields.
//! **Position:** shared by the route component, the grouped list and the dossier pane.
//! **Signals & state:** none — the function is pure.
//! **Invariants:** a missing key, a null and a value of the wrong type all read as an empty
//! string, so no caller has to handle an error case.

use serde_json::Value;

/// The string at `k`, or an empty string when the key is absent or is not a string.
pub(super) fn vstr(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or_default().into()
}
