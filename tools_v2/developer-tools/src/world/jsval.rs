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

/// `Math.round(v * 1000) / 1000` — 3-dp rounding for the uniform scale (T-090.12.1).
pub fn round3(v: f64) -> f64 {
    js_math_round(v * 1000.0) / 1000.0
}

/// T-090.12.1 — true when the full-transform trailers are all identity after rounding, so the
/// chunk row is written 5-wide (byte-identical to the v1 catalogue) instead of 8-wide.
#[must_use]
pub fn trailers_trivial(pitch: f64, roll: f64, scale: f64) -> bool {
    pitch == 0.0 && roll == 0.0 && scale == 1.0
}

/// One chunk wire row: `[pid, x, y, z, yaw]` or `[pid, x, y, z, yaw, pitch, roll, scale]`
/// (`map-object-instance.schema.json` chunk branch — exactly 5 or 8, never padded).
#[must_use]
#[allow(clippy::too_many_arguments)]
pub fn chunk_row_values(
    id: f64,
    x: f64,
    y: f64,
    z: f64,
    rot: f64,
    pitch: f64,
    roll: f64,
    scale: f64,
) -> Vec<Value> {
    let mut row = vec![js_num(id), js_num(x), js_num(y), js_num(z), js_num(rot)];
    if !trailers_trivial(pitch, roll, scale) {
        row.push(js_num(pitch));
        row.push(js_num(roll));
        row.push(js_num(scale));
    }
    row
}

#[cfg(test)]
mod transform_row_tests {
    use super::*;

    #[test]
    fn trivial_trailers_write_five_wide() {
        let r = chunk_row_values(9.0, 512.0, 700.25, 41.3, 90.0, 0.0, 0.0, 1.0);
        assert_eq!(r.len(), 5);
        assert_eq!(
            serde_json::to_string(&Value::Array(r)).unwrap(),
            "[9,512,700.25,41.3,90]"
        );
    }

    #[test]
    fn any_nontrivial_trailer_writes_eight_wide() {
        for (p, r, s) in [(-3.04, 0.0, 1.0), (0.0, 0.5, 1.0), (0.0, 0.0, 1.15)] {
            let row = chunk_row_values(14.0, 1.0, 2.0, 3.0, 4.0, p, r, s);
            assert_eq!(row.len(), 8, "{p} {r} {s}");
        }
        let row = chunk_row_values(14.0, 900.0, 950.0, 52.0, 47.25, -3.5, 1.25, 1.15);
        assert_eq!(
            serde_json::to_string(&Value::Array(row)).unwrap(),
            "[14,900,950,52,47.25,-3.5,1.25,1.15]"
        );
    }

    #[test]
    fn round3_is_js_math_round_at_three_places() {
        assert_eq!(round3(1.1234), 1.123);
        assert_eq!(round3(1.1235), 1.124);
        assert_eq!(round3(0.9995), 1.0);
        assert!(trailers_trivial(0.0, 0.0, round3(0.9995)));
    }

    #[test]
    fn negative_zero_angles_round_to_positive_zero() {
        // The raw export prints `-0` for a flat roll; round2 must land on +0 so the trailer is
        // trivial and the row stays 5-wide.
        assert!(trailers_trivial(round2(-0.0), round2(-0.0), 1.0));
        assert_eq!(js_num(round2(-0.0)), Value::from(0));
    }
}
