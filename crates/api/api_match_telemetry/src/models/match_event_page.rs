//! A page of a match's stored events, in sequence order.
//!
//! **Role:** the read shape of `GET /api/v1/matches/{matchId}/events`.
//! **Position:** filled by `handlers::match_event_reads` from `match_events`.
//! **Signals & state:** none.
//! **Invariants:** items are in ascending `sequence`; `next_after_sequence` is the last item's
//! sequence when another page may follow and null on the last page; the payload is the stored
//! `jsonb` emitted verbatim.
//! @contract match-telemetry.schema.json#

use api_identifiers::MatchEventId;
use chrono::{DateTime, Utc};
use serde::Serialize;

use api_foundation::wire_format::RawJson;
use fleet_wire_contract::rfc3339_timestamps::rfc3339_utc;

/// One stored event as the read route answers it.
/// @contract match-telemetry.schema.json#/definitions/MatchEvent
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct StoredMatchEvent {
    /// The runtime's id of the event, unique within the match.
    pub event_id: MatchEventId,
    /// The event's position in the match's event order.
    pub sequence: i64,
    /// The event kind (`combat.kill`, `medical.revived`, ...).
    pub kind: String,
    /// Mission time of the event, in milliseconds.
    pub mission_time_ms: i64,
    /// When the event happened.
    #[serde(with = "rfc3339_utc")]
    pub occurred_at: DateTime<Utc>,
    /// The stored payload, emitted verbatim.
    pub payload: RawJson,
}

/// A page of events.
#[derive(Debug, Serialize)]
pub struct MatchEventPage {
    /// The events of this page, in ascending `sequence`.
    pub items: Vec<StoredMatchEvent>,
    /// The `sequence` to page after when another page may follow; `None` on the last page.
    pub next_after_sequence: Option<i64>,
}
