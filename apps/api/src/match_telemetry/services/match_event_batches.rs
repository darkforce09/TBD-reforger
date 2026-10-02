//! The transaction that stores one batch of detailed events and counts the new ones.
//!
//! **Role:** refuses a batch that contradicts stored events, inserts the rest idempotently and
//! increments the per-identity totals and the match's event count by exactly the rows inserted.
//! **Position:** called by `handlers::match_event_batches` with the machine caller and a
//! validated [`MatchEventBatch`].
//! **Signals & state:** one transaction per call.
//! **Invariants:** the match row lock serializes batches of one match; a stored `event_id` with
//! another digest (409 `EVENT_CONFLICT`) or a stored `sequence` under another `event_id`
//! (409 `EVENT_SEQUENCE_CONFLICT`) refuses the whole batch before any write; only rows the insert
//! returns are counted, so a retried or overlapping batch never counts an event twice; events of
//! a finalized match are accepted.

use std::collections::BTreeMap;

use serde_json::json;
use sqlx::PgConnection;

use super::registered_match::lock_registered_match;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::match_telemetry::models::match_event::{
    IncomingEvent, MatchEventBatch, MatchEventBatchAnswer,
};
use crate::match_telemetry::models::telemetry_refusal::telemetry_conflict;
use crate::server_infrastructure::services::machine_credentials::MachineCaller;

/// Refusal code of an event id stored with another event.
pub const EVENT_CONFLICT: &str = "EVENT_CONFLICT";
/// Refusal code of a sequence stored under another event id.
pub const EVENT_SEQUENCE_CONFLICT: &str = "EVENT_SEQUENCE_CONFLICT";

/// Refuse the batch when an event contradicts what the match already stores.
async fn refuse_contradictions(
    connection: &mut PgConnection,
    match_id: uuid::Uuid,
    batch: &MatchEventBatch,
) -> Result<(), ApiError> {
    let ids: Vec<&str> = batch
        .events
        .iter()
        .map(|event| event.event_id.as_str())
        .collect();
    let sequences: Vec<i64> = batch.events.iter().map(|event| event.sequence).collect();
    let stored: Vec<(String, i64, String)> = sqlx::query_as(
        "SELECT event_id, sequence, payload_sha256 FROM match_events
         WHERE match_id = $1 AND (event_id = ANY($2) OR sequence = ANY($3))",
    )
    .bind(match_id)
    .bind(&ids)
    .bind(&sequences)
    .fetch_all(connection)
    .await?;
    for (index, event) in batch.events.iter().enumerate() {
        for (stored_id, stored_sequence, stored_sha256) in &stored {
            if *stored_id == event.event_id
                && (*stored_sha256 != event.payload_sha256 || *stored_sequence != event.sequence)
            {
                return Err(telemetry_conflict(
                    EVENT_CONFLICT,
                    format!("event {} is stored with other content", event.event_id),
                    json!({ "match_id": match_id, "index": index, "event_id": event.event_id }),
                ));
            }
            if *stored_sequence == event.sequence && *stored_id != event.event_id {
                return Err(telemetry_conflict(
                    EVENT_SEQUENCE_CONFLICT,
                    format!("sequence {} belongs to another event", event.sequence),
                    json!({ "match_id": match_id, "index": index, "sequence": event.sequence }),
                ));
            }
        }
    }
    Ok(())
}

async fn insert_event(
    connection: &mut PgConnection,
    match_id: uuid::Uuid,
    event: &IncomingEvent,
) -> Result<bool, ApiError> {
    let inserted: Option<String> = sqlx::query_scalar(
        "INSERT INTO match_events (match_id, event_id, sequence, kind, mission_time_ms,
                                   occurred_at, actor_arma_id, subject_arma_id, payload,
                                   payload_sha256)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
         ON CONFLICT DO NOTHING RETURNING event_id",
    )
    .bind(match_id)
    .bind(&event.event_id)
    .bind(event.sequence)
    .bind(event.kind)
    .bind(event.mission_time_ms)
    .bind(event.occurred_at)
    .bind(&event.actor_arma_id)
    .bind(&event.subject_arma_id)
    .bind(&event.payload)
    .bind(&event.payload_sha256)
    .fetch_optional(connection)
    .await?;
    Ok(inserted.is_some())
}

/// Store `batch` for the match the caller's server registered.
pub async fn ingest_event_batch(
    state: &AppState,
    caller: &MachineCaller,
    batch: &MatchEventBatch,
) -> Result<MatchEventBatchAnswer, ApiError> {
    let mut tx = state.pool.begin().await?;
    let stored = lock_registered_match(&mut tx, caller.server_id, &batch.source_match_id).await?;
    refuse_contradictions(&mut tx, stored.id, batch).await?;

    let mut accepted: i64 = 0;
    let mut totals: BTreeMap<(String, &'static str, &'static str), i64> = BTreeMap::new();
    for event in &batch.events {
        if !insert_event(&mut tx, stored.id, event).await? {
            continue;
        }
        accepted += 1;
        for (identity, role) in [
            (&event.actor_arma_id, "actor"),
            (&event.subject_arma_id, "subject"),
        ] {
            if let Some(identity) = identity {
                *totals
                    .entry((identity.clone(), event.kind, role))
                    .or_default() += 1;
            }
        }
    }
    for ((arma_id, kind, role), count) in &totals {
        sqlx::query(
            "INSERT INTO match_event_totals (match_id, arma_id, kind, participant_role, event_count)
             VALUES ($1, $2, $3, $4, $5)
             ON CONFLICT (match_id, arma_id, kind, participant_role)
             DO UPDATE SET event_count = match_event_totals.event_count + EXCLUDED.event_count",
        )
        .bind(stored.id)
        .bind(arma_id)
        .bind(kind)
        .bind(role)
        .bind(count)
        .execute(&mut *tx)
        .await?;
    }
    let (event_count, last_sequence): (i64, Option<i64>) = sqlx::query_as(
        "UPDATE matches SET event_count = event_count + $2 WHERE id = $1
         RETURNING event_count, (SELECT max(sequence) FROM match_events WHERE match_id = $1)",
    )
    .bind(stored.id)
    .bind(accepted)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(MatchEventBatchAnswer {
        match_id: stored.id,
        accepted,
        duplicates: batch.events.len() as i64 - accepted,
        event_count,
        last_sequence,
    })
}
