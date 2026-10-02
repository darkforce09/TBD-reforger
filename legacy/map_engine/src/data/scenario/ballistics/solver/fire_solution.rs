//! One gun's firing solution onto one target: bearing and a row for every charge.
//!
//! **Role:** validates a request, resolves the weapon and shell in a ballistics catalog, computes
//! the gun-to-target distance, height difference and azimuth, solves every charge of the shell
//! and names the recommended charge.
//!
//! **Position:** `ballistics/solver`; the API re-solves a submitted fire mission and the mortar
//! calculator solves locally, both through [`solve_fire_solution`]; the battery composes one
//! [`FireSolution`] per gun with its label and dispersion. Charges are solved by
//! [`crate::data::scenario::ballistics::solver::charge_selection`].
//!
//! **Signals & state:** none; borrows the catalog read-only.
//!
//! **Invariants:**
//! - Map frame metres, x east, y north, heights up; the azimuth is clockwise from north, from the
//!   gun to the target, `atan2(Δx, Δy)` through `libm`, folded into one turn; the distance is
//!   `hypot(Δx, Δy)` through `libm`.
//! - [`FireSolution::azimuth_deg`] is the geometric gun-to-target bearing; each solved charge's
//!   [`crate::data::scenario::ballistics::solver::ChargeSolution`] carries its own aim azimuth
//!   and the deflection and range corrections the wind demands.
//! - Angles are reported in degrees and in the weapon's mils
//!   ([`crate::data::scenario::ballistics::angular_units::MilsConvention`] from
//!   `mils_per_circle`).
//! - Request-wide faults (unknown weapon or shell, non-finite coordinates, an invalid wind, a
//!   non-physical shell or world, elevation limits outside `[-90°, 90°]`) are a
//!   [`FireSolutionError`]; charge-specific faults are that charge's
//!   [`crate::data::scenario::ballistics::solver::SolutionRefusal`]. No
//!   input panics.
//!
//! @contract fire-mission.schema.json#/definitions/GunFireSolution

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::charge_selection::{
    ChargeProblem, ChargeSolution, recommended_rings, solve_charge_elevation,
};
use crate::data::scenario::ballistics::angular_units::{
    AngularUnitsError, normalise_azimuth_degrees, normalise_azimuth_radians, radians_to_degrees,
};
use crate::data::scenario::ballistics::catalog::{
    BallisticsCatalog, CatalogLookupError, flight_parameters, muzzle_speed_m_s,
};
use crate::data::scenario::ballistics::flight_model::FlightError;
use crate::data::scenario::ballistics::wind::{Wind, WindError};

/// Degrees in a quarter turn: the steepest elevation a weapon may declare either way.
const QUARTER_TURN_DEG: f64 = 90.0;

/// A point on the map with its height.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MapPosition {
    /// Metres east.
    pub x_m: f64,
    /// Metres north.
    pub y_m: f64,
    /// Height in metres.
    pub height_m: f64,
}

/// One gun, one target, one weapon and shell of a catalog, and the wind.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FireSolutionRequest<'request> {
    /// Launcher identifier in the catalog.
    pub weapon_id: &'request str,
    /// Shell identifier in the catalog.
    pub shell_id: &'request str,
    /// Where the gun stands.
    pub gun: MapPosition,
    /// Where the shell must land.
    pub target: MapPosition,
    /// The surface wind; [`Wind::CALM`] for none.
    pub wind: Wind,
}

/// One gun's bearing and charge rows: the fields of a gun's solution the solver owns (the
/// battery adds the gun's index, label and dispersion).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FireSolution {
    /// Horizontal gun-to-target distance, metres.
    pub distance_m: f64,
    /// Target height minus gun height, metres.
    pub height_difference_m: f64,
    /// Geometric gun-to-target azimuth in degrees, `[0, 360)`; the azimuth to lay is each
    /// charge's aim azimuth.
    pub azimuth_deg: f64,
    /// Geometric azimuth in the weapon's mils, `[0, mils_per_circle)`.
    pub azimuth_mils: f64,
    /// Mils in the weapon's full circle.
    pub mils_per_circle: u32,
    /// One row per charge of the shell, in catalog order.
    pub charges: Vec<ChargeSolution>,
    /// Rings of the recommended charge, the fewest that solve; `None` when none does.
    pub recommended_rings: Option<u32>,
}

