//! The one fire-mission assembler: inputs against a pinned catalog to the whole solution.
//!
//! **Role:** turns the input fields of a fire-mission save (catalog reference, weapon, shell,
//! optional operator charge, target, guns, wind, burst height) and an optional terrain profile
//! under the lead gun's line of fire into the [`FireMissionSolution`]: every gun solved by
//! [`crate::battery::solve_battery`], then the lead gun's dispersion
//! ([`ballistics_solver::dispersion`]), time-fuze setting with its burst-point
//! aim ([`crate::fuze`]) and crest clearance
//! ([`ballistics_solver::crest_clearance`]), stamped with the catalog id,
//! version and [`SOLVER_REVISION`].
//!
//! **Position:** the `fire_mission_planning` crate; the API re-solves a saved fire mission and the
//! mortar calculator solves on the page, both through [`solve_fire_mission`], so the two
//! produce the same bytes from the same inputs; the API then checks the client's solution
//! against its own with
//! [`crate::fire_mission_comparison::compare_solutions`].
//!
//! **Signals & state:** none; pure functions over plain values and a borrowed catalog.
//!
//! **Invariants:**
//! - The fired charge is the operator's `charge_rings` when given, else the fuze's burst
//!   charge when the fuze sets, else the lead gun's recommended charge; the lead-gun
//!   dispersion, fuze and crest all describe that charge. `guns[i].recommended_rings` stays
//!   the fewest rings that solve the ground target for gun `i`.
//! - `fuze` is present exactly when a burst height is given and a charge is fired. With an
//!   operator charge it is that charge's setting; without one it is the lowest charge whose
//!   burst aim solves with a time inside the fuze window
//!   ([`crate::fuze::solve_time_fuze_over_charges`]), so a burst
//!   the ground-recommended charge cannot reach is fuzed on a stronger charge. Its
//!   `burst_aim` (the aim the gun lays to burst above the target, not the ground-impact aim,
//!   with the charge's rings) is present exactly when the fuze time is.
//! - `crest` is present exactly when a terrain profile is given and the fired charge solves;
//!   it re-flies the trajectory the gun fires: the burst-point aim when the fuze sets, else the
//!   ground-impact aim.
//! - A catalog whose id or version differs from the inputs, an unknown operator charge, a
//!   burst height for a shell without a time fuze and a malformed terrain profile are a
//!   [`FireMissionRefusal`]; a charge that does not solve is a row with its refusal instead.
//! - Deterministic: every number comes from the libm-backed solver chain, so equal inputs and
//!   catalog give bit-equal solutions on every target.
//!
//! @contract fire-mission.schema.json#/definitions/FireMissionSolution
//! @contract fire-mission.schema.json#/definitions/FuzeSetting
//! @contract fire-mission.schema.json#/definitions/FuzeBurstAim
//! @contract fire-mission.schema.json#/definitions/FireMissionSave
//! @contract fire-mission.schema.json#/definitions/MapPoint
//! @contract fire-mission.schema.json#/definitions/GunPosition
//! @contract fire-mission.schema.json#/definitions/HeightSource
//! @contract fire-mission.schema.json#/definitions/Wind

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::battery::{BatteryError, BatteryGun, BatteryRequest, GunFireSolution, solve_battery};
use crate::fuze::{
    FuzeError, FuzeRefusal, TimeFuzeSolution, solve_time_fuze, solve_time_fuze_over_charges,
};
use ballistics_model::angular_units::normalise_azimuth_radians;
use ballistics_model::catalog::{BallisticsCatalog, CatalogLookupError};
use ballistics_model::ids::{CatalogId, ShellId, WeaponId};
use ballistics_model::wind::Wind;
use ballistics_solver::crest_clearance::{
    CrestClearance, CrestClearanceError, TerrainProfile, crest_clearance_of_charge,
};
use ballistics_solver::dispersion::{DispersionError, ImpactDispersion, charge_dispersion};
use ballistics_solver::{
    ChargeProblem, ChargeSolution, FireSolutionRequest, MapPosition, solve_charge_elevation,
};

