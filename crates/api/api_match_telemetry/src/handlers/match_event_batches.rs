//! `POST /api/v1/ingest/match-events` — a batch of a registered match's detailed events.

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::response::Json;
use fleet_wire_contract::ExecutorKind;
use serde_json::Value;

use crate::models::match_event::{MatchEventBatchAnswer, decode_event_batch};
use crate::services::match_event_batches::ingest_event_batch;
use api_caller_identity::machine_caller::MachineCaller;
use api_foundation::error_handling::api_error::ApiError;
use api_state::AppState;

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
