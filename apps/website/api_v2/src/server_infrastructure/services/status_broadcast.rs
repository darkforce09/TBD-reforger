//! The `server:{id}` SSE topic: the one query that loads live server rows, and the one
//! serialization that puts them on the wire.
//!
//! Ingest publishes in-request and the scheduled publisher republishes on an interval; both go
//! through [`publish_server_status`], so every consumer of the topic sees one payload shape.

use sqlx::PgPool;

use crate::core::realtime_hub::Hub;
use crate::server_infrastructure::models::server::ServerStatus;

/// SQL that both the SSE snapshot and the scheduled publisher use — one shape, one cast.
const SELECT_SERVER_STATUSES: &str = "SELECT server_id, is_online, player_count, max_players, \
     server_fps::float8 AS server_fps, uptime_seconds, current_match_id, \
     COALESCE(ingame_time, '') AS ingame_time, COALESCE(ingame_weather, '') AS ingame_weather, \
     COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at \
     FROM server_statuses";

/// Serialize `status` and fan it out on `server:{id}` — the exact bytes ingest and the
/// scheduled publisher both put on the wire (and that the SSE handler's snapshot matches).
pub fn publish_server_status(hub: &Hub, status: &ServerStatus) {
    if let Ok(payload) = serde_json::to_vec(status) {
        hub.publish(&format!("server:{}", status.server_id), payload);
    }
}

/// Load every `server_statuses` row and publish each to its SSE topic.
///
/// Failures are returned to the caller (the scheduler logs and retries next tick). An empty
/// table is success with zero publishes — there is nothing to fan out until ingest (or a
/// seed) writes a row.
pub async fn publish_all_server_statuses(pool: &PgPool, hub: &Hub) -> Result<usize, sqlx::Error> {
    let rows: Vec<ServerStatus> = sqlx::query_as(SELECT_SERVER_STATUSES)
        .fetch_all(pool)
        .await?;
    let n = rows.len();
    for status in &rows {
        publish_server_status(hub, status);
    }
    Ok(n)
}

#[cfg(test)]
#[path = "tests/status_broadcast.rs"]
mod tests;
