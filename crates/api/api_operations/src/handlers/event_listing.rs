//! The calendar list with its per-event fill decoration.
//!
//! It filters and reports the EFFECTIVE status ([`crate::services::event_status_rules`]),
//! so an operation that started while the convergence sweep was between ticks is listed as
//! `live` rather than as whatever the stored column still says. Non-administrators see only the
//! events their access admits; totals and pages count only those events, and an event visible
//! through child policies alone is decorated from the seats the viewer may see.
//!
//! @contract event-schedule.schema.json#/definitions/EventListPage
//! @contract event-schedule.schema.json#/definitions/EventListItem

use api_identifiers::{EventId, EventMissionId, OrbatSlotId};
use std::collections::{BTreeMap, HashMap, HashSet};

use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::response::Json;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::PgConnection;

use crate::models::Event;
use crate::services::event_access::visibility::{EventVisibility, viewer_event_access};
use crate::services::event_status_rules::{EFFECTIVE_STATUS_SQL, EVENT_COLUMNS, sql};
use api_foundation::error_handling::api_error::ApiError;
use api_foundation::http::pagination::PageParams;
use api_http_layer::middleware::AuthUser;
use api_state::AppState;

/// One row of `GET /api/v1/events`: the event with its mission count and seat totals.
#[derive(Debug, Serialize)]
pub struct EventListItem {
    #[serde(flatten)]
    event: Event,
    mission_count: i64,
    registered: i64,
    filled: i64,
    total_slots: i64,
    percent: i64,
}

/// The `scope` query word of the event list; an absent scope is `upcoming`, and any other word
/// fails the query decode, which answers 400.
#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventListScope {
    /// Events still to start, and those running now.
    #[default]
    Upcoming,
    /// Events that have started, newest first.
    Past,
    /// Every event, oldest first.
    All,
}

/// The query of `GET /api/v1/events`: the time scope and the page.
#[derive(Debug, Deserialize)]
pub struct EventListQuery {
    #[serde(default)]
    scope: EventListScope,
    limit: Option<i64>,
    offset: Option<i64>,
}

/// The scope's filter and order; the SQL is fixed per scope, never bound text.
/// The `upcoming` filter tests the EFFECTIVE status, so an operation that started while the
/// sweep was between ticks is still listed as live, and one past its end horizon drops off.
fn scope_sql(scope: EventListScope) -> (String, &'static str) {
    match scope {
        EventListScope::Past => (
            "e.start_time <= now()".to_owned(),
            "e.start_time DESC, e.id",
        ),
        EventListScope::All => ("true".to_owned(), "e.start_time ASC, e.id"),
        EventListScope::Upcoming => (
            format!(
                "(e.start_time > now() OR ({})::text = 'live')",
                &*EFFECTIVE_STATUS_SQL
            ),
            "e.start_time ASC, e.id",
        ),
    }
}

/// `GET /api/v1/events` — Upcoming/Calendar list; an unknown `scope` answers 400.
///
/// @route GET /api/v1/events
pub async fn list_events(
    State(state): State<AppState>,
    user: AuthUser,
    query: Result<Query<EventListQuery>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let Query(q) =
        query.map_err(|rejection| ApiError::from_query_rejection(rejection, "event list query"))?;
    let (limit, offset) = PageParams {
        limit: q.limit,
        offset: q.offset,
    }
    .bounds();
    let (filter, order) = scope_sql(q.scope);
    let mut tx = state.pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await?;
    let candidates: Vec<EventId> = sqlx::query_scalar(sql(format!(
        "SELECT e.id FROM events e WHERE e.deleted_at IS NULL AND {filter} ORDER BY {order}"
    )))
    .fetch_all(&mut *tx)
    .await?;
    let access = viewer_event_access(
        &mut tx,
        &user.discord_id,
        user.role == "admin",
        &candidates,
        &state.cfg.discord_guild_id,
    )
    .await?;
    let visible: Vec<EventId> = candidates
        .into_iter()
        .filter(|id| access.get(id).is_some_and(|a| a.visibility.is_visible()))
        .collect();
    let total = visible.len() as i64;
    let page: Vec<EventId> = visible
        .iter()
        .skip(offset.max(0) as usize)
        .take(limit.max(0) as usize)
        .copied()
        .collect();
    let mut events: Vec<Event> = sqlx::query_as(sql(format!(
        "SELECT {} FROM events e WHERE e.id = ANY($1)",
        &*EVENT_COLUMNS
    )))
    .bind(&page)
    .fetch_all(&mut *tx)
    .await?;
    events.sort_by_key(|event| page.iter().position(|id| *id == event.id));
    let visibility: BTreeMap<EventId, EventVisibility> = access
        .into_iter()
        .map(|(id, access)| (id, access.visibility))
        .collect();
    let data = decorate_events(&mut tx, events, &visibility).await?;
    tx.commit().await?;
    Ok(Json(
        json!({ "data": data, "total": total, "limit": limit, "offset": offset }),
    ))
}