impl FireSolution {
    /// The recommended charge's row.
    pub fn recommended_charge(&self) -> Option<&ChargeSolution> {
        let rings = self.recommended_rings?;
        self.charges.iter().find(|charge| charge.rings == rings)
    }
}

/// Why a request has no firing solution at all.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum FireSolutionError {
    /// The catalog has no such weapon, shell or pairing.
    #[error(transparent)]
    Lookup(#[from] CatalogLookupError),
    /// A request or weapon value is NaN, infinite or out of its domain.
    #[error("fire solution input `{parameter}` is invalid (got {value})")]
    InvalidInput {
        /// Name of the offending input.
        parameter: &'static str,
        /// The value that was refused.
        value: f64,
    },
    /// The wind report is invalid.
    #[error(transparent)]
    InvalidWind(#[from] WindError),
    /// The weapon declares zero mils per circle.
    #[error(transparent)]
    InvalidMilsConvention(#[from] AngularUnitsError),
    /// The catalog's gravity or the shell's constants are not physical.
    #[error("shell flight parameters are invalid: {0}")]
    InvalidFlightParameters(FlightError),
}

/// Solves every charge of the requested shell from the gun onto the target.
///
/// # Errors
///
/// [`FireSolutionError`] for a request-wide fault; a charge that does not solve is a row with
/// its [`crate::data::scenario::ballistics::solver::SolutionRefusal`] instead.
pub fn solve_fire_solution(
    catalog: &BallisticsCatalog,
    request: &FireSolutionRequest<'_>,
) -> Result<FireSolution, FireSolutionError> {
    let (weapon, shell) = catalog.weapon_and_shell(request.weapon_id, request.shell_id)?;
    let mils = weapon.mils_convention()?;
    request.wind.validate()?;
    for (parameter, value) in [
        ("gun.x_m", request.gun.x_m),
        ("gun.y_m", request.gun.y_m),
        ("gun.height_m", request.gun.height_m),
        ("target.x_m", request.target.x_m),
        ("target.y_m", request.target.y_m),
        ("target.height_m", request.target.height_m),
    ] {
        require(parameter, value, value.is_finite())?;
    }
    for (parameter, value) in [
        ("elevation_min_deg", weapon.elevation_min_deg),
        ("elevation_max_deg", weapon.elevation_max_deg),
    ] {
        require(
            parameter,
            value,
            value.is_finite() && value.abs() <= QUARTER_TURN_DEG,
        )?;
    }
    require(
        "elevation_max_deg",
        weapon.elevation_max_deg,
        weapon.elevation_min_deg < weapon.elevation_max_deg,
    )?;
    let parameters = flight_parameters(catalog.gravity_m_s2, shell);
    parameters
        .validate()
        .map_err(FireSolutionError::InvalidFlightParameters)?;

    let east_m = request.target.x_m - request.gun.x_m;
    let north_m = request.target.y_m - request.gun.y_m;
    let height_difference_m = request.target.height_m - request.gun.height_m;
    let distance_m = libm::hypot(east_m, north_m);
    require("distance_m", distance_m, distance_m.is_finite())?;
    require(
        "height_difference_m",
        height_difference_m,
        height_difference_m.is_finite(),
    )?;
    let azimuth_rad = normalise_azimuth_radians(libm::atan2(east_m, north_m));

    let charges: Vec<ChargeSolution> = shell
        .charges
        .iter()
        .map(|charge| {
            let problem = ChargeProblem {
                flight_parameters: parameters,
                muzzle_speed_m_s: muzzle_speed_m_s(weapon, shell, charge),
                elevation_limits_rad: weapon.elevation_limits_rad(),
                azimuth_rad,
                distance_m,
                height_difference_m,
                wind: request.wind,
            };
            ChargeSolution::from_result(charge.rings, solve_charge_elevation(&problem), mils)
        })
        .collect();
    Ok(FireSolution {
        distance_m,
        height_difference_m,
        azimuth_deg: normalise_azimuth_degrees(radians_to_degrees(azimuth_rad)),
        azimuth_mils: mils.azimuth_radians_to_mils(azimuth_rad),
        mils_per_circle: mils.mils_per_circle(),
        recommended_rings: recommended_rings(&charges),
        charges,
    })
}

fn require(parameter: &'static str, value: f64, valid: bool) -> Result<(), FireSolutionError> {
    if valid {
        Ok(())
    } else {
        Err(FireSolutionError::InvalidInput { parameter, value })
    }
}
