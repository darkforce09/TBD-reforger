//! The floating-point precision a flight is computed in.
//!
//! **Role:** the arithmetic the engine step, the crossing and the flight loop are written
//! against, implemented for `f32` (the engine's single precision, the only precision
//! [`super::fly_to_height`] flies in) and `f64` (the same scheme without single-precision
//! rounding, the reference the scheme's own properties are proven on).
//!
//! **Position:** the `ballistics_model` crate's `flight_model` module; `integrator`, `crossing` and `trajectory` are
//! generic over [`FlightFloat`]; nothing outside the flight model sees it.
//!
//! **Signals & state:** none; a trait over plain numbers.
//!
//! **Invariants:**
//! - A value enters a flight through [`FlightFloat::from_f64`] (one rounding) and leaves through
//!   [`FlightFloat::to_f64`] (exact widening for `f32`).
//! - `sqrt` is IEEE correctly rounded; `sin` and `cos` come from `libm` (`sinf`/`cosf` for
//!   `f32`), so every target produces the same bits. No operation is fused.

use core::fmt::Debug;
use core::ops::{Add, AddAssign, Div, Mul, Sub, SubAssign};

/// The arithmetic of one flight precision.
pub(super) trait FlightFloat:
    Copy
    + Debug
    + PartialOrd
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + AddAssign
    + SubAssign
{
    /// Zero.
    const ZERO: Self;
    /// One.
    const ONE: Self;
    /// One half.
    const HALF: Self;

    /// `value` rounded to this precision.
    fn from_f64(value: f64) -> Self;

    /// This value in `f64`.
    fn to_f64(self) -> f64;

    /// The correctly rounded square root.
    fn square_root(self) -> Self;

    /// The sine of an angle in radians.
    fn sine(self) -> Self;

    /// The cosine of an angle in radians.
    fn cosine(self) -> Self;
}

impl FlightFloat for f32 {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;
    const HALF: Self = 0.5;

    fn from_f64(value: f64) -> Self {
        value as f32
    }

    fn to_f64(self) -> f64 {
        f64::from(self)
    }

    fn square_root(self) -> Self {
        self.sqrt()
    }

    fn sine(self) -> Self {
        libm::sinf(self)
    }

    fn cosine(self) -> Self {
        libm::cosf(self)
    }
}

impl FlightFloat for f64 {
    const ZERO: Self = 0.0;
    const ONE: Self = 1.0;
    const HALF: Self = 0.5;

    fn from_f64(value: f64) -> Self {
        value
    }

    fn to_f64(self) -> f64 {
        self
    }

    fn square_root(self) -> Self {
        self.sqrt()
    }

    fn sine(self) -> Self {
        libm::sin(self)
    }

    fn cosine(self) -> Self {
        libm::cos(self)
    }
}
