//! A battery's firing solution: every gun solved on its own onto one target.
//!
//! **Role:** takes several guns (label and map position), one target, one catalog weapon and
//! shell and the wind, and solves each gun independently with
//! [`ballistics_solver::solve_fire_solution`]: its own bearing, charge
//! rows (aim azimuth, elevation, time of flight, apex, wind corrections), recommended charge and
//! the impact dispersion of that charge
//! ([`ballistics_solver::dispersion::charge_dispersion`]).
//!
//! **Position:** the `fire_mission_planning` crate; the API re-solves a submitted fire mission's
//! guns and the mortar calculator solves its battery locally, both through [`solve_battery`]; the
//! agreement lattice of the `ballistics_agreement_cases` crate feeds it the same cases on native
//! and wasm32 builds.
//!
//! **Signals & state:** none; borrows the catalog read-only.
//!
//! **Invariants:**
//! - Gun `i` of the request is `guns[i]` of the answer with `gun_index = i`; no gun's answer
//!   depends on another gun (a battery of one gun is exactly the one-gun solution).
//! - The azimuth to lay is each charge row's aim azimuth (wind-corrected); the gun's
//!   `azimuth_deg`/`azimuth_mils` stay the geometric gun-to-target bearing.
//! - A gun's dispersion is that of its recommended charge laid at the row's aim; it is `None`
//!   when no charge solves or when a finite-difference flight of the spread does not land (a
//!   solution at the edge of the envelope). A catalog whose dispersion values are invalid fails
//!   the battery.
//! - A battery has at least one gun and every label is non-empty after trimming; a gun whose
//!   request is invalid fails the whole battery with its index.
//!
//! @contract fire-mission.schema.json#/definitions/GunFireSolution

use serde::{Deserialize, Serialize};
use thiserror::Error;

use ballistics_model::catalog::BallisticsCatalog;
use ballistics_model::ids::{ShellId, WeaponId};
use ballistics_model::wind::Wind;
use ballistics_solver::dispersion::{DispersionError, ImpactDispersion, charge_dispersion};
use ballistics_solver::{
    ChargeSolution, FireSolution, FireSolutionError, FireSolutionRequest, MapPosition,
    solve_fire_solution,
};

/// One gun of a battery.
#[derive(Debug, Clone, PartialEq)]
pub struct BatteryGun {
    /// Name shown on the plot and the fire orders; non-empty.
    pub label: String,
    /// Where the gun stands.
    pub position: MapPosition,
}

/// A battery of guns firing one catalog weapon and shell onto one target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BatteryRequest<'request> {
    /// Launcher identifier in the catalog; every gun is this weapon.
    pub weapon_id: &'request WeaponId,
    /// Shell identifier in the catalog.
    pub shell_id: &'request ShellId,
    /// The guns, in the order their answers are returned.
    pub guns: &'request [BatteryGun],
    /// Where the shells must land.
    pub target: MapPosition,
    /// The surface wind; [`Wind::CALM`] for none.
    pub wind: Wind,
}

/// One gun's answer: its index and label with the one-gun solution's bearing and charge rows.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GunFireSolution {
    /// Position of the gun in the request, from zero.
    pub gun_index: u32,
    /// The gun's label.
    pub label: String,
    /// Horizontal gun-to-target distance, metres.
    pub distance_m: f64,
    /// Target height minus gun height, metres.
    pub height_difference_m: f64,
    /// Geometric gun-to-target azimuth in degrees, `[0, 360)`.
    pub azimuth_deg: f64,
    /// Geometric azimuth in the weapon's mils, `[0, mils_per_circle)`.
    pub azimuth_mils: f64,
    /// Mils in the weapon's full circle.
    pub mils_per_circle: u32,
    /// One row per charge of the shell, in catalog order.
    pub charges: Vec<ChargeSolution>,
    /// Rings of the recommended charge, the fewest that solve; `None` when none does.
    pub recommended_rings: Option<u32>,
    /// Impact spread of the recommended charge; `None` when none solves or the spread does
    /// not propagate.
    pub dispersion: Option<ImpactDispersion>,
}

