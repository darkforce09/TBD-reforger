//! Angle units of weapon sights: degrees, radians and mils, and azimuth normalisation.
//!
//! **Role:** converts between radians, degrees and a weapon's mils, and folds any azimuth into
//! one turn.
//!
//! **Position:** the `ballistics_model` crate; the firing solver reports azimuths and elevations
//! through a weapon's [`MilsConvention`] (read from
//! [`crate::catalog::WeaponSystem::mils_per_circle`]), and
//! calibration measures its tolerances in [`MilsConvention::MILS_6400`].
//!
//! **Signals & state:** none; pure value type and functions.
//!
//! **Invariants:**
//! - A convention divides the full circle into `mils_per_circle` equal mils, never zero:
//!   `mils = radians × mils_per_circle / 2π = degrees × mils_per_circle / 360`.
//! - Normalised azimuths lie in `[0, one turn)`: never negative, never negative zero, never the
//!   full turn itself; NaN stays NaN and infinities become NaN.
//! - Only `+ - × /` and the IEEE remainder are used, so native and WASM builds produce the same
//!   bits.

use core::f64::consts::TAU;

use thiserror::Error;

/// Degrees in a full circle.
pub const DEGREES_PER_CIRCLE: f64 = 360.0;

/// How a weapon sight divides the full circle into mils.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MilsConvention {
    mils_per_circle: u32,
}

/// Why a mils convention cannot be built.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum AngularUnitsError {
    /// A circle of zero mils has no angle unit.
    #[error("a mils convention needs at least one mil per circle")]
    ZeroMilsPerCircle,
}

impl MilsConvention {
    /// 6,400 mils to the circle: the M252 sight and the calibration tolerance unit.
    pub const MILS_6400: MilsConvention = MilsConvention {
        mils_per_circle: 6400,
    };

    /// A convention of `mils_per_circle` mils to the full circle.
    ///
    /// # Errors
    ///
    /// [`AngularUnitsError::ZeroMilsPerCircle`] when `mils_per_circle` is zero.
    pub fn new(mils_per_circle: u32) -> Result<Self, AngularUnitsError> {
        if mils_per_circle == 0 {
            return Err(AngularUnitsError::ZeroMilsPerCircle);
        }
        Ok(Self { mils_per_circle })
    }

    /// Mils in the full circle.
    pub fn mils_per_circle(self) -> u32 {
        self.mils_per_circle
    }

    fn circle(self) -> f64 {
        f64::from(self.mils_per_circle)
    }

    /// Converts an angle in radians to mils.
    pub fn radians_to_mils(self, radians: f64) -> f64 {
        radians * self.circle() / TAU
    }

    /// Converts an angle in mils to radians.
    pub fn mils_to_radians(self, mils: f64) -> f64 {
        mils * TAU / self.circle()
    }

    /// Converts an angle in degrees to mils.
    pub fn degrees_to_mils(self, degrees: f64) -> f64 {
        degrees * self.circle() / DEGREES_PER_CIRCLE
    }

    /// Converts an angle in mils to degrees.
    pub fn mils_to_degrees(self, mils: f64) -> f64 {
        mils * DEGREES_PER_CIRCLE / self.circle()
    }

    /// Folds an azimuth in mils into `[0, mils_per_circle)`.
    pub fn normalise_azimuth_mils(self, mils: f64) -> f64 {
        wrap_into_turn(mils, self.circle())
    }

    /// Converts an azimuth in radians, clockwise from north, to mils in `[0, mils_per_circle)`.
    pub fn azimuth_radians_to_mils(self, radians: f64) -> f64 {
        self.normalise_azimuth_mils(self.radians_to_mils(radians))
    }
}

/// Converts degrees to radians.
pub fn degrees_to_radians(degrees: f64) -> f64 {
    degrees * TAU / DEGREES_PER_CIRCLE
}

/// Converts radians to degrees.
pub fn radians_to_degrees(radians: f64) -> f64 {
    radians * DEGREES_PER_CIRCLE / TAU
}

/// Folds an azimuth in degrees into `[0, 360)`.
pub fn normalise_azimuth_degrees(degrees: f64) -> f64 {
    wrap_into_turn(degrees, DEGREES_PER_CIRCLE)
}

/// Folds an azimuth in radians into `[0, 2π)`.
pub fn normalise_azimuth_radians(radians: f64) -> f64 {
    wrap_into_turn(radians, TAU)
}

/// `value` modulo `turn` in `[0, turn)`. The Euclidean remainder can round a tiny negative input
/// up to `turn` itself and keeps the sign of a negative zero; both fold to `+0`.
fn wrap_into_turn(value: f64, turn: f64) -> f64 {
    let remainder = value.rem_euclid(turn);
    if remainder >= turn || remainder == 0.0 {
        0.0
    } else {
        remainder
    }
}

#[cfg(test)]
#[path = "tests/angular_units.rs"]
mod tests;
