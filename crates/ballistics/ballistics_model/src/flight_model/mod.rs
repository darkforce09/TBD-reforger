//! Point-mass shell flight with quadratic air drag and a constant wind.
//!
//! **Role:** integrates a shell from the muzzle until it descends through a target height and
//! reports the time of flight, the impact point, the apex and, on request, the sampled path.
//!
//! **Position:** the `ballistics_model` crate; the firing solver, fuze and crest computations
//! call [`fly_to_height`] with [`FlightParameters`] they derive from a ballistics catalog, a
//! [`Launch`] and a [`crate::wind::Wind`]; the dispersion calls
//! [`fly_to_height_double_precision`], the same scheme in `f64`. This module knows no catalog,
//! weapon or shell type.
//!
//! **Signals & state:** none; pure functions over plain values.
//!
//! **Invariants:**
//! - Map frame metres with the muzzle at the origin: x east, y north, z up; azimuth clockwise
//!   from north, elevation up from the horizontal, both in radians.
//! - Acceleration `a = -g·ẑ - k·|v_rel|·v_rel` with `k = air_drag / mass_kg` and
//!   `v_rel = v - wind_influence_multiplier · w`; drag is isotropic, so a shell's side air-drag
//!   scale has no term here.
//! - The game engine's own simulation step at the engine's fixed step
//!   ([`DEFAULT_INTEGRATION_STEP_S`], 1/30 s): gravity first, then drag on the air-relative
//!   velocity, `v' = u - k·|u - w|·(u - w)·Δt` with `u = v - g·Δt·ẑ`, then the position by the
//!   mean velocity, `x' = x + (v + v')/2·Δt`. The flight ends where the polyline through the
//!   step points falls through the target height, by linear interpolation along that step, as
//!   the engine places it; the apex is the highest step point.
//! - Single precision, as the engine: the state, the constants (gravity, `k` divided in `f32`,
//!   the step, the scaled air velocity, the muzzle velocity from `libm`'s `sinf`/`cosf`) and
//!   every step and crossing operation are `f32`; results widen to `f64` at this module's
//!   boundary. `f32` arithmetic without fused operations is IEEE-deterministic, so native and
//!   wasm32 builds produce the same bits.
//! - Engine fidelity, measured against the engine oracle of game build 1.8.0.13 (4,185
//!   simulation samples: seven shells, every charge, 45° to 85°, winds 0 to 10 m/s, target
//!   heights -100, 0 and +100 m, points of fall 61 m to 3.1 km): see the module README for the
//!   measured maxima, pinned by the calibration tests. A double-precision replica of the same
//!   step misses by up to 0.016 m; every other scheme scored at least 70 times worse: the best
//!   alternative step (1/50 s) misses by 1.15 m and classical Runge-Kutta at 1/30 s by
//!   2.87 m. The oracle reports time of flight as the end of the step that crosses (a multiple
//!   of 1/30 s); the crossing time here is within 0.035 s of it.
//! - The step count is bounded by `time_to_live_s`; a shell still airborne then is refused with
//!   [`FlightError::TimeToLiveExceeded`], and a shell whose descent starts below the target
//!   height is refused with [`FlightError::TargetAboveApex`].
//! - Every input is validated first; NaN, infinities, non-positive mass, speed, gravity, step or
//!   lifetime answer a typed [`FlightError`], never a panic.
//! - Transcendentals go through `libm` (`sinf`, `cosf` in the flight; `sin`, `cos` in the
//!   `f64` projection of the point of fall); `+ - × / sqrt` are IEEE, so native and WASM builds
//!   produce the same bits.

mod crossing;
mod integrator;
mod precision;
mod trajectory;

use thiserror::Error;

use self::precision::FlightFloat;
use super::wind::WindError;

/// Re-exports the flight entry point and its results.
pub use trajectory::{
    FlightOutcome, FlightSample, PathRecording, fly_to_height, fly_to_height_double_precision,
};

/// The game engine's fixed shell simulation step, 1/30 s; every production caller uses it. The
/// flight rounds it to `f32`, the engine's `1.0f32 / 30.0`.
pub const DEFAULT_INTEGRATION_STEP_S: f64 = 1.0 / 30.0;

/// The largest number of integration steps one flight may take; `time_to_live_s` divided by the
/// step must stay at or under it.
pub const MAX_INTEGRATION_STEPS: f64 = 1_000_000.0;

/// The physical constants of one shell in one world, independent of how it is fired.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlightParameters {
    /// Gravitational acceleration, metres per second squared, positive downwards.
    pub gravity_m_s2: f64,
    /// Shell mass in kilograms (the flight component's mass, not the rigid body's).
    pub mass_kg: f64,
    /// Quadratic drag coefficient; drag deceleration is `air_drag / mass_kg · |v_rel|²`.
    pub air_drag: f64,
    /// Share of the wind velocity the air-relative velocity subtracts; zero ignores wind.
    pub wind_influence_multiplier: f64,
    /// Seconds the shell exists after firing; the flight must end before it.
    pub time_to_live_s: f64,
    /// Fixed simulation step in seconds.
    pub integration_step_s: f64,
}

