//! T-165.8 — JS-semantics JSON writers. `JSON.stringify` prints integral f64 as integers
//! (5 not 5.0) — every number that flows into an artifact goes through `js_num` so compact
//! and pretty output byte-match the Node pipeline.

use serde_json::{Number, Value};

/// JS number semantics: integral finite f64 → JSON integer (i64 range), else the f64.
pub fn js_num(v: f64) -> Value {
    if v.is_finite() && v.fract() == 0.0 && v.abs() < 9.007_199_254_740_992e15 {
        Value::Number(Number::from(v as i64))
    } else {
        Value::Number(Number::from_f64(v).expect("finite"))
    }
}

/// `Math.round(v * 100) / 100` — the pipeline's 2-dp rounding (JS Math.round = half up
/// toward +∞ on the scaled value).
pub fn round2(v: f64) -> f64 {
    js_math_round(v * 100.0) / 100.0
}

/// JS `Math.round`: floor(x + 0.5) — ties toward +∞ (NOT Rust's round-half-away-from-zero;
/// they differ on negative ties: Math.round(-2.5) = -2, (-2.5f64).round() = -3).
pub fn js_math_round(v: f64) -> f64 {
    (v + 0.5).floor()
}

/// `((h % 360) + 360) % 360` then round2 — heading normalization.
pub fn norm_heading(h: f64) -> f64 {
    round2(((h % 360.0) + 360.0) % 360.0)
}

/// Recursively rewrite every number in a Value to JS `JSON.stringify` semantics (integral
/// f64 → integer). Rule-file JSON may author `1.0`; Node's parse+stringify normalizes it to
/// `1`, so copied subtrees must be normalized before serialization.
pub fn js_normalize(v: &mut Value) {
    match v {
        Value::Number(n) => {
            if let Some(f) = n.as_f64()
                && n.as_i64().is_none()
                && n.as_u64().is_none()
            {
                *v = js_num(f);
            }
        }
        Value::Array(a) => {
            for x in a {
                js_normalize(x);
            }
        }
        Value::Object(m) => {
            for (_, x) in m.iter_mut() {
                js_normalize(x);
            }
        }
        _ => {}
    }
}
