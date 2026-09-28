//! `POST /api/v1/ingest/matches` — a game runtime registers a source match before reporting
//! about it.

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::Json;
use serde_json::Value;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::match_telemetry::models::match_registration::{
    MatchRegistrationAnswer, decode_registration,
};
use crate::match_telemetry::services::match_registration::register_match;
use crate::server_infrastructure::models::machine_credential::ExecutorKind;
use crate::server_infrastructure::services::machine_credentials::MachineCaller;

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
