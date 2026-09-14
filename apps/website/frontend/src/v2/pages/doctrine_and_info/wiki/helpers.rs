//! Field reads for the wiki payload, which is still untyped JSON.
//!
//! **Role:** two total accessors over a `serde_json::Value` row, used wherever the wiki reads a
//! page's fields.
//! **Position:** shared by the route component, the doctrine index and the article surface.
//! **Signals & state:** none — both functions are pure.
//! **Invariants:** a missing key, a null and a value of the wrong type all read as the empty
//! value, so no caller has to handle an error case.

use serde_json::Value;

/// The string at `k`, or an empty string when the key is absent or is not a string.
pub(super) fn vstr(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or_default().into()
}

/// The integer at `k`, or zero when the key is absent or is not an integer.
pub(super) fn vi64(v: &Value, k: &str) -> i64 {
    v.get(k).and_then(Value::as_i64).unwrap_or(0)
}
