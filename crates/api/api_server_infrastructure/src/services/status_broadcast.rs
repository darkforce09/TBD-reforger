//! The `server:{id}` SSE topic: the one query that loads live server rows, and the one
//! serialization that puts them on the wire.
//!
//! Ingest publishes in-request and the scheduled publisher republishes on an interval; both go
//! through [`publish_server_status`], so every consumer of the topic sees one payload shape.

use api_identifiers::ServerId;
use sqlx::PgPool;

use crate::models::server::{ServerStatus, ServerStatusRow, server_status_columns};
use api_http_layer::realtime_hub::Hub;

/// SQL that both the SSE snapshot and the scheduled publisher use — one shape, one cast.
/// One server's status by `server_id` (`$1`).
pub(crate) const SELECT_SERVER_STATUS: &str = concat!(
    "SELECT ",
    server_status_columns!(),
    " FROM server_statuses s WHERE s.server_id = $1"
);
/// The configured fleet's statuses (active servers only): the statement the scheduled
/// publisher and the command center's fleet overview both read.
pub const SELECT_FLEET_STATUSES: &str = concat!(
    "SELECT ",
    server_status_columns!(),
    " FROM server_statuses s JOIN servers ON servers.id = s.server_id WHERE servers.is_active"
);

/// Serialize `status` and fan it out on `server:{id}` — the exact bytes ingest and the
/// scheduled publisher both put on the wire (and that the SSE handler's snapshot matches).
pub fn publish_server_status(hub: &Hub, status: &ServerStatus) {
    if let Ok(payload) = serde_json::to_vec(status) {
        hub.publish(&format!("server:{}", status.server_id), payload);
    }
}

/// Load one server's status row and publish it; `false` when the server has no status row yet.
pub async fn publish_server_status_by_id(
    pool: &PgPool,
    hub: &Hub,
    server_id: ServerId,
) -> Result<bool, sqlx::Error> {
    let row: Option<ServerStatus> = sqlx::query_as::<_, ServerStatusRow>(SELECT_SERVER_STATUS)
        .bind(server_id)
        .fetch_optional(pool)
        .await?
        .map(ServerStatus::from);
    if let Some(status) = &row {
        publish_server_status(hub, status);
    }
    Ok(row.is_some())
}

/// Load the status row of every active server (the configured fleet) and publish each to its
/// SSE topic; inactive servers are not republished.
///
/// Failures are returned to the caller (the scheduler logs and retries next tick). An empty
/// table is success with zero publishes — there is nothing to fan out until ingest (or a
/// seed) writes a row.
pub async fn publish_all_server_statuses(pool: &PgPool, hub: &Hub) -> Result<usize, sqlx::Error> {
    let rows: Vec<ServerStatus> = sqlx::query_as::<_, ServerStatusRow>(SELECT_FLEET_STATUSES)
        .fetch_all(pool)
        .await?
        .into_iter()
        .map(ServerStatus::from)
        .collect();
    let n = rows.len();
    for status in &rows {
        publish_server_status(hub, status);
    }
    Ok(n)
}

#[cfg(test)]
#[path = "tests/status_broadcast.rs"]
mod tests;
