//! The server's re-solve of a fire-mission save and its check of the client's solution.
//!
//! **Role:** loads the catalog version a save pins, re-solves the save's inputs through the map
//! engine's one fire-mission assembler, and compares the client's solution with the re-solve
//! under the weapon's mils convention.
//!
//! **Position:** operations services; called by
//! [`crate::operations::handlers::fire_missions::save`] before anything is stored. Reads the pinned
//! catalog through
//! [`crate::operations::services::ballistics_catalogs::catalog_store::load_catalog`]; solves with
//! [`map_engine::data::scenario::ballistics::fire_mission::solve_fire_mission`] and compares
//! with [`map_engine::data::scenario::ballistics::fire_mission_comparison::compare_solutions`].
//! The stored row is written by [`crate::operations::services::fire_mission_store`].
//!
//! **Signals & state:** none; the solve runs on the blocking pool, so a large battery never
//! stalls the async workers.
//!
//! **Invariants:**
//! - The API never assembles a solution: the stored and answered solution is the map engine's
//!   re-solve, bit for bit.
//! - An unknown catalog id or version answers 404; a save the assembler refuses answers 422 with
//!   `details.code = "fire_mission_refused"`; a client solution the comparison rule does not
//!   tolerate answers 422 with `details.code = "solution_mismatch"`, every mismatch with both
//!   the client's and the server's value, and the server's solution.
//! - A lead gun whose fired charge does not solve answers 422 with
//!   `details.code = "no_firing_solution"`: the row's single-tube summary columns are required
//!   and no invented number fills them.

use axum::http::StatusCode;
use map_engine::data::scenario::ballistics::battery::GunFireSolution;
use map_engine::data::scenario::ballistics::fire_mission::{
    FireMissionInputs, FireMissionSolution, solve_fire_mission,
};
use map_engine::data::scenario::ballistics::fire_mission_comparison::compare_solutions;
use map_engine::data::scenario::ballistics::solver::ChargeSolution;
use serde_json::json;

use crate::core::error_handling::api_error::ApiError;
use crate::operations::services::ballistics_catalogs::catalog_store::{
    CatalogLoadError, load_catalog,
};

/// `details.code` of a client solution the server's re-solve does not tolerate.
pub const SOLUTION_MISMATCH_CODE: &str = "solution_mismatch";
/// `details.code` of a save the fire-mission assembler refuses as a whole.
pub const FIRE_MISSION_REFUSED_CODE: &str = "fire_mission_refused";
/// `details.code` of a save whose lead gun has no solving charge to fire.
pub const NO_FIRING_SOLUTION_CODE: &str = "no_firing_solution";

/// A save's re-solve that agrees with the client, with what the stored row needs of the catalog.
#[derive(Debug, Clone)]
pub struct ResolvedFireMission {
    /// The server's solution: the one stored and answered.
    pub solution: FireMissionSolution,
    /// Mils per full circle of the pinned weapon.
    pub mils_per_circle: u32,
    /// The pinned weapon's human-readable name, stored as the row's `weapon_system`.
    pub weapon_display_name: String,
}

/// One gun's fired charge: the operator's charge when given, else the gun's recommended one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FiredCharge {
    /// Azimuth the gun lays in the weapon's mils: the fired charge's aim azimuth when it solves,
    /// else the geometric azimuth.
    pub azimuth_mils: f64,
    /// Elevation in the weapon's mils; `None` when the fired charge does not solve.
    pub elevation_mils: Option<f64>,
    /// Rings of the fired charge; `None` when it does not solve.
    pub charge_rings: Option<u32>,
    /// Seconds to splash; `None` when the fired charge does not solve.
    pub time_of_flight_s: Option<f64>,
}

