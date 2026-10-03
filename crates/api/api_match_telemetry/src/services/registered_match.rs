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

use api_identifiers::{EventId, MatchId, MissionId, ServerId, SourceMatchId};
use serde_json::json;
use sqlx::PgConnection;

use crate::models::telemetry_refusal::telemetry_conflict;
use api_foundation::error_handling::api_error::ApiError;

/// Refusal code of a report about a match the server never registered.
pub const MATCH_NOT_REGISTERED: &str = "MATCH_NOT_REGISTERED";

/// The locked state of a registered match.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RegisteredMatch {
    /// The match's id.
    pub id: MatchId,
    /// The stored revision; 0 while no results revision applied.
    pub revision: i64,
    /// Digest of the stored revision, while one applied.
    pub report_sha256: Option<String>,
    /// Whether the match is finalized.
    pub finalized: bool,
    /// The event the match is attached to, when one.
    pub event_id: Option<EventId>,
    /// The mission the match is attached to, when one.
    pub mission_id: Option<MissionId>,
}

/// Lock the match `server_id` registered as `source_match_id`.
pub async fn lock_registered_match(
    connection: &mut PgConnection,
    server_id: ServerId,
    source_match_id: &SourceMatchId,
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
