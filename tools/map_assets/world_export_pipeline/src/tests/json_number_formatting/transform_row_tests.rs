//! Chunk row width and number rounding: a row with trivial pitch/roll/scale trailers writes five
//! values, any non-trivial trailer widens it to eight, and rounding follows JavaScript's
//! half-up `Math.round` at the declared number of places.

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
