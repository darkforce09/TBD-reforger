//! `GET /api/v1/events/{id}/fire-missions`: an event's saved fire missions.
//!
//! **Role:** answers the fire missions of one event, oldest first, legacy rows and
//! catalog-model rows alike, each catalog-model row with its guns.
//!
//! **Position:** operations handlers; reads through
//! [`crate::services::fire_mission_store::list_event_fire_missions`] after the event
//! access check of [`super`]. The mortar calculator's saved list is the caller.
//!
//! **Signals & state:** none.
//!
//! **Invariants:**
//! - A legacy row (stored before catalogs, including weapons no catalog solves) lists with
//!   every catalog-model field `null` and no guns: that is the mark that it cannot be
//!   re-solved.
//! - A caller who may not see the event gets the same 404 as for a missing event.
//!
//! @contract fire-mission.schema.json#/definitions/FireMissionList

use api_identifiers::EventId;
use axum::extract::State;
use axum::response::Json;
use serde::Serialize;

use crate::models::fire_mission::FireMission;
use crate::services::fire_mission_store::list_event_fire_missions;
use api_foundation::error_handling::api_error::ApiError;
use api_foundation::http::path_parameters::PathParams;
use api_http_layer::middleware::AuthUser;
use api_state::AppState;

/// The `FireMissionList` answer.
#[derive(Debug, Serialize)]
pub struct FireMissionList {
    /// The event's fire missions, oldest first.
    pub data: Vec<FireMission>,
}

/// `GET /api/v1/events/:id/fire-missions` — an event's saved fire missions, oldest first.
///
/// @route GET /api/v1/events/:id/fire-missions
pub async fn list_fire_missions(
    State(state): State<AppState>,
    user: AuthUser,
    PathParams(id): PathParams<String>,
) -> Result<Json<FireMissionList>, ApiError> {
    let Ok(event) = id.parse::<EventId>() else {
        return Err(ApiError::bad_request("invalid id"));
    };
    super::require_full_event_access(&state, &user, event).await?;
    let data = list_event_fire_missions(&state.pool, event).await?;
    Ok(Json(FireMissionList { data }))
}
