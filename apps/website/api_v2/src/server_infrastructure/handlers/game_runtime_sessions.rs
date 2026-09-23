//! Game-runtime session routes: a booting runtime starts the next generation of its server's
//! session, and a stopping runtime ends it. Both require a `mod_runtime` machine credential and
//! act only on the credential's own server.

use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::server_infrastructure::models::machine_credential::ExecutorKind;
use crate::server_infrastructure::services::machine_credentials::MachineCaller;
use crate::server_infrastructure::services::runtime_sessions::{
    LoadedArtifactReport, StartedRuntimeSession, end_runtime_session, start_runtime_session,
};

/// A runtime that loaded a mission artifact reports it (`{loaded_artifact_id,
/// loaded_artifact_sha256}`); an empty body starts a session that runs no artifact.
///
/// @route POST /api/v1/game-runtime/sessions
pub async fn start_server_runtime_session(
    State(state): State<AppState>,
    caller: MachineCaller,
    body: Bytes,
) -> Result<(StatusCode, Json<StartedRuntimeSession>), ApiError> {
    caller.require_executor(ExecutorKind::ModRuntime)?;
    let report: LoadedArtifactReport = if body.iter().all(u8::is_ascii_whitespace) {
        LoadedArtifactReport::default()
    } else {
        serde_json::from_slice(&body).map_err(|_| {
            ApiError::bad_request(
                "the body is {loaded_artifact_id, loaded_artifact_sha256}, or empty",
            )
        })?
    };
    let mut transaction = state.pool.begin().await?;
    let started = start_runtime_session(&mut transaction, &caller, &report).await?;
    transaction.commit().await?;
    Ok((StatusCode::CREATED, Json(started)))
}

/// @route POST /api/v1/game-runtime/sessions/:sessionId/end
pub async fn end_server_runtime_session(
    State(state): State<AppState>,
    caller: MachineCaller,
    Path(session): Path<String>,
) -> Result<Json<Value>, ApiError> {
    caller.require_executor(ExecutorKind::ModRuntime)?;
    let session = Uuid::parse_str(&session)
        .map_err(|_| ApiError::bad_request("invalid runtime session id"))?;
    let mut transaction = state.pool.begin().await?;
    let end_reason = end_runtime_session(&mut transaction, &caller, session).await?;
    transaction.commit().await?;
    Ok(Json(
        json!({ "runtime_session_id": session, "ended": true, "end_reason": end_reason }),
    ))
}
