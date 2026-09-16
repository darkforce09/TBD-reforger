//! Role: shaping.
//! Position: `camera/math` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// `Math.round` — round half **up** (toward +∞), i.e. `floor(x + 0.5)`. Rust's `f64::round` rounds half **away from zero**, which differs for negative half-integers (`Math.round(-2.5) === -2` but `(-2.5f64).round() == -3.0`). Every port that mirrors a JS `Math.round` uses this.
#[inline]
#[must_use]
pub(crate) fn round(x: f64) -> f64 {
    (x + 0.5).floor()
}

#[cfg(test)]
#[path = "tests/shaping_tests.rs"]
mod tests;
