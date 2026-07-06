//! Exact re-implementations of the JS numeric primitives whose semantics differ from Rust's, so
//! ports stay bit-identical (Class R).

/// `Math.round` — round half **up** (toward +∞), i.e. `floor(x + 0.5)`. Rust's `f64::round` rounds
/// half **away from zero**, which differs for negative half-integers (`Math.round(-2.5) === -2` but
/// `(-2.5f64).round() == -3.0`). Every port that mirrors a JS `Math.round` uses this.
#[inline]
#[must_use]
pub(crate) fn round(x: f64) -> f64 {
    (x + 0.5).floor()
}

#[cfg(test)]
mod tests {
    use super::round;

    #[test]
    fn matches_js_math_round() {
        assert_eq!(round(2.5), 3.0);
        assert_eq!(round(-2.5), -2.0); // JS: -2 (Rust f64::round would give -3)
        assert_eq!(round(0.5), 1.0);
        assert_eq!(round(-0.5), 0.0); // JS: -0
        assert_eq!(round(2.4), 2.0);
        assert_eq!(round(2.6), 3.0);
    }
}