/// Batch-load mission counts, registration counts and ORBAT fill per event, counting only the
/// seats and missions each event's visibility shows.
async fn decorate_events(
    connection: &mut PgConnection,
    events: Vec<Event>,
    visibility: &BTreeMap<EventId, EventVisibility>,
) -> Result<Vec<EventListItem>, ApiError> {
    let event_ids: Vec<EventId> = events.iter().map(|e| e.id).collect();
    let slots: Vec<(EventId, EventMissionId, OrbatSlotId, bool)> = sqlx::query_as(
        "SELECT m.event_id, s.event_mission_id, s.id, s.assigned_to IS NOT NULL
         FROM orbat_slots s JOIN event_missions m ON m.id = s.event_mission_id
         WHERE m.deleted_at IS NULL AND m.event_id = ANY($1)",
    )
    .bind(&event_ids)
    .fetch_all(&mut *connection)
    .await?;
    let attachments: Vec<(EventMissionId, EventId)> = sqlx::query_as(
        "SELECT id, event_id FROM event_missions WHERE deleted_at IS NULL AND event_id = ANY($1)",
    )
    .bind(&event_ids)
    .fetch_all(&mut *connection)
    .await?;
    let registrations: Vec<(EventMissionId, i64)> = sqlx::query_as(
        "SELECT r.event_mission_id, count(*) FROM event_registrations r
         JOIN event_missions m ON m.id = r.event_mission_id
         WHERE m.deleted_at IS NULL AND m.event_id = ANY($1)
           AND r.reservation_state::text IN ('registered', 'legacy_unknown')
         GROUP BY r.event_mission_id",
    )
    .bind(&event_ids)
    .fetch_all(&mut *connection)
    .await?;
    let shown = |event: &EventId, slot: &OrbatSlotId| {
        visibility
            .get(event)
            .is_some_and(|visibility| visibility.shows_slot(*slot))
    };
    let mut shown_missions: HashMap<EventId, HashSet<EventMissionId>> = HashMap::new();
    let mut total_by_event: HashMap<EventId, i64> = HashMap::new();
    let mut filled_by_event: HashMap<EventId, i64> = HashMap::new();
    for (event, mission, slot, filled) in &slots {
        if shown(event, slot) {
            shown_missions.entry(*event).or_default().insert(*mission);
            *total_by_event.entry(*event).or_default() += 1;
            *filled_by_event.entry(*event).or_default() += i64::from(*filled);
        }
    }
    // A fully visible event counts every operational mission, seated or not.
    for (mission, event) in &attachments {
        if visibility.get(event) == Some(&EventVisibility::Full) {
            shown_missions.entry(*event).or_default().insert(*mission);
        }
    }
    let event_of: HashMap<EventMissionId, EventId> = attachments.iter().copied().collect();
    let mut registered_by_event: HashMap<EventId, i64> = HashMap::new();
    for (mission, count) in registrations {
        if let Some(event) = event_of.get(&mission)
            && shown_missions
                .get(event)
                .is_some_and(|missions| missions.contains(&mission))
        {
            *registered_by_event.entry(*event).or_default() += count;
        }
    }
    Ok(events
        .into_iter()
        .map(|e| {
            let total = total_by_event.get(&e.id).copied().unwrap_or(0);
            let filled = filled_by_event.get(&e.id).copied().unwrap_or(0);
            let percent = if total > 0 { filled * 100 / total } else { 0 };
            EventListItem {
                mission_count: shown_missions.get(&e.id).map_or(0, |m| m.len() as i64),
                registered: registered_by_event.get(&e.id).copied().unwrap_or(0),
                filled,
                total_slots: total,
                percent,
                event: e,
            }
        })
        .collect())
}
