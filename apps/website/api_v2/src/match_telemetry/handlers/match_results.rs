//! `POST /api/v1/ingest/match-results` — one revision of a registered match's results report.

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::response::Json;
use serde_json::Value;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::match_telemetry::models::match_results_revision::{
    MatchResultsAnswer, decode_results_revision,
};
use crate::match_telemetry::services::match_results_ingest::ingest_results_revision;
use crate::server_infrastructure::models::machine_credential::ExecutorKind;
use crate::server_infrastructure::services::machine_credentials::MachineCaller;

/// Validate the whole revision, then decide and apply it against the caller's registered match.
/// Unowned identities keep their gameplay facts and are named in the answer and the audit log.
/// @route POST /api/v1/ingest/match-results
pub async fn ingest_match_results(
    State(state): State<AppState>,
    caller: MachineCaller,
    body: Result<Json<Value>, JsonRejection>,
) -> Result<Json<MatchResultsAnswer>, ApiError> {
    caller.require_executor(ExecutorKind::ModRuntime)?;
    let Json(body) = body.map_err(|_| ApiError::bad_request("the body must be a JSON object"))?;
    let revision = decode_results_revision(&body)?;
    Ok(Json(
        ingest_results_revision(&state, &caller, &revision).await?,
    ))
}