/// How the shell leaves the muzzle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Launch {
    /// Muzzle speed in metres per second (initial speed × charge and weapon coefficients).
    pub muzzle_speed_m_s: f64,
    /// Elevation above the horizontal, radians.
    pub elevation_rad: f64,
    /// Azimuth clockwise from north, radians.
    pub azimuth_rad: f64,
}

/// Why a flight produced no descending crossing of the target height.
#[derive(Debug, Clone, Copy, PartialEq, Error)]
pub enum FlightError {
    /// A parameter, launch value or target height is NaN, infinite or out of its domain.
    #[error("flight input `{parameter}` is invalid (got {value})")]
    InvalidInput {
        /// Field name of the offending input.
        parameter: &'static str,
        /// The value that was refused.
        value: f64,
    },
    /// The wind report is invalid.
    #[error(transparent)]
    InvalidWind(#[from] WindError),
    /// `time_to_live_s / integration_step_s` exceeds [`MAX_INTEGRATION_STEPS`].
    #[error("flight would need {step_count} integration steps, more than the bound")]
    TooManyIntegrationSteps {
        /// The requested step count.
        step_count: f64,
    },
    /// The shell starts descending below the target height, so it never reaches it.
    #[error("target height {target_height_m} m is above the trajectory apex {apex_height_m} m")]
    TargetAboveApex {
        /// Highest point the shell reached, metres above the muzzle.
        apex_height_m: f64,
        /// Requested target height, metres above the muzzle.
        target_height_m: f64,
    },
    /// The shell is still above the target height when its lifetime ends.
    #[error("shell is still airborne when its {time_to_live_s} s lifetime ends")]
    TimeToLiveExceeded {
        /// The shell lifetime that ran out, seconds.
        time_to_live_s: f64,
    },
}

impl FlightParameters {
    /// Checks every field against its domain and the step count against
    /// [`MAX_INTEGRATION_STEPS`].
    ///
    /// # Errors
    ///
    /// [`FlightError::InvalidInput`] naming the first field outside its domain, or
    /// [`FlightError::TooManyIntegrationSteps`].
    pub fn validate(&self) -> Result<(), FlightError> {
        require_positive("gravity_m_s2", self.gravity_m_s2)?;
        require_positive("mass_kg", self.mass_kg)?;
        require_non_negative("air_drag", self.air_drag)?;
        require_non_negative("wind_influence_multiplier", self.wind_influence_multiplier)?;
        require_positive("time_to_live_s", self.time_to_live_s)?;
        require_positive("integration_step_s", self.integration_step_s)?;
        let step_count = (self.time_to_live_s / self.integration_step_s).ceil();
        if step_count > MAX_INTEGRATION_STEPS {
            return Err(FlightError::TooManyIntegrationSteps { step_count });
        }
        Ok(())
    }
}

impl Launch {
    /// Checks that the muzzle speed is finite and positive and both angles are finite.
    ///
    /// # Errors
    ///
    /// [`FlightError::InvalidInput`] naming the offending field.
    pub fn validate(&self) -> Result<(), FlightError> {
        require_positive("muzzle_speed_m_s", self.muzzle_speed_m_s)?;
        require_finite("elevation_rad", self.elevation_rad)?;
        require_finite("azimuth_rad", self.azimuth_rad)
    }

    /// The muzzle velocity in the map frame, metres per second `[east, north, up]`, as the
    /// engine forms it in single precision (see [`fly_to_height`]).
    pub fn velocity_m_s(&self) -> [f64; 3] {
        self.velocity_in::<f32>().map(f64::from)
    }

    /// The muzzle velocity in the precision `F`: the speed and both angles rounded once, the
    /// sines and cosines taken by `libm` in that precision.
    fn velocity_in<F: FlightFloat>(&self) -> [F; 3] {
        let speed = F::from_f64(self.muzzle_speed_m_s);
        let elevation = F::from_f64(self.elevation_rad);
        let azimuth = F::from_f64(self.azimuth_rad);
        let horizontal = speed * elevation.cosine();
        [
            horizontal * azimuth.sine(),
            horizontal * azimuth.cosine(),
            speed * elevation.sine(),
        ]
    }
}

fn require_finite(parameter: &'static str, value: f64) -> Result<(), FlightError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(FlightError::InvalidInput { parameter, value })
    }
}

fn require_positive(parameter: &'static str, value: f64) -> Result<(), FlightError> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(FlightError::InvalidInput { parameter, value })
    }
}

fn require_non_negative(parameter: &'static str, value: f64) -> Result<(), FlightError> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(FlightError::InvalidInput { parameter, value })
    }
}

#[cfg(test)]
#[path = "tests/flight_model.rs"]
mod tests;
