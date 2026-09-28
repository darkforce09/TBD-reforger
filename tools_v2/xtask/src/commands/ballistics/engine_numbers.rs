//! Engine numbers written as the shortest decimal of the 32-bit float the engine holds.
//!
//! **Role:** Rewrites every number read from the gameplay export or the oracle output that is
//! exactly a 32-bit float into the shortest decimal that parses back to that same float, so the
//! committed documents say `1.531` where the export says `1.531000018119812`.
//!
//! **Position:** Applied by the trim readers to each value as it leaves the export or the oracle
//! files, before any comparison or serialization; the catalog and the calibration bundle only ever
//! see normalized numbers, so a coefficient copied into both documents is one identical `f64`.
//!
//! **Signals & state:** none; pure functions.
//!
//! **Invariants:** Lossless for the engine: the result converts back to the exact same `f32`. A
//! number that is not an exact `f32` (an integer beyond 2^24, or a value computed in 64 bits) is
//! returned unchanged. Integers stored as JSON integers stay integers.
use serde_json::{Number, Value};

/// The shortest decimal of `value`'s `f32` when `value` is exactly an `f32`; `value` otherwise.
pub(crate) fn engine_number(value: f64) -> f64 {
    let single = value as f32;
    if !value.is_finite() || f64::from(single) != value {
        return value;
    }
    format!("{single}").parse::<f64>().unwrap_or(value)
}

/// `value` with every floating-point number inside it passed through [`engine_number`].
pub(crate) fn normalize_engine_numbers(value: &Value) -> Value {
    match value {
        Value::Number(number) if number.is_f64() => number
            .as_f64()
            .and_then(|float| Number::from_f64(engine_number(float)))
            .map_or_else(|| value.clone(), Value::Number),
        Value::Array(items) => Value::Array(items.iter().map(normalize_engine_numbers).collect()),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(key, item)| (key.clone(), normalize_engine_numbers(item)))
                .collect(),
        ),
        other => other.clone(),
    }
}
