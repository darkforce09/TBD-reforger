//! The row lock every report about a registered match starts with.
//!
//! **Role:** finds the match a server registered under a source match id and locks its row.
//! **Position:** first statement of the results and event-batch transactions
//! ([`super::match_results_ingest`], [`super::match_event_batches`]).
//! **Signals & state:** none; the lock lives in the caller's transaction.
//! **Invariants:** the lookup is scoped to the caller's server, so a server never reaches another
//! server's match; an unregistered source is the 409 `MATCH_NOT_REGISTERED`, never a 404 (the mod
//! treats 404 as permanent); the row is held `FOR NO KEY UPDATE`, which serializes reports about
//! one match without blocking foreign keys that reference it.

use serde_json::json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::core::error_handling::api_error::ApiError;
use crate::match_telemetry::models::telemetry_refusal::telemetry_conflict;

/// Refusal code of a report about a match the server never registered.
pub const MATCH_NOT_REGISTERED: &str = "MATCH_NOT_REGISTERED";

/// The locked state of a registered match.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RegisteredMatch {
    pub id: Uuid,
    pub revision: i64,
    pub report_sha256: Option<String>,
    pub finalized: bool,
    pub event_id: Option<Uuid>,
    pub mission_id: Option<Uuid>,
}

/// Lock the match `server_id` registered as `source_match_id`.
pub async fn lock_registered_match(
    connection: &mut PgConnection,
    server_id: Uuid,
    source_match_id: &str,
) -> Result<RegisteredMatch, ApiError> {
    let found: Option<RegisteredMatch> = sqlx::query_as(
        "SELECT id, revision, report_sha256, finalized_at IS NOT NULL AS finalized, event_id,
                mission_id
         FROM matches WHERE server_id = $1 AND source_match_id = $2 FOR NO KEY UPDATE",
    )
    .bind(server_id)
    .bind(source_match_id)
    .fetch_optional(connection)
    .await?;
    found.ok_or_else(|| {
        telemetry_conflict(
            MATCH_NOT_REGISTERED,
            "register the match with POST /api/v1/ingest/matches before reporting about it",
            json!({ "source_match_id": source_match_id }),
        )
    })
}
