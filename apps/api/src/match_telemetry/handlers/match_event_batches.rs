//! `POST /api/v1/ingest/match-events` — a batch of a registered match's detailed events.

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::response::Json;
use fleet_wire_contract::ExecutorKind;
use serde_json::Value;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::match_telemetry::models::match_event::{MatchEventBatchAnswer, decode_event_batch};
use crate::match_telemetry::services::match_event_batches::ingest_event_batch;
use crate::server_infrastructure::services::machine_credentials::MachineCaller;

/// Validate the whole batch, then store its new events and count them once.
/// @route POST /api/v1/ingest/match-events
pub async fn ingest_match_events(
    State(state): State<AppState>,
    caller: MachineCaller,
    body: Result<Json<Value>, JsonRejection>,
) -> Result<Json<MatchEventBatchAnswer>, ApiError> {
    caller.require_executor(ExecutorKind::ModRuntime)?;
    let Json(body) = body.map_err(ApiError::from_json_rejection)?;
    let batch = decode_event_batch(&body)?;
    Ok(Json(ingest_event_batch(&state, &caller, &batch).await?))
}
