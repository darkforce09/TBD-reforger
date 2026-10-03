//! Impact dispersion of one solved charge: the probable-error ellipse the game's dispersion
//! parameters imply.
//!
//! **Role:** turns a launcher's angular dispersion disc and a shell's muzzle speed spread into
//! the spread of the impact point at the target height: range and deflection probable errors,
//! the 50 % ellipse with its orientation, and the shell's range-card standard dispersion beside
//! them. This is a documented interpretation of the game's parameters, not verified in-engine:
//! no engine call returns a dispersion, so [`ImpactDispersion::verified_in_engine`] is always
//! `false`.
//!
//! **Position:** the `ballistics_solver` crate; the battery computes one per gun from the gun's
//! solved [`crate::ChargeSolution`] (its wind-corrected aim
//! azimuth and elevation) through [`charge_dispersion`]; [`impact_dispersion`] takes explicit
//! values. Every impact comes from
//! [`ballistics_model::flight_model::fly_to_height_double_precision`]: the
//! engine's step, step size and constants in `f64`.
//!
//! **Signals & state:** none; pure functions over plain values and a borrowed catalog.
//!
//! **Invariants:**
//! - Angular spread: a uniform disc of diameter `dispersion_diameter_m × dispersion_multiplier`
//!   at `dispersion_range_m`. With disc radius `r`, the per-axis angular σ is
//!   `(r / dispersion_range_m) / 2`. The two axes are perpendicular to the bore: pitch (an
//!   elevation change) and transverse (a rotation of the launch direction about the bore normal),
//!   which lays the azimuth off by `δ / cos θ` at elevation `θ`.
//! - Speed spread: uniform `±init_speed_variation` metres per second on the muzzle speed as
//!   fired, σ = `a / √3`. The export declares no unit and the engine API documentation carries no
//!   entry for the attribute; metres per second is the working reading because a 7.62×39 mm
//!   cartridge in the same export (InitSpeed 732, InitSpeedVariation 7) reads as about ±1 % in
//!   metres per second and as ±51 m/s in percent. The unit stays part of the interpretation.
//! - Propagation: central finite differences of the horizontal impact point at the target
//!   height ([`FiniteDifferenceSteps::DEFAULT`]: 1e-5 rad for angles, 1e-3·v0 for speed) give
//!   the Jacobian `J`; the impact covariance is `J·Σ·Jᵀ`, `Σ` the diagonal of the three
//!   variances. The dispersion is a derivative of the point of fall, so it flies the engine's
//!   scheme in double precision: the single-precision flight is a step function of the launch
//!   angle at the scale of one `f32` rounding (about a millimetre at the point of fall), which a
//!   derivative would amplify, while the `f64` flight is smooth at the 1e-5 rad step and lands
//!   within 0.02 m of the `f32` flight.
//! - The range axis is the bearing from the gun to the nominal impact (the gun-to-target line
//!   once the aim is wind-corrected); deflection is positive to its right.
//! - Probable error = [`PROBABLE_ERROR_PER_SIGMA`] × σ; the 50 % ellipse semi-axes are
//!   [`HALF_PROBABILITY_ELLIPSE_PER_SIGMA`] × the principal σ; its orientation is the bearing of
//!   the semi-major axis in `[0°, 180°)`.
//! - No input panics: a NaN, infinite or out-of-domain value is a [`DispersionError`].
//!
//! @contract fire-mission.schema.json#/definitions/Dispersion

use core::f64::consts::FRAC_PI_2;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{ChargeSolution, FireSolutionRequest, SolutionRefusal};
use ballistics_model::angular_units::{
    degrees_to_radians, normalise_azimuth_degrees, normalise_azimuth_radians, radians_to_degrees,
};
use ballistics_model::catalog::{BallisticsCatalog, CatalogLookupError, Shell, WeaponSystem};
use ballistics_model::flight_model::{
    FlightError, FlightParameters, Launch, PathRecording, fly_to_height_double_precision,
};
use ballistics_model::wind::Wind;

/// Probable error per standard deviation of a normal distribution: half the mass lies within
/// `±0.6745 σ`.
pub const PROBABLE_ERROR_PER_SIGMA: f64 = 0.6745;

