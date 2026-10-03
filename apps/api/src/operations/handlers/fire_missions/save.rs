//! `POST /api/v1/fire-missions`: a signed-in member's fire mission, re-solved and stored.
//!
//! **Role:** decodes a `FireMissionSave` body, checks what the solver cannot (the event, the
//! grid text, the wind's stored range), re-solves it through
//! [`crate::operations::services::fire_mission_resolve::resolve_fire_mission`] and stores the
//! accepted mission with its guns through
//! [`crate::operations::services::fire_mission_store::insert_fire_mission`].
//!
//! **Position:** operations handlers; the mortar calculator is the caller.
//!
//! **Signals & state:** none.
//!
//! **Invariants:**
//! - The body is decoded strictly: an unknown or missing field is a 400 naming it, so a body
//!   in the retired single-tube shape is refused rather than half read.
//! - `event_id` absent or `null` saves outside an event; a present value must be a UUID of an
//!   event the caller fully sees (see [`super`]); a blank one is refused, never demoted to "no
//!   event".
//! - A battery of more than [`MAX_BATTERY_GUNS`] guns is a 400 before any solve.
//! - The answer is 201 with the server's solution and the stored mission read back through
//!   `RETURNING`, never the client's solution.
//!
//! @contract fire-mission.schema.json#/definitions/FireMissionSave
//! @contract fire-mission.schema.json#/definitions/SavedFireMission

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::Json;
use fire_mission_planning::fire_mission::{
    FireMissionGunPosition, FireMissionInputs, FireMissionPoint, FireMissionSolution,
    FireMissionWind,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AuthUser;
use crate::operations::models::fire_mission::FireMission;
use crate::operations::services::fire_mission_resolve::resolve_fire_mission;
use crate::operations::services::fire_mission_store::{NewFireMission, insert_fire_mission};

/// The `FireMissionSave` body: the solve inputs against a pinned catalog version, the target's
/// grid text and the client's solution.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FireMissionSaveBody {
    /// The event the mission belongs to; absent or `null` outside an event.
    #[serde(default)]
    pub event_id: Option<String>,
    /// Catalog the mission is solved against.
    pub catalog_id: String,
    /// Version of that catalog, 1 or more.
    pub catalog_version: u32,
    /// Launcher identifier in the catalog.
    pub weapon_id: String,
    /// Shell identifier in the catalog.
    pub shell_id: String,
    /// The operator's charge; absent or `null` fires each gun's recommended charge.
    #[serde(default)]
    pub charge_rings: Option<u32>,
    /// Where the shells must land.
    pub target: FireMissionPoint,
    /// The guns; the first is the lead gun.
    pub guns: Vec<FireMissionGunPosition>,
    /// The surface wind; absent or `null` is calm air.
    #[serde(default)]
    pub wind: Option<FireMissionWind>,
    /// Burst height above the target for a time-fuzed shell, metres.
    #[serde(default)]
    pub burst_height_m: Option<f64>,
    /// The target's grid reference as the operator entered it.
    pub target_grid: String,
    /// The solution the client computed and showed.
    pub client_solution: FireMissionSolution,
}

/// The `SavedFireMission` answer: the server's solution and the stored mission.
#[derive(Debug, Serialize)]
pub struct SavedFireMission {
    /// The server's re-solve, which the stored mission records.
    pub solution: FireMissionSolution,
    /// The stored mission with its guns.
    pub fire_mission: FireMission,
}

/// The event a present `event_id` names; a blank or malformed one is a 400.
fn parse_event_id(event_id: Option<&str>) -> Result<Option<Uuid>, ApiError> {
    match event_id {
        None => Ok(None),
        Some(text) if text.trim().is_empty() => Err(ApiError::bad_request(
            "event_id must not be blank — omit it or send null for no event",
        )),
        Some(text) => Uuid::parse_str(text)
            .map(Some)
            .map_err(|_| ApiError::bad_request("invalid event_id")),
    }
}

/// Most guns one saved battery holds: the contract's `FireMissionSave.guns` `maxItems`.
pub const MAX_BATTERY_GUNS: usize = 12;

/// Refuses the values the columns constrain and the solver accepts: the catalog version, the
/// battery size, the grid text and the wind.
fn check_stored_ranges(body: &FireMissionSaveBody) -> Result<(), ApiError> {
    if body.catalog_version == 0 {
        return Err(ApiError::bad_request("catalog_version must be 1 or more"));
    }
    if body.guns.len() > MAX_BATTERY_GUNS {
        return Err(ApiError::bad_request(format!(
            "a battery has at most {MAX_BATTERY_GUNS} guns"
        )));
    }
    if body.target_grid.trim().is_empty() {
        return Err(ApiError::bad_request("target_grid is required"));
    }
    if let Some(wind) = body.wind {
        if wind.speed_m_s.is_nan() || wind.speed_m_s < 0.0 {
            return Err(ApiError::bad_request("wind.speed_m_s must be 0 or more"));
        }
        if !(0.0..360.0).contains(&wind.from_deg) {
            return Err(ApiError::bad_request(
                "wind.from_deg must be in [0, 360) degrees",
            ));
        }
    }
    Ok(())
}

/// `POST /api/v1/fire-missions` — re-solve, check and store a fire mission.
///
/// @route POST /api/v1/fire-missions
pub async fn save_fire_mission(
    State(state): State<AppState>,
    user: AuthUser,
    body: Result<Json<FireMissionSaveBody>, JsonRejection>,
) -> Result<(StatusCode, Json<SavedFireMission>), ApiError> {
    let Json(body) = body.map_err(ApiError::from_json_rejection)?;
    check_stored_ranges(&body)?;
    let event_id = parse_event_id(body.event_id.as_deref())?;
    if let Some(event) = event_id {
        super::require_full_event_access(&state, &user, event).await?;
    }
    let inputs = FireMissionInputs {
        catalog_id: body.catalog_id.into(),
        catalog_version: body.catalog_version,
        weapon_id: body.weapon_id.into(),
        shell_id: body.shell_id.into(),
        charge_rings: body.charge_rings,
        target: body.target,
        guns: body.guns,
        wind: body.wind,
        burst_height_m: body.burst_height_m,
        crest_profile: None,
    };
    let resolved = resolve_fire_mission(&state.pool, inputs.clone(), &body.client_solution).await?;
    let fire_mission = insert_fire_mission(
        &state.pool,
        NewFireMission {
            event_id,
            created_by: &user.discord_id,
            target_grid: body.target_grid.trim(),
            inputs: &inputs,
            resolved: &resolved,
        },
    )
    .await?;
    Ok((
        StatusCode::CREATED,
        Json(SavedFireMission {
            solution: resolved.solution,
            fire_mission,
        }),
    ))
}
