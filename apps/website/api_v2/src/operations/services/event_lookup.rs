//! The canonical single-row reads for an event and an event mission, shared by every handler
//! that starts from a path id.

use sqlx::PgPool;
use uuid::Uuid;

use crate::core::error_handling::api_error::ApiError;
use crate::operations::models::{Event, EventMission};
use crate::operations::services::event_status_rules::{EVENT_COLUMNS, sql};

/// Load one event. `status` is the **effective** status ([`EVENT_COLUMNS`]), so every
/// caller — the hub, the roster, and the transition check in `update_event` — reasons
/// about where the event is *now*, not where the last write left the column.
pub(crate) async fn load_event(pool: &PgPool, id: &str) -> Result<Event, ApiError> {
    let Ok(id) = Uuid::parse_str(id) else {
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
    let Ok(id) = Uuid::parse_str(emid) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    sqlx::query_as("SELECT id, event_id, mission_id, start_time, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM event_missions WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::not_found("mission not found"))
}