/// The charge `gun` fires when the operator chose `operator_rings` (or none), read from its
/// charge rows. The three solution values are present together or absent together.
pub fn fired_charge(gun: &GunFireSolution, operator_rings: Option<u32>) -> FiredCharge {
    let solved: Option<&ChargeSolution> = operator_rings
        .or(gun.recommended_rings)
        .and_then(|rings| gun.charges.iter().find(|row| row.rings == rings))
        .filter(|row| row.solves());
    match solved.and_then(|row| {
        Some((
            row.rings,
            row.elevation_mils?,
            row.time_of_flight_s?,
            row.aim_azimuth_mils.unwrap_or(gun.azimuth_mils),
        ))
    }) {
        Some((rings, elevation_mils, time_of_flight_s, azimuth_mils)) => FiredCharge {
            azimuth_mils,
            elevation_mils: Some(elevation_mils),
            charge_rings: Some(rings),
            time_of_flight_s: Some(time_of_flight_s),
        },
        None => FiredCharge {
            azimuth_mils: gun.azimuth_mils,
            elevation_mils: None,
            charge_rings: None,
            time_of_flight_s: None,
        },
    }
}

/// Re-solves `inputs` against the catalog version they pin and checks `client` against it.
///
/// # Errors
///
/// 404 for a catalog version that is not stored; 422 with a `details.code` of
/// [`FIRE_MISSION_REFUSED_CODE`], [`NO_FIRING_SOLUTION_CODE`] or [`SOLUTION_MISMATCH_CODE`];
/// 500 for a failed read, a stored catalog this build cannot decode, or a solve that panicked.
pub async fn resolve_fire_mission(
    pool: &sqlx::PgPool,
    inputs: FireMissionInputs,
    client: &FireMissionSolution,
) -> Result<ResolvedFireMission, ApiError> {
    let not_found = || ApiError::not_found("ballistics catalog version not found");
    let version = i32::try_from(inputs.catalog_version).map_err(|_| not_found())?;
    let catalog = match load_catalog(pool, &inputs.catalog_id, version).await {
        Ok(Some(catalog)) => catalog,
        Ok(None) => return Err(not_found()),
        Err(CatalogLoadError::Database(error)) => return Err(error.into()),
        Err(CatalogLoadError::Undecodable(error)) => {
            tracing::error!(
                catalog_id = %inputs.catalog_id,
                catalog_version = version,
                %error,
                "stored ballistics catalog does not decode"
            );
            return Err(ApiError::internal("internal error"));
        }
    };
    let operator_rings = inputs.charge_rings;
    let weapon_id = inputs.weapon_id.clone();
    let (catalog, solved) = tokio::task::spawn_blocking(move || {
        let solved = solve_fire_mission(&catalog, &inputs);
        (catalog, solved)
    })
    .await
    .map_err(|error| {
        tracing::error!(%error, "fire-mission solve task failed");
        ApiError::internal("internal error")
    })?;
    let server = solved.map_err(|refusal| {
        ApiError::with_details(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("fire mission refused: {refusal}"),
            json!({ "code": FIRE_MISSION_REFUSED_CODE, "reason": refusal.to_string() }),
        )
    })?;
    // The battery refuses an empty gun list, so a solved mission has a lead gun.
    let lead = &server.guns[0];
    let mils_per_circle = lead.mils_per_circle;
    let comparison = compare_solutions(client, &server, mils_per_circle);
    if !comparison.agrees() {
        return Err(ApiError::with_details(
            StatusCode::UNPROCESSABLE_ENTITY,
            "client solution disagrees with the server's re-solve",
            json!({
                "code": SOLUTION_MISMATCH_CODE,
                "mismatches": comparison.mismatches,
                "server_solution": server,
            }),
        ));
    }
    if fired_charge(lead, operator_rings).elevation_mils.is_none() {
        return Err(ApiError::with_details(
            StatusCode::UNPROCESSABLE_ENTITY,
            "the lead gun has no solving charge to fire",
            json!({ "code": NO_FIRING_SOLUTION_CODE, "server_solution": server }),
        ));
    }
    let weapon_display_name = catalog
        .weapon(&weapon_id)
        .map(|weapon| weapon.display_name.clone())
        .map_err(|error| {
            ApiError::with_details(
                StatusCode::UNPROCESSABLE_ENTITY,
                format!("fire mission refused: {error}"),
                json!({ "code": FIRE_MISSION_REFUSED_CODE, "reason": error.to_string() }),
            )
        })?;
    Ok(ResolvedFireMission {
        solution: server,
        mils_per_circle,
        weapon_display_name,
    })
}
