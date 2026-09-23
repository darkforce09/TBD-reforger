//! Executor routes of the fleet command ledger, for host agents and game runtimes: claim the
//! next command, report that its effect is starting, and report its outcome. Each takes the
//! caller's machine credential and acts only on the caller's server and executor kind.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde::Deserialize;
use uuid::Uuid;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::server_infrastructure::models::fleet_command::{
    ExecutionResult, ExecutionStart, FleetCommandReceipt,
};
use crate::server_infrastructure::services::fleet_commands::executor_claims::{
    claim_next_command, mark_executing, record_result,
};
use crate::server_infrastructure::services::machine_credentials::MachineCaller;

fn body<T>(input: Result<Json<T>, JsonRejection>) -> Result<T, ApiError> {
    input.map(|Json(value)| value).map_err(|rejection| {
        ApiError::bad_request(format!("invalid body: {}", rejection.body_text()))
    })
}

fn command_id(raw: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw).map_err(|_| ApiError::bad_request("invalid command id"))
}

/// `POST /fleet-executor/commands/claim` body: a game runtime names its open runtime session.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimRequest {
    #[serde(default)]
    runtime_session_id: Option<Uuid>,
}

/// Answers the claimed command, or 204 when nothing is claimable now.
///
/// @route POST /api/v1/fleet-executor/commands/claim
pub async fn claim_fleet_command(
    State(state): State<AppState>,
    caller: MachineCaller,
    request: Result<Json<ClaimRequest>, JsonRejection>,
) -> Result<Response, ApiError> {
    let request = body(request)?;
    let mut transaction = state.pool.begin().await?;
    let claimed = claim_next_command(
        &mut transaction,
        &caller,
        request.runtime_session_id,
        &state.cfg.discord_guild_id,
    )
    .await?;
    transaction.commit().await?;
    Ok(match claimed {
        Some(command) => Json(command).into_response(),
        None => StatusCode::NO_CONTENT.into_response(),
    })
}

/// @route POST /api/v1/fleet-executor/commands/:commandId/executing
pub async fn start_fleet_command(
    State(state): State<AppState>,
    caller: MachineCaller,
    Path(command): Path<String>,
    request: Result<Json<ExecutionStart>, JsonRejection>,
) -> Result<Json<FleetCommandReceipt>, ApiError> {
    let command = command_id(&command)?;
    let request = body(request)?;
    let mut transaction = state.pool.begin().await?;
    let receipt = mark_executing(&mut transaction, &caller, command, request.fencing_token).await?;
    transaction.commit().await?;
    Ok(Json(receipt))
}

/// @route POST /api/v1/fleet-executor/commands/:commandId/result
pub async fn finish_fleet_command(
    State(state): State<AppState>,
    caller: MachineCaller,
    Path(command): Path<String>,
    request: Result<Json<ExecutionResult>, JsonRejection>,
) -> Result<Json<FleetCommandReceipt>, ApiError> {
    let command = command_id(&command)?;
    let request = body(request)?;
    let mut transaction = state.pool.begin().await?;
    let receipt = record_result(&mut transaction, &caller, command, &request).await?;
    transaction.commit().await?;
    Ok(Json(receipt))
}
