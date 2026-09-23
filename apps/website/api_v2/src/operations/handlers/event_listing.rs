//! The calendar list with its per-event fill decoration.
//!
//! It filters and reports the EFFECTIVE status ([`crate::operations::services::event_status_rules`]),
//! so an operation that started while the convergence sweep was between ticks is listed as
//! `live` rather than as whatever the stored column still says. Non-administrators see only the
//! events their access admits; totals and pages count only those events, and an event visible
//! through child policies alone is decorated from the seats the viewer may see.

use std::collections::{BTreeMap, HashMap, HashSet};

use axum::extract::{Query, State};
use axum::response::Json;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::http::pagination::PageParams;
use crate::core::middleware::AuthUser;
use crate::operations::models::Event;
use crate::operations::services::event_access::visibility::{EventVisibility, viewer_event_access};
use crate::operations::services::event_status_rules::{EFFECTIVE_STATUS_SQL, EVENT_COLUMNS, sql};

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

#[derive(Debug, Deserialize)]
pub struct EventListQuery {
    scope: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

/// The scope's filter and order. Scope words are a hardcoded whitelist, never bound text.
/// The `upcoming` filter tests the EFFECTIVE status, so an operation that started while the
/// sweep was between ticks is still listed as live, and one past its end horizon drops off.
fn scope_sql(scope: Option<&str>) -> (String, &'static str) {
    match scope.unwrap_or("upcoming") {
        "past" => (
            "e.start_time <= now()".to_owned(),
            "e.start_time DESC, e.id",
        ),
        "all" => ("true".to_owned(), "e.start_time ASC, e.id"),
        _ => (
            format!(
                "(e.start_time > now() OR ({})::text = 'live')",
                &*EFFECTIVE_STATUS_SQL
            ),
            "e.start_time ASC, e.id",
        ),
    }
}

/// `GET /api/v1/events` — Upcoming/Calendar list.
///
/// @route GET /api/v1/events
pub async fn list_events(
    State(state): State<AppState>,
    user: AuthUser,
    Query(q): Query<EventListQuery>,
) -> Result<Json<Value>, ApiError> {
    let (limit, offset) = PageParams {
        limit: q.limit,
        offset: q.offset,
    }
    .bounds();
    let (filter, order) = scope_sql(q.scope.as_deref());
    let mut tx = state.pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await?;
    let candidates: Vec<Uuid> = sqlx::query_scalar(sql(format!(
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
    let visible: Vec<Uuid> = candidates
        .into_iter()
        .filter(|id| access.get(id).is_some_and(|a| a.visibility.is_visible()))
        .collect();
    let total = visible.len() as i64;
    let page: Vec<Uuid> = visible
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
    let visibility: BTreeMap<Uuid, EventVisibility> = access
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
    visibility: &BTreeMap<Uuid, EventVisibility>,
) -> Result<Vec<EventListItem>, ApiError> {
    let event_ids: Vec<Uuid> = events.iter().map(|e| e.id).collect();
    let slots: Vec<(Uuid, Uuid, Uuid, bool)> = sqlx::query_as(
        "SELECT m.event_id, s.event_mission_id, s.id, s.assigned_to IS NOT NULL
         FROM orbat_slots s JOIN event_missions m ON m.id = s.event_mission_id
         WHERE m.deleted_at IS NULL AND m.event_id = ANY($1)",
    )
    .bind(&event_ids)
    .fetch_all(&mut *connection)
    .await?;
    let attachments: Vec<(Uuid, Uuid)> = sqlx::query_as(
        "SELECT id, event_id FROM event_missions WHERE deleted_at IS NULL AND event_id = ANY($1)",
    )
    .bind(&event_ids)
    .fetch_all(&mut *connection)
    .await?;
    let registrations: Vec<(Uuid, i64)> = sqlx::query_as(
        "SELECT r.event_mission_id, count(*) FROM event_registrations r
         JOIN event_missions m ON m.id = r.event_mission_id
         WHERE m.deleted_at IS NULL AND m.event_id = ANY($1)
           AND r.reservation_state::text IN ('registered', 'legacy_unknown')
         GROUP BY r.event_mission_id",
    )
    .bind(&event_ids)
    .fetch_all(&mut *connection)
    .await?;
    let shown = |event: &Uuid, slot: &Uuid| {
        visibility
            .get(event)
            .is_some_and(|visibility| visibility.shows_slot(*slot))
    };
    let mut shown_missions: HashMap<Uuid, HashSet<Uuid>> = HashMap::new();
    let mut total_by_event: HashMap<Uuid, i64> = HashMap::new();
    let mut filled_by_event: HashMap<Uuid, i64> = HashMap::new();
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
    let event_of: HashMap<Uuid, Uuid> = attachments.iter().copied().collect();
    let mut registered_by_event: HashMap<Uuid, i64> = HashMap::new();
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
