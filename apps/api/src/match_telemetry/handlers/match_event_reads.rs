//! `GET /api/v1/matches/{matchId}/events` — a match's detailed events, in sequence order.

use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::response::Json;
use serde::Deserialize;
use uuid::Uuid;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::http::path_parameters::PathParams;
use crate::core::middleware::AuthUser;
use crate::match_telemetry::models::match_event_page::{MatchEventPage, StoredMatchEvent};

const DEFAULT_PAGE: i64 = 100;
const MAX_PAGE: i64 = 500;

/// Paging of the event read: events after `after_sequence`, at most `limit`.
#[derive(Debug, Deserialize)]
pub struct EventPageQuery {
    after_sequence: Option<i64>,
    limit: Option<i64>,
}

/// One page of the match's events; 404 for an unknown match.
/// @route GET /api/v1/matches/:matchId/events
pub async fn list_match_events(
    State(state): State<AppState>,
    _user: AuthUser,
    PathParams(match_id): PathParams<Uuid>,
    query: Result<Query<EventPageQuery>, QueryRejection>,
) -> Result<Json<MatchEventPage>, ApiError> {
    let Query(query) = query
        .map_err(|rejection| ApiError::from_query_rejection(rejection, "match event page query"))?;
    let limit = query.limit.unwrap_or(DEFAULT_PAGE);
    if !(1..=MAX_PAGE).contains(&limit) {
        return Err(ApiError::bad_request(format!(
            "limit must be between 1 and {MAX_PAGE}"
        )));
    }
    let after = query.after_sequence.unwrap_or(0);
    if after < 0 {
        return Err(ApiError::bad_request("after_sequence must not be negative"));
    }
    let exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM matches WHERE id = $1)")
        .bind(match_id)
        .fetch_one(&state.pool)
        .await?;
    if !exists {
        return Err(ApiError::not_found("match not found"));
    }
    let mut items: Vec<StoredMatchEvent> = sqlx::query_as(
        "SELECT event_id, sequence, kind, mission_time_ms, occurred_at, payload
         FROM match_events WHERE match_id = $1 AND sequence > $2
         ORDER BY sequence LIMIT $3",
    )
    .bind(match_id)
    .bind(after)
    .bind(limit + 1)
    .fetch_all(&state.pool)
    .await?;
    let next_after_sequence = if items.len() as i64 > limit {
        items.truncate(limit as usize);
        items.last().map(|item| item.sequence)
    } else {
        None
    };
    Ok(Json(MatchEventPage {
        items,
        next_after_sequence,
    }))
}
