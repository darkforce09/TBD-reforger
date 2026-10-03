//! The canonical single-row reads for an event and an event mission, shared by every handler
//! that starts from a path id.

use api_identifiers::{EventId, EventMissionId};
use sqlx::PgPool;

use crate::models::{Event, EventMission};
use crate::services::event_status_rules::{EVENT_COLUMNS, sql};
use api_foundation::error_handling::api_error::ApiError;

/// Load one event. `status` is the **effective** status ([`EVENT_COLUMNS`]), so every
/// caller — the hub, the roster, and the transition check in `update_event` — reasons
/// about where the event is *now*, not where the last write left the column.
pub(crate) async fn load_event(pool: &PgPool, id: &str) -> Result<Event, ApiError> {
    let Ok(id) = id.parse::<EventId>() else {
        return Err(ApiError::bad_request("invalid id"));
    };
    sqlx::query_as(sql(format!(
        "SELECT {} FROM events e WHERE e.id = $1 AND e.deleted_at IS NULL",
        &*EVENT_COLUMNS
    )))
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| ApiError::not_found("event not found"))
}

pub(crate) async fn load_em(pool: &PgPool, emid: &str) -> Result<EventMission, ApiError> {
    let Ok(id) = emid.parse::<EventMissionId>() else {
        return Err(ApiError::bad_request("invalid id"));
    };
    sqlx::query_as("SELECT id, event_id, mission_id, start_time, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM event_missions WHERE deleted_at IS NULL AND id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::not_found("mission not found"))
}
