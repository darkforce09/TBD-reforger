//! A page of a match's stored events, in sequence order.
//!
//! **Role:** the read shape of `GET /api/v1/matches/{matchId}/events`.
//! **Position:** filled by `handlers::match_event_reads` from `match_events`.
//! **Signals & state:** none.
//! **Invariants:** items are in ascending `sequence`; `next_after_sequence` is the last item's
//! sequence when another page may follow and null on the last page; the payload is the stored
//! `jsonb` emitted verbatim.
//! @contract match-telemetry.schema.json#

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::core::wire_format::{RawJson, rfc3339_utc};

/// One stored event as the read route answers it.
/// @contract match-telemetry.schema.json#/definitions/MatchEvent
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct StoredMatchEvent {
    pub event_id: String,
    pub sequence: i64,
    pub kind: String,
    pub mission_time_ms: i64,
    #[serde(with = "rfc3339_utc")]
    pub occurred_at: DateTime<Utc>,
    pub payload: RawJson,
}

/// A page of events.
#[derive(Debug, Serialize)]
pub struct MatchEventPage {
    pub items: Vec<StoredMatchEvent>,
    pub next_after_sequence: Option<i64>,
}
