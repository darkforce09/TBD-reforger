//! deck.gl's scalar rules for viewport sizes and extents.
//!
//! **Role:** size coercion (`or_one`), size rounding, NaN-aware four-way minimum and maximum, and
//! clamp.
//! **Position:** private to the crate; the orthographic camera's state, unprojection and
//! controls call it.
//! **Signals & state:** none; pure functions.
//! **Invariants:** `or_one` turns 0 or NaN into 1; `js_min4` and `js_max4` answer NaN when any input
//! is NaN, as `Math.min` and `Math.max` do; sizes round with JavaScript's `Math.round`.

/// Or one.
pub(crate) fn or_one(v: f64) -> f64 {
    if v == 0.0 || v.is_nan() { 1.0 } else { v }
}

/// Round dimension.
pub(crate) fn round_dimension(v: f64) -> f64 {
    map_coordinates::rounding::round(v)
}

/// Js min4.
pub(crate) fn js_min4(a: f64, b: f64, c: f64, d: f64) -> f64 {
    let mut m = a;
    for v in [b, c, d] {
        if v.is_nan() || m.is_nan() {
            return f64::NAN;
        }
        if v < m {
            m = v;
        }
    }
    m
}

/// Js max4.
pub(crate) fn js_max4(a: f64, b: f64, c: f64, d: f64) -> f64 {
    let mut m = a;
    for v in [b, c, d] {
        if v.is_nan() || m.is_nan() {
            return f64::NAN;
        }
        if v > m {
            m = v;
        }
    }
    m
}

/// Clamp.
pub(crate) fn clamp(value: f64, min: f64, max: f64) -> f64 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}
