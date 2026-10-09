//! Rounding to the nearest integer with ties toward +∞.
//!
//! **Role:** [`round_half_up`], the rounding rule of the export image rasterizers: the nearest
//! integer, a tie going to the larger one, exactly as ECMAScript's `Math.round` specifies it.
//! **Position:** called by the map raster pipeline's water and road export image lanes wherever a
//! world or pixel coordinate becomes a grid index; [`crate::disc_stamping`] needs no call because
//! its rounded values are never negative.
//! **Signals & state:** none; a pure function.
//! **Invariants:** exact for every f64: `value - value.floor()` is computed without rounding
//! error, so no value just below a half (`0.49999999999999994`) rounds up and no large odd integer
//! moves; a value in `[-0.5, 0)` and `-0.0` give `-0.0`; NaN and ±∞ pass through unchanged.

/// The nearest integer to `value`, a tie rounding toward +∞ (`round_half_up(-2.5)` is `-2.0`,
/// where [`f64::round`] gives `-3.0`).
///
/// The result is `-0.0` for every value in `[-0.5, 0)` and for `-0.0`, NaN for NaN, and ±∞ for
/// ±∞.
#[inline]
#[must_use]
pub fn round_half_up(value: f64) -> f64 {
    let floor = value.floor();
    let rounded = if value - floor >= 0.5 {
        floor + 1.0
    } else {
        floor
    };
    if rounded == 0.0 && value.is_sign_negative() {
        -0.0
    } else {
        rounded
    }
}

#[cfg(test)]
#[path = "tests/half_up_rounding_tests.rs"]
mod tests;
