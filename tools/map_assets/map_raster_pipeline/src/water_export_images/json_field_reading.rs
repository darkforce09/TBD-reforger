//! Reading the loosely typed fields of the Workbench water export's JSON documents.
//!
//! **Role:** the two ways the water export lane reads a field: as *given* ([`is_given`],
//! [`first_given`], [`given_number`]), where `null`, `false`, `0` and `""` count as absent and
//! the reader falls back to the next source, and as a *JSON number* ([`json_number`]), where every
//! number counts, `0` and negatives included; plus the coordinate of a point array
//! ([`coordinate`]).
//! **Position:** called by the raster geometry (the meta's sizes) and by the polygon and river
//! rasterizers (depths, widths, rings and spline points).
//! **Signals & state:** none; pure functions over borrowed [`serde_json::Value`]s.
//! **Invariants:** an array or an object counts as given even when empty; a field read from a
//! value that is not an object is absent; a value that is given but is not a number is absent to
//! [`given_number`], so a width or a size is always a number or the fallback; a coordinate that is
//! missing or not a number reads as NaN, which no comparison passes and no grid index accepts.

use serde_json::Value;

/// Whether `value` counts as given: everything but `null`, `false`, `0` and `""`; an array or an
/// object counts even when it is empty.
pub(super) fn is_given(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(flag) => *flag,
        Value::Number(number) => number.as_f64().is_some_and(|number| number != 0.0),
        Value::String(text) => !text.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

/// The value of the first of `keys` that `object` gives (see [`is_given`]), or `None` when it
/// gives none of them or is not an object.
pub(super) fn first_given<'document>(
    object: &'document Value,
    keys: &[&str],
) -> Option<&'document Value> {
    keys.iter()
        .filter_map(|key| object.get(*key))
        .find(|value| is_given(value))
}

/// The number `value` holds when it is given (see [`is_given`]) and is a JSON number: `None` for
/// a missing value, `null`, `0` and every value that is not a number.
pub(super) fn given_number(value: Option<&Value>) -> Option<f64> {
    value
        .and_then(Value::as_f64)
        .filter(|number| *number != 0.0)
}

/// The number `value` holds when it is a JSON number, `0` and negatives included; `None` for a
/// missing value and every value that is not a number.
pub(super) fn json_number(value: Option<&Value>) -> Option<f64> {
    value.and_then(Value::as_f64)
}

/// Element `index` of the point array `point` as a number, or NaN when `point` is not an array,
/// is shorter, or holds something other than a number there.
pub(super) fn coordinate(point: Option<&Value>, index: usize) -> f64 {
    point
        .and_then(|point| point.get(index))
        .and_then(Value::as_f64)
        .unwrap_or(f64::NAN)
}

#[cfg(test)]
#[path = "../tests/water_export_images_json_field_tests.rs"]
mod tests;
