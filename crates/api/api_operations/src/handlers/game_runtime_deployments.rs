//! Game-runtime deployment routes: authorize a player life into an ORBAT slot, and end exactly
//! one life. Both require a `mod_runtime` machine credential and act only within the
//! credential's own server and runtime session.

use api_identifiers::{LiveSlotOccupancyId, RuntimeSessionId};
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::response::Json;
use fleet_wire_contract::ExecutorKind;
use std::str::FromStr;

use crate::models::live_occupancy::{DeploymentDecision, DeploymentRequest, EndedLife};
use crate::services::live_slot_occupancy::{authorize_deployment, end_life};
use api_caller_identity::machine_caller::MachineCaller;
use api_foundation::error_handling::api_error::ApiError;
use api_foundation::http::path_parameters::PathParams;
use api_state::AppState;

fn parse_id<Id: FromStr>(raw: &str, field: &str) -> Result<Id, ApiError> {
    raw.parse()
        .map_err(|_| ApiError::bad_request(format!("invalid {field}")))
}

/// A refusal is a decision, not an error: it answers 200 with `decision = "denied"`.
///
/// @route POST /api/v1/game-runtime/sessions/:sessionId/deployments
pub async fn authorize_player_deployment(
    State(state): State<AppState>,
    caller: MachineCaller,
    PathParams(session): PathParams<String>,
    body: Result<Json<DeploymentRequest>, JsonRejection>,
) -> Result<Json<DeploymentDecision>, ApiError> {
    caller.require_executor(ExecutorKind::ModRuntime)?;
    let session: RuntimeSessionId = parse_id(&session, "runtime session id")?;
    let Json(request) = body.map_err(ApiError::from_json_rejection)?;
    let mut transaction = state.pool.begin().await?;
    let decision = authorize_deployment(
        &mut transaction,
        &caller,
        session,
        &request,
        &state.cfg.discord_guild_id,
    )
    .await?;
    transaction.commit().await?;
    Ok(Json(decision))
}

/// @route POST /api/v1/game-runtime/sessions/:sessionId/deployments/:occupancyId/end
pub async fn end_player_life(
    State(state): State<AppState>,
    caller: MachineCaller,
    PathParams((session, occupancy)): PathParams<(String, String)>,
) -> Result<Json<EndedLife>, ApiError> {
    caller.require_executor(ExecutorKind::ModRuntime)?;
    let session: RuntimeSessionId = parse_id(&session, "runtime session id")?;
    let occupancy: LiveSlotOccupancyId = parse_id(&occupancy, "occupancy id")?;
    let mut transaction = state.pool.begin().await?;
    let ended = end_life(&mut transaction, &caller, session, occupancy).await?;
    transaction.commit().await?;
    Ok(Json(ended))
}
