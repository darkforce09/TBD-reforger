//! **Role:** unit tests of [`crate::half_up_rounding::round_half_up`]: ties, negative zero, the value
//! just below a half, large integers and non-finite values.
//! **Position:** `src/tests` of `grid_rasterization`, declared by `half_up_rounding.rs`.
//! **Signals & state:** none; pure functions over literal values.
//! **Invariants:** every expected value is written out by hand, never derived from the code under
//! test.

use super::round_half_up;

#[test]
fn a_positive_tie_rounds_up() {
    assert_eq!(round_half_up(0.5), 1.0);
    assert_eq!(round_half_up(2.5), 3.0);
}

#[test]
fn a_negative_tie_rounds_toward_positive_infinity() {
    assert_eq!(round_half_up(-1.5), -1.0);
    assert_eq!(round_half_up(-2.5), -2.0);
}

#[test]
fn values_in_minus_half_to_zero_give_negative_zero() {
    for value in [-0.5, -0.25, -0.0] {
        let rounded = round_half_up(value);
        assert_eq!(rounded, 0.0);
        assert!(rounded.is_sign_negative(), "{value} rounds to {rounded}");
    }
    assert!(round_half_up(0.25).is_sign_positive());
}

#[test]
fn the_largest_value_below_a_half_rounds_down() {
    assert_eq!(round_half_up(0.499_999_999_999_999_94), 0.0);
    assert_eq!(round_half_up(-0.500_000_000_000_000_1), -1.0);
}

#[test]
fn non_ties_round_to_the_nearest_integer() {
    assert_eq!(round_half_up(1.4), 1.0);
    assert_eq!(round_half_up(1.6), 2.0);
    assert_eq!(round_half_up(-1.4), -1.0);
    assert_eq!(round_half_up(-1.6), -2.0);
}

#[test]
fn large_integers_and_non_finite_values_pass_through() {
    assert_eq!(round_half_up(1e300), 1e300);
    assert_eq!(
        round_half_up(4_503_599_627_370_497.0),
        4_503_599_627_370_497.0
    );
    assert_eq!(round_half_up(f64::INFINITY), f64::INFINITY);
    assert_eq!(round_half_up(f64::NEG_INFINITY), f64::NEG_INFINITY);
    assert!(round_half_up(f64::NAN).is_nan());
}