/// Revision of the solver numerics stamped on every [`FireMissionSolution`].
///
/// Bump it in the same change as anything that can alter a solved number's bits: the flight
/// model or its step, the elevation search, the wind-corrected aim loop, the dispersion,
/// fuze or crest computation, a unit conversion, or this assembler's choice of charge. A
/// stored solution then always names the numerics that produced it, and a client solution
/// from other numerics is refused by the revision check rather than by a numeric drift.
pub const SOLVER_REVISION: &str = "game-ballistics-2";

/// Where a height came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeightSource {
    /// Sampled from the terrain elevation model.
    Dem,
    /// Typed by the operator.
    Manual,
}

/// The target: map metres (x east, y north) with its height and where the height came from.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FireMissionPoint {
    /// Easting, metres.
    pub x: f64,
    /// Northing, metres.
    pub y: f64,
    /// Height, metres.
    pub height_m: f64,
    /// Where the height came from.
    pub height_source: HeightSource,
}

/// One gun of the battery: its label and position.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FireMissionGunPosition {
    /// Name shown on the plot and the fire orders.
    pub label: String,
    /// Easting, metres.
    pub x: f64,
    /// Northing, metres.
    pub y: f64,
    /// Height, metres.
    pub height_m: f64,
    /// Where the height came from.
    pub height_source: HeightSource,
}

/// Constant surface wind: speed and the meteorological direction it blows from.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FireMissionWind {
    /// Wind speed, metres per second.
    pub speed_m_s: f64,
    /// Direction the wind blows from, degrees clockwise from north.
    pub from_deg: f64,
}

/// Everything a fire mission is solved from.
///
/// The serde shape is the input fields of a `FireMissionSave` body; the body's `event_id`,
/// `target_grid` and `client_solution` are ignored on read and belong to the caller.
/// `crest_profile` is not part of the save body: the page samples it from the terrain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FireMissionInputs {
    /// Catalog the mission is solved against.
    pub catalog_id: CatalogId,
    /// Version of that catalog.
    pub catalog_version: u32,
    /// Launcher identifier in the catalog.
    pub weapon_id: WeaponId,
    /// Shell identifier in the catalog.
    pub shell_id: ShellId,
    /// The charge the operator fires; `None` fires the lead gun's recommended charge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charge_rings: Option<u32>,
    /// Where the shells must land.
    pub target: FireMissionPoint,
    /// The guns; the first is the lead gun.
    pub guns: Vec<FireMissionGunPosition>,
    /// The surface wind; `None` is calm air.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wind: Option<FireMissionWind>,
    /// Burst height above the target for a time-fuzed shell, metres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub burst_height_m: Option<f64>,
    /// Terrain under the lead gun's line of fire, for the crest clearance.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crest_profile: Option<TerrainProfile>,
}

/// The aim a time-fuzed charge lays to burst above the target.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FuzeBurstAim {
    /// Propellant rings of the fired charge.
    pub rings: u32,
    /// Azimuth to lay in degrees, `[0, 360)`.
    pub aim_azimuth_deg: f64,
    /// Azimuth to lay in the weapon's mils.
    pub aim_azimuth_mils: f64,
    /// Elevation in degrees above the horizontal.
    pub elevation_deg: f64,
    /// Elevation in the weapon's mils.
    pub elevation_mils: f64,
}

/// The lead gun's fuze setting with the burst-point aim that produces its time.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FireMissionFuze {
    /// Burst height above the target, metres.
    pub burst_height_m: f64,
    /// Fuze time in seconds; `None` when refused.
    pub time_s: Option<f64>,
    /// Shortest settable fuze time, seconds.
    pub min_s: f64,
    /// Longest settable fuze time, seconds.
    pub max_s: f64,
    /// The shell's default fuze time, seconds.
    pub default_s: f64,
    /// Why the setting is refused; `None` when it is not.
    pub refusal: Option<FuzeRefusal>,
    /// The aim to lay for the burst; `None` exactly when `time_s` is.
    pub burst_aim: Option<FuzeBurstAim>,
}

impl FireMissionFuze {
    /// The wire setting of a solved time fuze: the burst aim is kept only with a fuze time.
    pub fn from_time_fuze(solved: &TimeFuzeSolution) -> Self {
        let setting = solved.setting;
        Self {
            burst_height_m: setting.burst_height_m,
            time_s: setting.time_s,
            min_s: setting.min_s,
            max_s: setting.max_s,
            default_s: setting.default_s,
            refusal: setting.refusal,
            burst_aim: setting.time_s.and_then(|_| burst_aim_of(&solved.burst_aim)),
        }
    }
}