/// Semi-axis of the 50 % ellipse of a bivariate normal per principal σ: `√(2 ln 2)`.
pub const HALF_PROBABILITY_ELLIPSE_PER_SIGMA: f64 = 1.177_410_022_515_474_7;

/// Degrees in a half turn: an ellipse axis has no direction, so its bearing folds into it.
const HALF_TURN_DEG: f64 = 180.0;

/// Nominal impacts nearer the gun than this, metres, take the aim azimuth as the range axis.
const MIN_REFERENCE_DISTANCE_M: f64 = 1e-6;

/// Step sizes of the central finite differences.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FiniteDifferenceSteps {
    /// Step on the elevation and the azimuth, radians.
    pub angle_rad: f64,
    /// Step on the muzzle speed as a fraction of it.
    pub speed_fraction: f64,
}

impl FiniteDifferenceSteps {
    /// The production steps: 1e-5 rad and 1e-3 of the muzzle speed, on the double-precision
    /// flight (see the module invariants).
    pub const DEFAULT: FiniteDifferenceSteps = FiniteDifferenceSteps {
        angle_rad: 1e-5,
        speed_fraction: 1e-3,
    };

    fn validate(&self) -> Result<(), DispersionError> {
        require(
            "angle_rad",
            self.angle_rad,
            self.angle_rad.is_finite() && self.angle_rad > 0.0,
        )?;
        require(
            "speed_fraction",
            self.speed_fraction,
            self.speed_fraction.is_finite()
                && self.speed_fraction > 0.0
                && self.speed_fraction < 1.0,
        )
    }
}

/// The game's dispersion parameters for one launcher firing one shell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DispersionParameters {
    /// Diameter of the launcher's uniform dispersion disc, metres.
    pub dispersion_diameter_m: f64,
    /// The shell's factor on the disc diameter.
    pub dispersion_multiplier: f64,
    /// Range at which the disc diameter is measured, metres.
    pub dispersion_range_m: f64,
    /// Half-width of the uniform muzzle speed spread, metres per second.
    pub init_speed_variation_m_s: f64,
    /// The shell's range-card standard dispersion, metres; reported beside the ellipse.
    pub standard_dispersion_m: f64,
}

impl DispersionParameters {
    /// The parameters of `shell` fired by `weapon`, read from a ballistics catalog.
    pub fn from_catalog(weapon: &WeaponSystem, shell: &Shell) -> Self {
        Self {
            dispersion_diameter_m: weapon.dispersion_diameter_m,
            dispersion_multiplier: shell.dispersion_multiplier,
            dispersion_range_m: weapon.dispersion_range_m,
            init_speed_variation_m_s: shell.init_speed_variation,
            standard_dispersion_m: shell.standard_dispersion_m,
        }
    }

    /// Standard deviation of each angular axis, radians: `(r / dispersion_range_m) / 2` with
    /// `r` the radius of the scaled disc.
    pub fn angular_sigma_rad(&self) -> f64 {
        let radius_m = self.dispersion_diameter_m * self.dispersion_multiplier / 2.0;
        radius_m / self.dispersion_range_m / 2.0
    }

    /// Standard deviation of the muzzle speed, metres per second: `a / √3` for a uniform
    /// spread of half-width `a`.
    pub fn speed_sigma_m_s(&self) -> f64 {
        self.init_speed_variation_m_s / 3.0_f64.sqrt()
    }

    fn validate(&self) -> Result<(), DispersionError> {
        for (parameter, value) in [
            ("dispersion_diameter_m", self.dispersion_diameter_m),
            ("dispersion_multiplier", self.dispersion_multiplier),
            ("init_speed_variation_m_s", self.init_speed_variation_m_s),
            ("standard_dispersion_m", self.standard_dispersion_m),
        ] {
            require(parameter, value, value.is_finite() && value >= 0.0)?;
        }
        require(
            "dispersion_range_m",
            self.dispersion_range_m,
            self.dispersion_range_m.is_finite() && self.dispersion_range_m > 0.0,
        )
    }
}

