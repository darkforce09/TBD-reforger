//! Game-runtime deployment routes: authorize a player life into an ORBAT slot, and end exactly
//! one life. Both require a `mod_runtime` machine credential and act only within the
//! credential's own server and runtime session.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::response::Json;
use uuid::Uuid;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::operations::models::live_occupancy::{DeploymentDecision, DeploymentRequest, EndedLife};
use crate::operations::services::live_slot_occupancy::{authorize_deployment, end_life};
use crate::server_infrastructure::models::machine_credential::ExecutorKind;
use crate::server_infrastructure::services::machine_credentials::MachineCaller;

fn parse_id(raw: &str, field: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw).map_err(|_| ApiError::bad_request(format!("invalid {field}")))
}

/// A refusal is a decision, not an error: it answers 200 with `decision = "denied"`.
///
/// @route POST /api/v1/game-runtime/sessions/:sessionId/deployments
pub async fn authorize_player_deployment(
    State(state): State<AppState>,
    caller: MachineCaller,
    Path(session): Path<String>,
    body: Result<Json<DeploymentRequest>, JsonRejection>,
) -> Result<Json<DeploymentDecision>, ApiError> {
    caller.require_executor(ExecutorKind::ModRuntime)?;
    let session = parse_id(&session, "runtime session id")?;
    let Json(request) = body.map_err(|rejection| {
        ApiError::bad_request(format!("invalid body: {}", rejection.body_text()))
    })?;
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
    Path((session, occupancy)): Path<(String, String)>,
) -> Result<Json<EndedLife>, ApiError> {
    caller.require_executor(ExecutorKind::ModRuntime)?;
    let session = parse_id(&session, "runtime session id")?;
    let occupancy = parse_id(&occupancy, "occupancy id")?;
    let mut transaction = state.pool.begin().await?;
    let ended = end_life(&mut transaction, &caller, session, occupancy).await?;
    transaction.commit().await?;
    Ok(Json(ended))
}
