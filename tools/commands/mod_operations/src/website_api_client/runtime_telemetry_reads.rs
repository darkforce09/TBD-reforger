//! Reads of what the platform recorded about a game runtime's telemetry.
//!
//! **Role:** read a server's status card (its current match and the game runtime's telemetry
//! queue reading) and whether a match holds acknowledged events.
//!
//! **Position:** called by the playtest's telemetry check
//! (`playtest_server/telemetry_check.rs`) through an administrator's [`ApiClient`]; the answers
//! are `GET /api/v1/servers/{id}/status` and `GET /api/v1/matches/{matchId}/events?limit=1`.
//!
//! **Signals & state:** none; each read is one request.
//!
//! **Invariants:** a status card whose `status.telemetry_queue` is present carries all five
//! fields as the API writes them (`backlog`, `capacity`, `dropped_total` and
//! `oldest_age_seconds` non-negative integers, `reported_at` a timestamp string); any other shape
//! is an error, never a reading of zero. An empty `current_match_id` reads as no match.

use crate::error::{Result, ResultExt};
use serde_json::Value;

use super::api_client::ApiClient;
use super::http_exchange::encode_component;

/// The game runtime's telemetry queue as its last heartbeat reported it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TelemetryQueueReading {
    /// Entries waiting for the API's acknowledgement.
    pub backlog: u64,
    /// The queue's entry capacity.
    pub capacity: u64,
    /// Entries the queue dropped since the runtime's state began.
    pub dropped_total: u64,
    /// Age in seconds of the oldest waiting entry; 0 with an empty backlog.
    pub oldest_age_seconds: u64,
    /// When the API stored the reading (RFC 3339).
    pub reported_at: String,
}

/// The telemetry half of a server's status card.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct ServerTelemetryStatus {
    /// The match the server runs, when its status names one.
    pub(crate) current_match_id: Option<String>,
    /// The last queue reading; absent until the runtime reports one.
    pub telemetry_queue: Option<TelemetryQueueReading>,
}

/// `GET /api/v1/servers/{id}/status`, reduced to its telemetry fields.
pub(crate) fn server_telemetry_status(
    client: &ApiClient<'_>,
    server_id: &str,
) -> Result<ServerTelemetryStatus> {
    let card = client.expect_json(
        "GET",
        &format!("/api/v1/servers/{}/status", encode_component(server_id)),
        None,
        200,
    )?;
    telemetry_status_from_card(&card)
        .with_context(|| format!("the status card of server {server_id}"))
}

/// Whether `GET /api/v1/matches/{matchId}/events?limit=1` answers at least one event.
pub(crate) fn match_has_acknowledged_events(
    client: &ApiClient<'_>,
    match_id: &str,
) -> Result<bool> {
    let page = client.expect_json(
        "GET",
        &format!(
            "/api/v1/matches/{}/events?limit=1",
            encode_component(match_id)
        ),
        None,
        200,
    )?;
    let items = page
        .get("items")
        .and_then(Value::as_array)
        .with_context(|| format!("the event page of match {match_id} has no `items` array"))?;
    Ok(!items.is_empty())
}

/// The telemetry fields of a status card (`{…server, status: {current_match_id?,
/// telemetry_queue?} | null}`).
pub(super) fn telemetry_status_from_card(card: &Value) -> Result<ServerTelemetryStatus> {
    let Some(status) = card.get("status").filter(|status| !status.is_null()) else {
        return Ok(ServerTelemetryStatus::default());
    };
    let current_match_id = status
        .get("current_match_id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
        .map(str::to_string);
    let telemetry_queue = match status.get("telemetry_queue") {
        None | Some(Value::Null) => None,
        Some(block) => Some(queue_reading(block)?),
    };
    Ok(ServerTelemetryStatus {
        current_match_id,
        telemetry_queue,
    })
}

/// One `telemetry_queue` block; every field is required.
fn queue_reading(block: &Value) -> Result<TelemetryQueueReading> {
    let count = |key: &str| {
        block.get(key).and_then(Value::as_u64).with_context(|| {
            format!("telemetry_queue.{key} is not a non-negative integer: {block}")
        })
    };
    Ok(TelemetryQueueReading {
        backlog: count("backlog")?,
        capacity: count("capacity")?,
        dropped_total: count("dropped_total")?,
        oldest_age_seconds: count("oldest_age_seconds")?,
        reported_at: block
            .get("reported_at")
            .and_then(Value::as_str)
            .with_context(|| format!("telemetry_queue.reported_at is not a string: {block}"))?
            .to_string(),
    })
}
