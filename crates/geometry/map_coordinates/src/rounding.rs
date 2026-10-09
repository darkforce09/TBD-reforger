//! JavaScript's rounding rule.
//!
//! **Role:** [`round`], JavaScript's `Math.round`, for every port of a JavaScript routine that
//! rounds.
//! **Position:** called by `camera_math` (viewport sizes) and the map engine's terrain elevation
//! grid and hillshade.
//! **Signals & state:** none; a pure function.
//! **Invariants:** a half rounds toward +∞ (`round(-2.5)` is `-2`), unlike `f64::round`, which
//! rounds a half away from zero.

/// `Math.round` — round half **up** (toward +∞), i.e. `floor(x + 0.5)`. Rust's `f64::round` rounds half **away from zero**, which differs for negative half-integers (`Math.round(-2.5) === -2` but `(-2.5f64).round() == -3.0`). Every port that mirrors a JS `Math.round` uses this.
#[inline]
#[must_use]
pub fn round(x: f64) -> f64 {
    (x + 0.5).floor()
}