/// One solved charge fired at its aim, with the spread to propagate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DispersionProblem {
    /// The shell's physical constants.
    pub flight_parameters: FlightParameters,
    /// Nominal muzzle speed, metres per second.
    pub muzzle_speed_m_s: f64,
    /// Laid elevation, radians, strictly inside `(-π/2, π/2)`.
    pub elevation_rad: f64,
    /// Laid (wind-corrected) azimuth clockwise from north, radians.
    pub aim_azimuth_rad: f64,
    /// Target height minus gun height, metres.
    pub height_difference_m: f64,
    /// The surface wind.
    pub wind: Wind,
    /// The game's dispersion parameters.
    pub spread: DispersionParameters,
}

/// The impact spread of one gun's charge.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImpactDispersion {
    /// Probable error along the line of fire, metres.
    pub range_probable_error_m: f64,
    /// Probable error across the line of fire, metres.
    pub deflection_probable_error_m: f64,
    /// Semi-major axis of the 50 % ellipse, metres.
    pub ellipse_semi_major_m: f64,
    /// Semi-minor axis of the 50 % ellipse, metres.
    pub ellipse_semi_minor_m: f64,
    /// Bearing of the semi-major axis clockwise from north, degrees in `[0, 180)`.
    pub ellipse_orientation_deg: f64,
    /// The shell's range-card standard dispersion, metres.
    pub standard_dispersion_m: f64,
    /// Always `false`: the ellipse is an interpretation of the game's parameters. A read of
    /// `true` fails, because the contract's `verified_in_engine` is the constant `false`.
    #[serde(deserialize_with = "deserialize_not_verified_in_engine")]
    pub verified_in_engine: bool,
}

/// The reported dispersion with the covariance it comes from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DispersionAnalysis {
    /// The reported values.
    pub dispersion: ImpactDispersion,
    /// `[east, north]` metres of the nominal impact from the gun.
    pub nominal_impact_m: [f64; 2],
    /// Bearing of the range axis clockwise from north, radians in `[0, 2π)`.
    pub range_axis_bearing_rad: f64,
    /// Variance along the range axis, square metres.
    pub range_variance_m2: f64,
    /// Variance along the deflection axis, square metres.
    pub deflection_variance_m2: f64,
    /// Range-deflection covariance, square metres.
    pub range_deflection_covariance_m2: f64,
}

