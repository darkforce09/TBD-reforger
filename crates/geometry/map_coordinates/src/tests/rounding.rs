//! **Role:** unit tests for [`crate::rounding::round`] against JavaScript's `Math.round`.
//! **Position:** `src/tests` of `map_coordinates`, declared by `rounding.rs`.
//! **Signals & state:** none.
//! **Invariants:** the expected values are JavaScript's answers, halves included.

use crate::rounding::round;

#[test]
fn matches_js_math_round() {
    assert_eq!(round(2.5), 3.0);
    assert_eq!(round(-2.5), -2.0);
    assert_eq!(round(0.5), 1.0);
    assert_eq!(round(-0.5), 0.0);
    assert_eq!(round(2.4), 2.0);
    assert_eq!(round(2.6), 3.0);
}
