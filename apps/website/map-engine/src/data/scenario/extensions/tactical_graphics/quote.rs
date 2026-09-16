//! Role: quote.
//! Position: `mission/extensions/tactical_graphics` in the headless mission domain.
//! Signals & state: explicit data inputs; no UI or graphics state.
//! Invariants: preserve authored order, numeric precision, and wire representations.

use super::Value;

/// Quote using the supplied domain data.
pub(super) fn quote(s: &str) -> String {
    format!("{s:?}")
}

/// Type name using the supplied domain data.
pub(super) fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}