/// Why a dispersion cannot be computed.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum DispersionError {
    /// The catalog has no such weapon, shell, pairing or charge.
    #[error(transparent)]
    Lookup(#[from] CatalogLookupError),
    /// A value is NaN, infinite or out of its domain.
    #[error("dispersion input `{parameter}` is invalid (got {value})")]
    InvalidInput {
        /// Name of the offending input.
        parameter: &'static str,
        /// The value that was refused.
        value: f64,
    },
    /// The charge row carries no elevation and aim azimuth.
    #[error("the charge of {rings} rings has no solution to disperse ({refusal:?})")]
    ChargeDidNotSolve {
        /// Rings of the charge.
        rings: u32,
        /// The charge's refusal.
        refusal: Option<SolutionRefusal>,
    },
    /// The nominal or a perturbed flight has no descending crossing of the target height.
    #[error("dispersion flight failed: {0}")]
    Flight(#[from] FlightError),
}

/// The dispersion of `charge`, one solved row of the solution of `request`, laid at its
/// wind-corrected aim azimuth and elevation.
///
/// # Errors
///
/// [`DispersionError::Lookup`] when the catalog lacks the weapon, shell or charge,
/// [`DispersionError::ChargeDidNotSolve`] for a refused row, then any error of
/// [`impact_dispersion`].
pub fn charge_dispersion(
    catalog: &BallisticsCatalog,
    request: &FireSolutionRequest<'_>,
    charge: &ChargeSolution,
) -> Result<DispersionAnalysis, DispersionError> {
    let firing = catalog.resolve_firing(request.weapon_id, request.shell_id, charge.rings)?;
    let (Some(elevation_deg), Some(aim_azimuth_deg)) =
        (charge.elevation_deg, charge.aim_azimuth_deg)
    else {
        return Err(DispersionError::ChargeDidNotSolve {
            rings: charge.rings,
            refusal: charge.refusal,
        });
    };
    impact_dispersion(&DispersionProblem {
        flight_parameters: firing.flight_parameters,
        muzzle_speed_m_s: firing.muzzle_speed_m_s,
        elevation_rad: degrees_to_radians(elevation_deg),
        aim_azimuth_rad: degrees_to_radians(aim_azimuth_deg),
        height_difference_m: request.target.height_m - request.gun.height_m,
        wind: request.wind,
        spread: DispersionParameters::from_catalog(firing.weapon, firing.shell),
    })
}

/// The dispersion of `problem` with the production [`FiniteDifferenceSteps::DEFAULT`].
///
/// # Errors
///
/// See [`impact_dispersion_with_steps`].
pub fn impact_dispersion(
    problem: &DispersionProblem,
) -> Result<DispersionAnalysis, DispersionError> {
    impact_dispersion_with_steps(problem, FiniteDifferenceSteps::DEFAULT)
}

/// The dispersion of `problem` with explicit finite-difference `steps`.
///
/// # Errors
///
/// [`DispersionError::InvalidInput`] for a spread, step, elevation or height outside its
/// domain; [`DispersionError::Flight`] when the nominal or a perturbed flight fails.
pub fn impact_dispersion_with_steps(
    problem: &DispersionProblem,
    steps: FiniteDifferenceSteps,
) -> Result<DispersionAnalysis, DispersionError> {
    problem.spread.validate()?;
    steps.validate()?;
    let elevation_rad = problem.elevation_rad;
    require(
        "elevation_rad",
        elevation_rad,
        elevation_rad.is_finite() && elevation_rad.abs() < FRAC_PI_2,
    )?;
    require(
        "height_difference_m",
        problem.height_difference_m,
        problem.height_difference_m.is_finite(),
    )?;
    let azimuth_rad = problem.aim_azimuth_rad;
    let speed_m_s = problem.muzzle_speed_m_s;
    let impact = |elevation_rad: f64, azimuth_rad: f64, muzzle_speed_m_s: f64| {
        horizontal_impact_m(problem, elevation_rad, azimuth_rad, muzzle_speed_m_s)
    };

    let nominal_impact_m = impact(elevation_rad, azimuth_rad, speed_m_s)?;
    let angle_step_rad = steps.angle_rad;
    let speed_step_m_s = steps.speed_fraction * speed_m_s;
    let per_pitch = central_difference(
        impact(elevation_rad + angle_step_rad, azimuth_rad, speed_m_s)?,
        impact(elevation_rad - angle_step_rad, azimuth_rad, speed_m_s)?,
        angle_step_rad,
    );
    let per_azimuth = central_difference(
        impact(elevation_rad, azimuth_rad + angle_step_rad, speed_m_s)?,
        impact(elevation_rad, azimuth_rad - angle_step_rad, speed_m_s)?,
        angle_step_rad,
    );
    let per_speed = central_difference(
        impact(elevation_rad, azimuth_rad, speed_m_s + speed_step_m_s)?,
        impact(elevation_rad, azimuth_rad, speed_m_s - speed_step_m_s)?,
        speed_step_m_s,
    );
    let transverse_per_azimuth = 1.0 / libm::cos(elevation_rad);
    let per_transverse = [
        per_azimuth[0] * transverse_per_azimuth,
        per_azimuth[1] * transverse_per_azimuth,
    ];

    let range_axis_bearing_rad =
        if libm::hypot(nominal_impact_m[0], nominal_impact_m[1]) > MIN_REFERENCE_DISTANCE_M {
            normalise_azimuth_radians(libm::atan2(nominal_impact_m[0], nominal_impact_m[1]))
        } else {
            normalise_azimuth_radians(azimuth_rad)
        };
    let angular_variance = problem.spread.angular_sigma_rad().powi(2);
    let speed_variance = problem.spread.speed_sigma_m_s().powi(2);
    let mut range_variance_m2 = 0.0;
    let mut deflection_variance_m2 = 0.0;
    let mut range_deflection_covariance_m2 = 0.0;
    for (column, variance) in [
        (per_pitch, angular_variance),
        (per_transverse, angular_variance),
        (per_speed, speed_variance),
    ] {
        let [range, deflection] = range_and_deflection(column, range_axis_bearing_rad);
        range_variance_m2 += variance * range * range;
        deflection_variance_m2 += variance * deflection * deflection;
        range_deflection_covariance_m2 += variance * range * deflection;
    }

    let mean = (range_variance_m2 + deflection_variance_m2) / 2.0;
    let half_difference = (range_variance_m2 - deflection_variance_m2) / 2.0;
    let radius = libm::hypot(half_difference, range_deflection_covariance_m2);
    let major_variance = mean + radius;
    let minor_variance = (mean - radius).max(0.0);
    let major_axis_from_range_rad = 0.5
        * libm::atan2(
            2.0 * range_deflection_covariance_m2,
            range_variance_m2 - deflection_variance_m2,
        );

    Ok(DispersionAnalysis {
        dispersion: ImpactDispersion {
            range_probable_error_m: PROBABLE_ERROR_PER_SIGMA * range_variance_m2.sqrt(),
            deflection_probable_error_m: PROBABLE_ERROR_PER_SIGMA * deflection_variance_m2.sqrt(),
            ellipse_semi_major_m: HALF_PROBABILITY_ELLIPSE_PER_SIGMA * major_variance.sqrt(),
            ellipse_semi_minor_m: HALF_PROBABILITY_ELLIPSE_PER_SIGMA * minor_variance.sqrt(),
            ellipse_orientation_deg: fold_into_half_turn_deg(radians_to_degrees(
                range_axis_bearing_rad + major_axis_from_range_rad,
            )),
            standard_dispersion_m: problem.spread.standard_dispersion_m,
            verified_in_engine: false,
        },
        nominal_impact_m,
        range_axis_bearing_rad,
        range_variance_m2,
        deflection_variance_m2,
        range_deflection_covariance_m2,
    })
}

/// `[east, north]` metres of the descending crossing of the target height, flown in double
/// precision.
fn horizontal_impact_m(
    problem: &DispersionProblem,
    elevation_rad: f64,
    azimuth_rad: f64,
    muzzle_speed_m_s: f64,
) -> Result<[f64; 2], DispersionError> {
    let outcome = fly_to_height_double_precision(
        &problem.flight_parameters,
        &Launch {
            muzzle_speed_m_s,
            elevation_rad,
            azimuth_rad,
        },
        &problem.wind,
        problem.height_difference_m,
        PathRecording::Discard,
    )?;
    Ok([outcome.impact_position_m[0], outcome.impact_position_m[1]])
}

/// `(plus - minus) / (2·step)` per horizontal component.
fn central_difference(plus: [f64; 2], minus: [f64; 2], step: f64) -> [f64; 2] {
    [
        (plus[0] - minus[0]) / (2.0 * step),
        (plus[1] - minus[1]) / (2.0 * step),
    ]
}

/// Projects an `[east, north]` vector on the range axis at `bearing_rad` and on the deflection
/// axis to its right.
fn range_and_deflection(east_north: [f64; 2], bearing_rad: f64) -> [f64; 2] {
    let sin = libm::sin(bearing_rad);
    let cos = libm::cos(bearing_rad);
    [
        east_north[0] * sin + east_north[1] * cos,
        east_north[0] * cos - east_north[1] * sin,
    ]
}

/// Folds a bearing in degrees into `[0, 180)`.
fn fold_into_half_turn_deg(degrees: f64) -> f64 {
    let full_turn = normalise_azimuth_degrees(degrees);
    if full_turn >= HALF_TURN_DEG {
        full_turn - HALF_TURN_DEG
    } else {
        full_turn
    }
}

fn require(parameter: &'static str, value: f64, valid: bool) -> Result<(), DispersionError> {
    if valid {
        Ok(())
    } else {
        Err(DispersionError::InvalidInput { parameter, value })
    }
}

/// Reads `verified_in_engine`, refusing anything but the contract's constant `false`.
fn deserialize_not_verified_in_engine<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: serde::Deserializer<'de>,
{
    if bool::deserialize(deserializer)? {
        Err(serde::de::Error::custom(
            "verified_in_engine must be false: no engine call verifies a dispersion",
        ))
    } else {
        Ok(false)
    }
}

#[cfg(test)]
#[path = "tests/dispersion.rs"]
mod tests;
