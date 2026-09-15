//! Role: dimensions.
//! Position: `camera/math` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// Or one.
pub(crate) fn or_one(v: f64) -> f64 {
    if v == 0.0 || v.is_nan() { 1.0 } else { v }
}

/// Round dimension.
pub(crate) fn round_dimension(v: f64) -> f64 {
    crate::camera::math::shaping::round(v)
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
