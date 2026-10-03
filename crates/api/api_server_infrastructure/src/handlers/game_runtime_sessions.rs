//! Game-runtime session routes: a booting runtime starts the next generation of its server's
//! session, and a stopping runtime ends it. Both require a `mod_runtime` machine credential and
//! act only on the credential's own server.
//!
//! @contract game-runtime-session.schema.json#/definitions/RuntimeSessionEnd

use api_identifiers::RuntimeSessionId;
use axum::body::{Body, Bytes};
use axum::extract::rejection::BytesRejection;
use axum::extract::{FromRequest, Request, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::Json;
use fleet_wire_contract::ExecutorKind;
use serde_json::{Value, json};

use crate::services::runtime_sessions::{
    LoadedArtifactReport, StartedRuntimeSession, end_runtime_session, start_runtime_session,
};
use api_caller_identity::machine_caller::MachineCaller;
use api_foundation::error_handling::api_error::ApiError;
use api_foundation::http::path_parameters::PathParams;
use api_state::AppState;

/// A runtime that loaded a mission artifact reports it (`{loaded_artifact_id,
/// loaded_artifact_sha256}`); an empty body starts a session that runs no artifact. A body over
/// the request body limit, a non-empty body without a JSON content type and one that does not
/// decode answer through [`ApiError::from_json_rejection`] (413, 415, 400).
///
/// @route POST /api/v1/game-runtime/sessions
pub async fn start_server_runtime_session(
    State(state): State<AppState>,
    caller: MachineCaller,
    headers: HeaderMap,
    body: Result<Bytes, BytesRejection>,
) -> Result<(StatusCode, Json<StartedRuntimeSession>), ApiError> {
    caller.require_executor(ExecutorKind::ModRuntime)?;
    let body = body.map_err(|rejection| ApiError::from_json_rejection(rejection.into()))?;
    let report = if body.iter().all(u8::is_ascii_whitespace) {
        LoadedArtifactReport::default()
    } else {
        loaded_artifact_report(headers, body).await?
    };
    let mut transaction = state.pool.begin().await?;
    let started = start_runtime_session(&mut transaction, &caller, &report).await?;
    transaction.commit().await?;
    Ok((StatusCode::CREATED, Json(started)))
}

/// Decode a non-empty session-start body through the same [`Json`] extractor every JSON route
/// uses, so its content-type check and decode errors are the shared ones.
async fn loaded_artifact_report(
    headers: HeaderMap,
    body: Bytes,
) -> Result<LoadedArtifactReport, ApiError> {
    let mut request = Request::new(Body::from(body));
    *request.headers_mut() = headers;
    let Json(report) = Json::<LoadedArtifactReport>::from_request(request, &())
        .await
        .map_err(ApiError::from_json_rejection)?;
    Ok(report)
}

/// @route POST /api/v1/game-runtime/sessions/:sessionId/end
pub async fn end_server_runtime_session(
    State(state): State<AppState>,
    caller: MachineCaller,
    PathParams(session): PathParams<String>,
) -> Result<Json<Value>, ApiError> {
    caller.require_executor(ExecutorKind::ModRuntime)?;
    let session = session
        .parse::<RuntimeSessionId>()
        .map_err(|_| ApiError::bad_request("invalid runtime session id"))?;
    let mut transaction = state.pool.begin().await?;
    let end_reason = end_runtime_session(&mut transaction, &caller, session).await?;
    transaction.commit().await?;
    Ok(Json(
        json!({ "runtime_session_id": session, "ended": true, "end_reason": end_reason }),
    ))
}
