//! Home dashboard aggregation — Rust port of `handlers/dashboard.go`. Many
//! best-effort, null-safe lookups composed into one response.

use axum::extract::State;
use axum::response::Json;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{Value, json};

use crate::error::ApiError;
use crate::handlers::deployments::mission_title_terrain;
use crate::handlers::modpacks::load_current_modpack;
use crate::middleware::AuthUser;
use crate::models::serde_helpers::go_time;
use crate::models::{Announcement, Event, EventMission, OrbatSlot, ServerStatus};
use crate::state::AppState;

#[derive(Debug, Serialize)]
struct EventSummary {
    event_id: String,
    name: String,
    terrain: String,
    #[serde(with = "go_time")]
    start_time: DateTime<Utc>,
    registered: i64,
    max_slots: i64,
    status: String,
}

#[derive(Debug, Serialize)]
struct AssignmentSummary {
    event_id: String,
    name: String,
    faction: String,
    squad: String,
    role: String,
}

/// `GET /api/v1/dashboard` — next op, my assignment, server status, modpack, news.
///
/// @route GET /api/v1/dashboard
pub async fn get_dashboard(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let me = &user.discord_id;
    let pool = &state.pool;

    // Next upcoming operation.
    let next_event: Option<EventSummary> = {
        let ev: Option<Event> = sqlx::query_as(
            "SELECT * FROM events WHERE start_time > now() \
             AND status::text IN ('scheduled', 'open', 'live') AND deleted_at IS NULL \
             ORDER BY start_time ASC LIMIT 1",
        )
        .fetch_optional(pool)
        .await?;
        match ev {
            Some(ev) => {
                let em: Option<EventMission> = sqlx::query_as(
                    "SELECT * FROM event_missions WHERE event_id = $1 ORDER BY start_time ASC LIMIT 1",
                )
                .bind(ev.id)
                .fetch_optional(pool)
                .await?;
                let mt = match &em {
                    Some(em) => mission_title_terrain(pool, em.mission_id).await,
                    None => None,
                };
                let registered: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM event_registrations \
                     JOIN event_missions ON event_missions.id = event_registrations.event_mission_id \
                     WHERE event_missions.event_id = $1 \
                       AND event_registrations.state::text IN ('registered', 'waitlisted')",
                )
                .bind(ev.id)
                .fetch_one(pool)
                .await?;
                let name = if ev.name_override.is_empty() {
                    mt.as_ref().map(|(t, _)| t.clone()).unwrap_or_default()
                } else {
                    ev.name_override.clone()
                };
                Some(EventSummary {
                    event_id: ev.id.to_string(),
                    name,
                    terrain: mt.map(|(_, t)| t.as_str().to_string()).unwrap_or_default(),
                    start_time: ev.start_time,
                    registered,
                    max_slots: ev.max_slots,
                    status: ev.status.as_str().to_string(),
                })
            }
            None => None,
        }
    };

    // Caller's assigned ORBAT slot for an upcoming mission.
    let my_assignment: Option<AssignmentSummary> = {
        let slot: Option<OrbatSlot> = sqlx::query_as(
            "SELECT orbat_slots.* FROM orbat_slots \
             JOIN event_missions ON event_missions.id = orbat_slots.event_mission_id \
             JOIN events ON events.id = event_missions.event_id \
             WHERE orbat_slots.assigned_to = $1 AND event_missions.start_time > now() \
               AND events.deleted_at IS NULL \
             ORDER BY event_missions.start_time ASC LIMIT 1",
        )
        .bind(me)
        .fetch_optional(pool)
        .await?;
        match slot {
            Some(slot) => {
                let em: Option<EventMission> =
                    sqlx::query_as("SELECT * FROM event_missions WHERE id = $1")
                        .bind(slot.event_mission_id)
                        .fetch_optional(pool)
                        .await?;
                let (event_id, mission_id) = em
                    .as_ref()
                    .map(|em| (em.event_id, em.mission_id))
                    .unwrap_or_default();
                let ev_name: String = sqlx::query_scalar(
                    "SELECT COALESCE(name_override, '') FROM events WHERE id = $1",
                )
                .bind(event_id)
                .fetch_optional(pool)
                .await?
                .unwrap_or_default();
                let name = if ev_name.is_empty() {
                    mission_title_terrain(pool, mission_id)
                        .await
                        .map(|(t, _)| t)
                        .unwrap_or_default()
                } else {
                    ev_name
                };
                Some(AssignmentSummary {
                    event_id: event_id.to_string(),
                    name,
                    faction: slot.faction,
                    squad: slot.squad,
                    role: slot.role,
                })
            }
            None => None,
        }
    };

    // Live server status (single primary server assumption).
    let server_status: Option<ServerStatus> = sqlx::query_as(
        "SELECT server_id, is_online, player_count, max_players, server_fps::float8 AS server_fps, \
         uptime_seconds, current_match_id, ingame_time, ingame_weather, updated_at \
         FROM server_statuses LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;

    let current_modpack = load_current_modpack(pool).await?;

    let recent: Vec<Announcement> = sqlx::query_as(
        "SELECT * FROM announcements WHERE status = 'published' AND deleted_at IS NULL \
         ORDER BY published_at DESC LIMIT 3",
    )
    .fetch_all(pool)
    .await?;

    Ok(Json(json!({
        "next_event": next_event,
        "my_assignment": my_assignment,
        "server_status": server_status,
        "current_modpack": current_modpack,
        "recent_announcements": recent,
    })))
}