/// The whole battery solution with the lead gun's extras and its provenance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FireMissionSolution {
    /// Every gun's solution, in input order.
    pub guns: Vec<GunFireSolution>,
    /// The lead gun's impact spread at the fired charge; `None` when it does not solve.
    pub dispersion: Option<ImpactDispersion>,
    /// The lead gun's time-fuze setting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fuze: Option<FireMissionFuze>,
    /// The lead gun's trajectory clearance over the terrain profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub crest: Option<CrestClearance>,
    /// Catalog the solution was solved against.
    pub catalog_id: CatalogId,
    /// Version of that catalog.
    pub catalog_version: u32,
    /// [`SOLVER_REVISION`] of the numerics that produced it.
    pub solver_revision: String,
}

/// Why a fire mission has no solution at all.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum FireMissionRefusal {
    /// The catalog handed to the assembler is not the one the inputs pin.
    #[error(
        "inputs pin catalog `{requested_id}` v{requested_version}, \
         the solver holds `{held_id}` v{held_version}"
    )]
    CatalogMismatch {
        /// Catalog id of the inputs.
        requested_id: CatalogId,
        /// Catalog version of the inputs.
        requested_version: u32,
        /// Catalog id of the catalog handed in.
        held_id: CatalogId,
        /// Catalog version of the catalog handed in.
        held_version: u32,
    },
    /// The operator's charge is not one the shell accepts.
    #[error(transparent)]
    UnknownCharge(#[from] CatalogLookupError),
    /// The battery has no solution (no gun, an empty label, an invalid gun request).
    #[error(transparent)]
    Battery(#[from] BatteryError),
    /// The fired charge's dispersion values are invalid.
    #[error(transparent)]
    Dispersion(#[from] DispersionError),
    /// The fuze cannot be set (no time fuze, invalid burst height or window).
    #[error(transparent)]
    Fuze(#[from] FuzeError),
    /// The terrain profile is malformed or lies beyond the impact.
    #[error(transparent)]
    Crest(#[from] CrestClearanceError),
}

/// Solves `inputs` against `catalog` into the whole fire-mission solution.
///
/// # Errors
///
/// [`FireMissionRefusal::CatalogMismatch`] when `catalog` is not the pinned one, then the
/// refusal of the step that fails: the operator charge lookup, the battery, the fired charge's
/// dispersion, the fuze or the crest clearance.
pub fn solve_fire_mission(
    catalog: &BallisticsCatalog,
    inputs: &FireMissionInputs,
) -> Result<FireMissionSolution, FireMissionRefusal> {
    if catalog.catalog_id != inputs.catalog_id || catalog.catalog_version != inputs.catalog_version
    {
        return Err(FireMissionRefusal::CatalogMismatch {
            requested_id: inputs.catalog_id.clone(),
            requested_version: inputs.catalog_version,
            held_id: catalog.catalog_id.clone(),
            held_version: catalog.catalog_version,
        });
    }
    if let Some(rings) = inputs.charge_rings {
        catalog.resolve_firing(&inputs.weapon_id, &inputs.shell_id, rings)?;
    }
    let wind = inputs.wind.map_or(Wind::CALM, |wind| Wind {
        speed_m_s: wind.speed_m_s,
        from_deg: wind.from_deg,
    });
    let target = MapPosition {
        x_m: inputs.target.x,
        y_m: inputs.target.y,
        height_m: inputs.target.height_m,
    };
    let battery_guns: Vec<BatteryGun> = inputs
        .guns
        .iter()
        .map(|gun| BatteryGun {
            label: gun.label.clone(),
            position: MapPosition {
                x_m: gun.x,
                y_m: gun.y,
                height_m: gun.height_m,
            },
        })
        .collect();
    let guns = solve_battery(
        catalog,
        &BatteryRequest {
            weapon_id: &inputs.weapon_id,
            shell_id: &inputs.shell_id,
            guns: &battery_guns,
            target,
            wind,
        },
    )?;

    let lead = &guns[0];
    let lead_request = FireSolutionRequest {
        weapon_id: &inputs.weapon_id,
        shell_id: &inputs.shell_id,
        gun: battery_guns[0].position,
        target,
        wind,
    };
    let fuze = match (
        inputs.burst_height_m,
        inputs.charge_rings,
        lead.recommended_rings,
    ) {
        (Some(burst_height_m), Some(rings), _) => Some(FireMissionFuze::from_time_fuze(
            &solve_time_fuze(catalog, &lead_request, rings, burst_height_m)?,
        )),
        (Some(burst_height_m), None, Some(_)) => Some(FireMissionFuze::from_time_fuze(
            &solve_time_fuze_over_charges(catalog, &lead_request, burst_height_m)?,
        )),
        _ => None,
    };
    let fired_rings = inputs
        .charge_rings
        .or_else(|| fuze.and_then(|fuze| fuze.burst_aim).map(|aim| aim.rings))
        .or(lead.recommended_rings);
    let dispersion = match fired_rings {
        Some(rings) if inputs.charge_rings.is_some() || fired_rings != lead.recommended_rings => {
            fired_charge_dispersion(catalog, &lead_request, lead, rings)?
        }
        _ => lead.dispersion,
    };
    let crest = match (&inputs.crest_profile, fired_rings) {
        (Some(profile), Some(rings)) => {
            let burst_height_m = fuze
                .filter(|fuze| fuze.burst_aim.is_some())
                .map_or(0.0, |fuze| fuze.burst_height_m);
            fired_crest(catalog, &lead_request, rings, burst_height_m, profile)?
        }
        _ => None,
    };

    Ok(FireMissionSolution {
        guns,
        dispersion,
        fuze,
        crest,
        catalog_id: catalog.catalog_id.clone(),
        catalog_version: catalog.catalog_version,
        solver_revision: SOLVER_REVISION.to_owned(),
    })
}

/// The dispersion of the lead gun's `rings` row: `None` when the row does not solve or a
/// spread flight does not land.
fn fired_charge_dispersion(
    catalog: &BallisticsCatalog,
    request: &FireSolutionRequest<'_>,
    lead: &GunFireSolution,
    rings: u32,
) -> Result<Option<ImpactDispersion>, DispersionError> {
    let Some(charge) = lead
        .charges
        .iter()
        .find(|charge| charge.rings == rings && charge.solves())
    else {
        return Ok(None);
    };
    match charge_dispersion(catalog, request, charge) {
        Ok(analysis) => Ok(Some(analysis.dispersion)),
        Err(DispersionError::Flight(_)) => Ok(None),
        Err(error) => Err(error),
    }
}

/// The crest clearance of the lead gun's `rings` charge solved onto the target raised by
/// `burst_height_m`: `None` when that charge does not solve.
fn fired_crest(
    catalog: &BallisticsCatalog,
    request: &FireSolutionRequest<'_>,
    rings: u32,
    burst_height_m: f64,
    profile: &TerrainProfile,
) -> Result<Option<CrestClearance>, FireMissionRefusal> {
    let firing = catalog.resolve_firing(request.weapon_id, request.shell_id, rings)?;
    let east_m = request.target.x_m - request.gun.x_m;
    let north_m = request.target.y_m - request.gun.y_m;
    let problem = ChargeProblem {
        flight_parameters: firing.flight_parameters,
        muzzle_speed_m_s: firing.muzzle_speed_m_s,
        elevation_limits_rad: firing.weapon.elevation_limits_rad(),
        azimuth_rad: normalise_azimuth_radians(libm::atan2(east_m, north_m)),
        distance_m: libm::hypot(east_m, north_m),
        height_difference_m: request.target.height_m + burst_height_m - request.gun.height_m,
        wind: request.wind,
    };
    let Ok(solved) = solve_charge_elevation(&problem) else {
        return Ok(None);
    };
    Ok(Some(crest_clearance_of_charge(
        &problem,
        &solved,
        request.gun.height_m,
        profile,
    )?))
}

/// The burst aim of a solved burst-point row; `None` when the row does not solve.
fn burst_aim_of(row: &ChargeSolution) -> Option<FuzeBurstAim> {
    Some(FuzeBurstAim {
        rings: row.rings,
        aim_azimuth_deg: row.aim_azimuth_deg?,
        aim_azimuth_mils: row.aim_azimuth_mils?,
        elevation_deg: row.elevation_deg?,
        elevation_mils: row.elevation_mils?,
    })
}

#[cfg(test)]
#[path = "tests/fire_mission.rs"]
mod tests;