impl GunFireSolution {
    /// The answer of gun `gun_index` labelled `label` from its one-gun `solution` and the
    /// `dispersion` of its recommended charge.
    pub fn from_fire_solution(
        gun_index: u32,
        label: String,
        solution: FireSolution,
        dispersion: Option<ImpactDispersion>,
    ) -> Self {
        Self {
            gun_index,
            label,
            distance_m: solution.distance_m,
            height_difference_m: solution.height_difference_m,
            azimuth_deg: solution.azimuth_deg,
            azimuth_mils: solution.azimuth_mils,
            mils_per_circle: solution.mils_per_circle,
            charges: solution.charges,
            recommended_rings: solution.recommended_rings,
            dispersion,
        }
    }

    /// The recommended charge's row: the aim azimuth, elevation and time of flight to fire.
    pub fn recommended_charge(&self) -> Option<&ChargeSolution> {
        let rings = self.recommended_rings?;
        self.charges.iter().find(|charge| charge.rings == rings)
    }
}

/// Why a battery has no firing solution.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum BatteryError {
    /// The request names no gun.
    #[error("a battery needs at least one gun")]
    NoGuns,
    /// A gun's label is empty or only whitespace.
    #[error("gun {gun_index} has an empty label")]
    EmptyLabel {
        /// Position of the gun in the request.
        gun_index: u32,
    },
    /// A gun's one-gun request is invalid.
    #[error("gun {gun_index}: {source}")]
    Gun {
        /// Position of the gun in the request.
        gun_index: u32,
        /// The one-gun solver's error.
        #[source]
        source: FireSolutionError,
    },
    /// The catalog's dispersion values for the recommended charge are invalid.
    #[error("gun {gun_index} dispersion: {source}")]
    Dispersion {
        /// Position of the gun in the request.
        gun_index: u32,
        /// The dispersion error.
        #[source]
        source: DispersionError,
    },
    /// The request has more guns than a `u32` index counts.
    #[error("a battery has at most {max} guns", max = u32::MAX)]
    TooManyGuns,
}

/// Solves every gun of `request` onto its target, each on its own.
///
/// # Errors
///
/// [`BatteryError::NoGuns`], [`BatteryError::TooManyGuns`], [`BatteryError::EmptyLabel`], or
/// [`BatteryError::Gun`] with the first gun whose one-gun request fails, or
/// [`BatteryError::Dispersion`] when the catalog's dispersion values are invalid; a charge that
/// does not solve is a row with its refusal instead.
pub fn solve_battery(
    catalog: &BallisticsCatalog,
    request: &BatteryRequest<'_>,
) -> Result<Vec<GunFireSolution>, BatteryError> {
    if request.guns.is_empty() {
        return Err(BatteryError::NoGuns);
    }
    request
        .guns
        .iter()
        .enumerate()
        .map(|(position, gun)| {
            let gun_index = u32::try_from(position).map_err(|_| BatteryError::TooManyGuns)?;
            if gun.label.trim().is_empty() {
                return Err(BatteryError::EmptyLabel { gun_index });
            }
            let one_gun = FireSolutionRequest {
                weapon_id: request.weapon_id,
                shell_id: request.shell_id,
                gun: gun.position,
                target: request.target,
                wind: request.wind,
            };
            let solution = solve_fire_solution(catalog, &one_gun)
                .map_err(|source| BatteryError::Gun { gun_index, source })?;
            let dispersion = recommended_dispersion(catalog, &one_gun, &solution)
                .map_err(|source| BatteryError::Dispersion { gun_index, source })?;
            Ok(GunFireSolution::from_fire_solution(
                gun_index,
                gun.label.clone(),
                solution,
                dispersion,
            ))
        })
        .collect()
}

/// The dispersion of the recommended charge of `solution`: `None` without a recommended charge
/// or when a spread flight does not land.
fn recommended_dispersion(
    catalog: &BallisticsCatalog,
    request: &FireSolutionRequest<'_>,
    solution: &FireSolution,
) -> Result<Option<ImpactDispersion>, DispersionError> {
    let Some(charge) = solution.recommended_charge() else {
        return Ok(None);
    };
    match charge_dispersion(catalog, request, charge) {
        Ok(analysis) => Ok(Some(analysis.dispersion)),
        Err(DispersionError::Flight(_)) => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
#[path = "tests/battery.rs"]
mod tests;
