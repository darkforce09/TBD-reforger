//! `POST /api/v1/ingest/matches` — a game runtime registers a source match before reporting
//! about it.

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::Json;
use fleet_wire_contract::ExecutorKind;
use serde_json::Value;

use crate::models::match_registration::{MatchRegistrationAnswer, decode_registration};
use crate::services::match_registration::register_match;
use api_caller_identity::machine_caller::MachineCaller;
use api_foundation::error_handling::api_error::ApiError;
use api_state::AppState;

/// Register a server-scoped source match: 201 for a new one, 200 for a repeat of the same body.
/// @route POST /api/v1/ingest/matches
pub async fn ingest_match_registration(
    State(state): State<AppState>,
    caller: MachineCaller,
    body: Result<Json<Value>, JsonRejection>,
) -> Result<(StatusCode, Json<MatchRegistrationAnswer>), ApiError> {
    caller.require_executor(ExecutorKind::ModRuntime)?;
    let Json(body) = body.map_err(ApiError::from_json_rejection)?;
    let registration = decode_registration(&body)?;
    let answer = register_match(&state, &caller, &registration).await?;
    let status = if answer.registered {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((status, Json(answer)))
}
