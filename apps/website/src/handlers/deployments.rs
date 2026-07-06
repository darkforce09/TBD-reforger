//! My Deployments + Leave of Absence — Rust port of `handlers/deployments.go`.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::{PageParams, load_user};
use crate::middleware::{AdminUser, AuthUser};
use crate::models::serde_helpers::go_time;
use crate::models::{
    Event, EventMission, EventRegistration, LeaveRequest, Match, MatchPlayerStat, OrbatSlot,
    TerrainType,
};
use crate::state::AppState;

#[derive(Debug, Serialize)]
struct DeploymentUpcoming {
    event_id: String,
    event_mission_id: String,
    name: String,
    terrain: String,
    #[serde(with = "go_time")]
    start_time: DateTime<Utc>,
    state: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    faction: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    squad: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    role: String,
}

#[derive(Debug, Serialize)]
struct ServiceRecord {
    #[serde(with = "go_time")]
    date: DateTime<Utc>,
    operation: String,
    role: String,
    outcome: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    aar_replay_url: String,
}

/// Fetch a mission's (title, terrain) for enrichment (avoids the full-row time cast).
pub(crate) async fn mission_title_terrain(
    pool: &sqlx::PgPool,
    id: Uuid,
) -> Option<(String, TerrainType)> {
    sqlx::query_as("SELECT title, terrain FROM missions WHERE id = $1 AND deleted_at IS NULL")
        .bind(id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
}

/// `GET /api/v1/me/deployments` — service record: stats, upcoming, history.
///
/// @route GET /api/v1/me/deployments
pub async fn get_my_deployments(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let me = &user.discord_id;
    let Some(u) = load_user(&state.pool, me).await? else {
        return Err(ApiError::not_found("user not found"));
    };

    // Upcoming: my registrations on future missions within events.
    let regs: Vec<EventRegistration> = sqlx::query_as(
        "SELECT event_registrations.* FROM event_registrations \
         JOIN event_missions ON event_missions.id = event_registrations.event_mission_id \
         JOIN events ON events.id = event_missions.event_id \
         WHERE event_registrations.discord_id = $1 AND event_missions.start_time > now() \
           AND events.deleted_at IS NULL \
         ORDER BY event_missions.start_time ASC",
    )
    .bind(me)
    .fetch_all(&state.pool)
    .await?;

    let mut upcoming: Vec<DeploymentUpcoming> = Vec::with_capacity(regs.len());
    for reg in regs {
        let Some(em) = load_event_mission(&state.pool, reg.event_mission_id).await? else {
            continue;
        };
        let Some(ev) = load_event(&state.pool, em.event_id).await? else {
            continue;
        };
        let mt = mission_title_terrain(&state.pool, em.mission_id).await;
        let name = if ev.name_override.is_empty() {
            mt.as_ref().map(|(t, _)| t.clone()).unwrap_or_default()
        } else {
            ev.name_override.clone()
        };
        let slot: Option<OrbatSlot> = sqlx::query_as(
            "SELECT id, event_mission_id, faction, squad, COALESCE(callsign, '') AS callsign, role, COALESCE(loadout, '') AS loadout, COALESCE(tag, '') AS tag, slot_index, assigned_to, assigned_at FROM orbat_slots WHERE event_mission_id = $1 AND assigned_to = $2",
        )
        .bind(em.id)
        .bind(me)
        .fetch_optional(&state.pool)
        .await?;
        let (faction, squad, role) = slot
            .map(|s| (s.faction, s.squad, s.role))
            .unwrap_or_default();
        upcoming.push(DeploymentUpcoming {
            event_id: ev.id.to_string(),
            event_mission_id: em.id.to_string(),
            name,
            terrain: mt.map(|(_, t)| t.as_str().to_string()).unwrap_or_default(),
            start_time: em.start_time,
            state: reg.state.as_str().to_string(),
            faction,
            squad,
            role,
        });
    }

    // Service history: past match participation.
    let stats: Vec<MatchPlayerStat> = sqlx::query_as(
        "SELECT id, match_id, discord_id, arma_id, COALESCE(role_played, '') AS role_played, kills, deaths, team_kills, longest_kill_m, vehicles_destroyed, is_command, command_win, source_event_id, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at FROM match_player_stats WHERE discord_id = $1 ORDER BY created_at DESC LIMIT 50",
    )
    .bind(me)
    .fetch_all(&state.pool)
    .await?;
    let mut history: Vec<ServiceRecord> = Vec::with_capacity(stats.len());
    for s in stats {
        let m: Option<Match> = sqlx::query_as("SELECT id, source_match_id, event_id, mission_id, terrain, started_at, ended_at, outcome, COALESCE(winning_faction, '') AS winning_faction, COALESCE(aar_replay_url, '') AS aar_replay_url, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at FROM matches WHERE id = $1")
            .bind(s.match_id)
            .fetch_optional(&state.pool)
            .await?;
        let (date, outcome, aar, operation) = match m {
            Some(mm) => {
                let op = match mm.mission_id {
                    Some(mid) => mission_title_terrain(&state.pool, mid)
                        .await
                        .map(|(t, _)| t)
                        .unwrap_or_default(),
                    None => String::new(),
                };
                (
                    mm.started_at,
                    mm.outcome.as_str().to_string(),
                    mm.aar_replay_url,
                    op,
                )
            }
            None => (go_zero(), String::new(), String::new(), String::new()),
        };
        history.push(ServiceRecord {
            date,
            operation,
            role: s.role_played,
            outcome,
            aar_replay_url: aar,
        });
    }

    Ok(Json(json!({
        "total_operations": u.total_deployments,
        "attendance_rate": u.attendance_rate,
        "upcoming": upcoming,
        "service_history": history,
    })))
}

async fn load_event_mission(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<Option<EventMission>> {
    sqlx::query_as("SELECT id, event_id, mission_id, start_time, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM event_missions WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await
}

async fn load_event(pool: &sqlx::PgPool, id: Uuid) -> sqlx::Result<Option<Event>> {
    sqlx::query_as("SELECT id, COALESCE(name_override, '') AS name_override, start_time, COALESCE(briefing, '') AS briefing, COALESCE(banner_image_url, '') AS banner_image_url, status, registration_locked, max_slots, created_by, match_id, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM events WHERE id = $1 AND deleted_at IS NULL")
        .bind(id)
        .fetch_optional(pool)
        .await
}

/// Go's zero `time.Time` (`0001-01-01T00:00:00Z`) — used only for the unreachable
/// orphan-match path (a MatchPlayerStat always references a real match).
fn go_zero() -> DateTime<Utc> {
    NaiveDate::from_ymd_opt(1, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
}

// --- Leave of Absence ---

#[derive(Debug, Deserialize)]
pub struct CreateLeaveInput {
    #[serde(default)]
    starts_on: String,
    #[serde(default)]
    ends_on: String,
    #[serde(default)]
    reason: String,
}

/// `POST /api/v1/me/leave-requests` — file an LOA.
///
/// @route POST /api/v1/me/leave-requests
pub async fn submit_leave(
    State(state): State<AppState>,
    user: AuthUser,
    body: Result<Json<CreateLeaveInput>, JsonRejection>,
) -> Result<(StatusCode, Json<LeaveRequest>), ApiError> {
    let Json(input) =
        body.map_err(|_| ApiError::bad_request("starts_on and ends_on are required"))?;
    if input.starts_on.is_empty() || input.ends_on.is_empty() {
        return Err(ApiError::bad_request("starts_on and ends_on are required"));
    }
    let (Ok(start), Ok(end)) = (
        NaiveDate::parse_from_str(&input.starts_on, "%Y-%m-%d"),
        NaiveDate::parse_from_str(&input.ends_on, "%Y-%m-%d"),
    ) else {
        return Err(ApiError::bad_request("dates must be YYYY-MM-DD"));
    };
    if end < start {
        return Err(ApiError::bad_request(
            "ends_on must be on or after starts_on",
        ));
    }

    let loa: LeaveRequest = sqlx::query_as(
        "INSERT INTO leave_requests (discord_id, starts_on, ends_on, reason, status, created_at) \
         VALUES ($1, $2, $3, $4, 'pending', now()) RETURNING id, discord_id, starts_on, ends_on, COALESCE(reason, '') AS reason, status, reviewed_by, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at",
    )
    .bind(&user.discord_id)
    .bind(start)
    .bind(end)
    .bind(&input.reason)
    .fetch_one(&state.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(loa)))
}

/// `GET /api/v1/me/leave-requests` — the caller's LOA requests.
///
/// @route GET /api/v1/me/leave-requests
pub async fn list_my_leave(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let loas: Vec<LeaveRequest> = sqlx::query_as(
        "SELECT id, discord_id, starts_on, ends_on, COALESCE(reason, '') AS reason, status, reviewed_by, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at FROM leave_requests WHERE discord_id = $1 ORDER BY created_at DESC",
    )
    .bind(&user.discord_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({ "data": loas })))
}

/// `GET /api/v1/admin/leave-requests` — LOA review queue (admin), pending first.
///
/// @route GET /api/v1/admin/leave-requests
pub async fn list_all_leave(
    State(state): State<AppState>,
    _a: AdminUser,
    Query(page): Query<PageParams>,
) -> Result<Json<Value>, ApiError> {
    let (limit, offset) = page.bounds();
    let total: i64 = sqlx::query_scalar("SELECT count(*) FROM leave_requests")
        .fetch_one(&state.pool)
        .await?;
    let loas: Vec<LeaveRequest> = sqlx::query_as(
        "SELECT id, discord_id, starts_on, ends_on, COALESCE(reason, '') AS reason, status, reviewed_by, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at FROM leave_requests ORDER BY (status::text = 'pending') DESC, created_at DESC \
         LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(
        json!({ "data": loas, "total": total, "limit": limit, "offset": offset }),
    ))
}

#[derive(Debug, Deserialize)]
pub struct ReviewLeaveInput {
    #[serde(default)]
    status: String,
}

/// `PATCH /api/v1/admin/leave-requests/:id` — approve/deny an LOA (admin).
///
/// @route PATCH /api/v1/admin/leave-requests/:id
pub async fn review_leave(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
    body: Result<Json<ReviewLeaveInput>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let Ok(id) = Uuid::parse_str(&id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    let Json(input) = body.map_err(|_| ApiError::bad_request("status required"))?;
    if input.status.is_empty() {
        return Err(ApiError::bad_request("status required"));
    }
    if input.status != "approved" && input.status != "denied" {
        return Err(ApiError::bad_request("status must be approved or denied"));
    }
    let res = sqlx::query(
        "UPDATE leave_requests SET status = $1::leave_status, reviewed_by = $2 WHERE id = $3",
    )
    .bind(&input.status)
    .bind(&admin.0.discord_id)
    .bind(id)
    .execute(&state.pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::not_found("LOA not found"));
    }
    Ok(Json(json!({ "status": input.status })))
}
