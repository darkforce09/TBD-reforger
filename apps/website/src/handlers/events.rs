//! Event (campaign) + ORBAT + registration handlers — Rust port of `handlers/events.go`.
//! The registration path is the concurrency gate **G7b** (lock + conditional slot claim).

use std::collections::{HashMap, HashSet};

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::PageParams;
use crate::middleware::{AdminUser, AuthUser, LeaderUser};
use crate::models::serde_helpers::go_time;
use crate::models::{
    Event, EventMission, EventStatus, MissionArmory, OrbatReservation, OrbatSlot, RegistrationState,
};
use crate::services::{OrbatSquadTemplate, parse_orbat_template};
use crate::state::AppState;

fn valid_event_status(s: &str) -> Option<EventStatus> {
    match s {
        "" | "scheduled" => Some(EventStatus::Scheduled),
        "open" => Some(EventStatus::Open),
        "locked" => Some(EventStatus::Locked),
        "live" => Some(EventStatus::Live),
        "completed" => Some(EventStatus::Completed),
        "cancelled" => Some(EventStatus::Cancelled),
        _ => None,
    }
}

fn can_register_status(s: EventStatus) -> bool {
    s == EventStatus::Scheduled || s == EventStatus::Open
}

// --- helpers ---

async fn load_event(pool: &PgPool, id: &str) -> Result<Event, ApiError> {
    let Ok(id) = Uuid::parse_str(id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    sqlx::query_as("SELECT id, COALESCE(name_override, '') AS name_override, start_time, COALESCE(briefing, '') AS briefing, COALESCE(banner_image_url, '') AS banner_image_url, status, registration_locked, max_slots, created_by, match_id, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM events WHERE id = $1 AND deleted_at IS NULL")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::not_found("event not found"))
}

async fn load_em(pool: &PgPool, emid: &str) -> Result<EventMission, ApiError> {
    let Ok(id) = Uuid::parse_str(emid) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    sqlx::query_as("SELECT id, event_id, mission_id, start_time, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM event_missions WHERE id = $1")
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| ApiError::not_found("mission not found"))
}

/// Materialize parsed squads into OrbatSlot rows for one event mission.
async fn materialize_slots(
    tx: &mut sqlx::PgConnection,
    em_id: Uuid,
    squads: &[OrbatSquadTemplate],
) -> sqlx::Result<()> {
    for sq in squads {
        for (i, sl) in sq.slots.iter().enumerate() {
            sqlx::query(
                "INSERT INTO orbat_slots (event_mission_id, faction, callsign, squad, role, loadout, tag, slot_index) \
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(em_id)
            .bind(&sq.faction)
            .bind(&sq.callsign)
            .bind(&sq.squad)
            .bind(&sl.role)
            .bind(&sl.loadout)
            .bind(&sl.tag)
            .bind(i as i64)
            .execute(&mut *tx)
            .await?;
        }
    }
    Ok(())
}

/// Resolve a mission's ORBAT template from its current published version payload.
async fn orbat_template_for_mission(pool: &PgPool, mission_id: Uuid) -> Vec<OrbatSquadTemplate> {
    let cur: Option<Option<Uuid>> = sqlx::query_scalar(
        "SELECT current_version_id FROM missions WHERE id = $1 AND deleted_at IS NULL",
    )
    .bind(mission_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    let Some(Some(vid)) = cur else {
        return Vec::new();
    };
    let payload: Option<crate::models::RawJson> =
        sqlx::query_scalar("SELECT json_payload FROM mission_versions WHERE id = $1")
            .bind(vid)
            .fetch_optional(pool)
            .await
            .ok()
            .flatten();
    match payload {
        Some(p) => parse_orbat_template(p.0.get().as_bytes()),
        None => Vec::new(),
    }
}

// --- Event container CRUD ---

#[derive(Debug, Deserialize)]
pub struct CreateEventInput {
    start_time: Option<DateTime<Utc>>,
    #[serde(default)]
    name_override: String,
    #[serde(default)]
    briefing: String,
    #[serde(default)]
    banner_image_url: String,
    #[serde(default)]
    max_slots: i64,
    #[serde(default)]
    registration_locked: bool,
    #[serde(default)]
    status: String,
}

/// `POST /api/v1/events` — schedule an operation container (admin).
///
/// @route POST /api/v1/events
pub async fn create_event(
    State(state): State<AppState>,
    _a: AdminUser,
    body: Result<Json<CreateEventInput>, JsonRejection>,
) -> Result<(StatusCode, Json<Event>), ApiError> {
    let Json(input) = body.map_err(|_| ApiError::bad_request("start_time is required"))?;
    let (Some(start_time), true) = (input.start_time, (0..=256).contains(&input.max_slots)) else {
        return Err(ApiError::bad_request("start_time is required"));
    };
    let Some(status) = valid_event_status(&input.status) else {
        return Err(ApiError::bad_request("invalid status"));
    };
    let ev: Event = sqlx::query_as(
        "INSERT INTO events (name_override, start_time, briefing, banner_image_url, status, \
         registration_locked, max_slots, created_by, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now(), now()) RETURNING id, COALESCE(name_override, '') AS name_override, start_time, COALESCE(briefing, '') AS briefing, COALESCE(banner_image_url, '') AS banner_image_url, status, registration_locked, max_slots, created_by, match_id, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at",
    )
    .bind(&input.name_override)
    .bind(start_time)
    .bind(&input.briefing)
    .bind(&input.banner_image_url)
    .bind(status)
    .bind(input.registration_locked)
    .bind(input.max_slots)
    .bind(&_a.0.discord_id)
    .fetch_one(&state.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(ev)))
}

#[derive(Debug, Deserialize)]
pub struct AddMissionInput {
    #[serde(default)]
    mission_id: String,
    start_time: Option<DateTime<Utc>>,
    #[serde(default)]
    orbat: Vec<OrbatSquadTemplate>,
}

/// `POST /api/v1/events/:id/missions` — attach a mission + auto-materialize ORBAT (admin).
///
/// @route POST /api/v1/events/:id/missions
pub async fn add_event_mission(
    State(state): State<AppState>,
    _a: AdminUser,
    Path(id): Path<String>,
    body: Result<Json<AddMissionInput>, JsonRejection>,
) -> Result<(StatusCode, Json<EventMission>), ApiError> {
    let ev = load_event(&state.pool, &id).await?;
    let Json(input) =
        body.map_err(|_| ApiError::bad_request("mission_id and start_time are required"))?;
    let Some(start_time) = input.start_time else {
        return Err(ApiError::bad_request(
            "mission_id and start_time are required",
        ));
    };
    let Ok(mission_id) = Uuid::parse_str(&input.mission_id) else {
        return Err(ApiError::bad_request("invalid mission_id"));
    };
    let exists: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM missions WHERE id = $1 AND deleted_at IS NULL")
            .bind(mission_id)
            .fetch_optional(&state.pool)
            .await?;
    if exists.is_none() {
        return Err(ApiError::not_found("mission not found"));
    }

    let template = if input.orbat.is_empty() {
        orbat_template_for_mission(&state.pool, mission_id).await
    } else {
        input.orbat
    };

    let mut tx = state.pool.begin().await?;
    let em: EventMission = sqlx::query_as(
        "INSERT INTO event_missions (event_id, mission_id, start_time, created_at, updated_at) \
         VALUES ($1, $2, $3, now(), now()) RETURNING id, event_id, mission_id, start_time, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at",
    )
    .bind(ev.id)
    .bind(mission_id)
    .bind(start_time)
    .fetch_one(&mut *tx)
    .await?;
    materialize_slots(&mut tx, em.id, &template).await?;
    tx.commit().await?;

    Ok((StatusCode::CREATED, Json(em)))
}

/// `DELETE /api/v1/events/:id/missions/:emid` — detach a mission (admin).
///
/// @route DELETE /api/v1/events/:id/missions/:emid
pub async fn remove_event_mission(
    State(state): State<AppState>,
    _a: AdminUser,
    Path((id, emid)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let ev = load_event(&state.pool, &id).await?;
    let Ok(em_id) = Uuid::parse_str(&emid) else {
        return Err(ApiError::bad_request("invalid mission id"));
    };
    let mut tx = state.pool.begin().await?;
    let found: Option<Uuid> =
        sqlx::query_scalar("SELECT id FROM event_missions WHERE id = $1 AND event_id = $2")
            .bind(em_id)
            .bind(ev.id)
            .fetch_optional(&mut *tx)
            .await?;
    if found.is_none() {
        return Err(ApiError::not_found("mission not found in event"));
    }
    sqlx::query("DELETE FROM event_registrations WHERE event_mission_id = $1")
        .bind(em_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM orbat_slots WHERE event_mission_id = $1")
        .bind(em_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("DELETE FROM event_missions WHERE id = $1")
        .bind(em_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

// --- Event lists ---

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

/// `GET /api/v1/events` — Upcoming/Calendar list.
///
/// @route GET /api/v1/events
pub async fn list_events(
    State(state): State<AppState>,
    _u: AuthUser,
    Query(q): Query<EventListQuery>,
) -> Result<Json<Value>, ApiError> {
    let (limit, offset) = PageParams {
        limit: q.limit,
        offset: q.offset,
    }
    .bounds();

    // Static per-scope queries (the scope word is a hardcoded whitelist, never bound text).
    let (count_sql, select_sql): (&str, &str) = match q.scope.as_deref().unwrap_or("upcoming") {
        "past" => (
            "SELECT count(*) FROM events WHERE deleted_at IS NULL AND start_time <= now()",
            "SELECT id, COALESCE(name_override, '') AS name_override, start_time, COALESCE(briefing, '') AS briefing, COALESCE(banner_image_url, '') AS banner_image_url, status, registration_locked, max_slots, created_by, match_id, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM events WHERE deleted_at IS NULL AND start_time <= now() \
             ORDER BY start_time DESC LIMIT $1 OFFSET $2",
        ),
        "all" => (
            "SELECT count(*) FROM events WHERE deleted_at IS NULL",
            "SELECT id, COALESCE(name_override, '') AS name_override, start_time, COALESCE(briefing, '') AS briefing, COALESCE(banner_image_url, '') AS banner_image_url, status, registration_locked, max_slots, created_by, match_id, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM events WHERE deleted_at IS NULL ORDER BY start_time ASC LIMIT $1 OFFSET $2",
        ),
        _ => (
            "SELECT count(*) FROM events WHERE deleted_at IS NULL \
             AND (start_time > now() OR status::text = 'live')",
            "SELECT id, COALESCE(name_override, '') AS name_override, start_time, COALESCE(briefing, '') AS briefing, COALESCE(banner_image_url, '') AS banner_image_url, status, registration_locked, max_slots, created_by, match_id, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM events WHERE deleted_at IS NULL \
             AND (start_time > now() OR status::text = 'live') \
             ORDER BY start_time ASC LIMIT $1 OFFSET $2",
        ),
    };

    let total: i64 = sqlx::query_scalar(count_sql)
        .fetch_one(&state.pool)
        .await
        .map_err(ApiError::from)?;
    let events: Vec<Event> = sqlx::query_as(select_sql)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.pool)
        .await
        .map_err(ApiError::from)?;

    let data = decorate_events(&state.pool, events).await?;
    Ok(Json(
        json!({ "data": data, "total": total, "limit": limit, "offset": offset }),
    ))
}

/// Batch-load mission counts, registration counts, ORBAT fill totals per event.
async fn decorate_events(
    pool: &PgPool,
    events: Vec<Event>,
) -> Result<Vec<EventListItem>, ApiError> {
    let event_ids: Vec<Uuid> = events.iter().map(|e| e.id).collect();

    // event_mission id → event id.
    let ems: Vec<(Uuid, Uuid)> =
        sqlx::query_as("SELECT id, event_id FROM event_missions WHERE event_id = ANY($1)")
            .bind(&event_ids)
            .fetch_all(pool)
            .await?;
    let mut mission_count: HashMap<Uuid, i64> = HashMap::new();
    let mut em_to_event: HashMap<Uuid, Uuid> = HashMap::new();
    for (em_id, ev_id) in &ems {
        *mission_count.entry(*ev_id).or_default() += 1;
        em_to_event.insert(*em_id, *ev_id);
    }
    let em_ids: Vec<Uuid> = em_to_event.keys().copied().collect();

    let mut reg_by_event: HashMap<Uuid, i64> = HashMap::new();
    let mut total_by_event: HashMap<Uuid, i64> = HashMap::new();
    let mut filled_by_event: HashMap<Uuid, i64> = HashMap::new();
    if !em_ids.is_empty() {
        let regs: Vec<(Uuid, i64)> = sqlx::query_as(
            "SELECT event_mission_id, count(*) FROM event_registrations \
             WHERE event_mission_id = ANY($1) AND state::text = 'registered' GROUP BY event_mission_id",
        )
        .bind(&em_ids)
        .fetch_all(pool)
        .await?;
        for (em_id, n) in regs {
            if let Some(ev) = em_to_event.get(&em_id) {
                *reg_by_event.entry(*ev).or_default() += n;
            }
        }
        let slots: Vec<(Uuid, i64, i64)> = sqlx::query_as(
            "SELECT event_mission_id, count(*) AS total, count(assigned_to) AS filled \
             FROM orbat_slots WHERE event_mission_id = ANY($1) GROUP BY event_mission_id",
        )
        .bind(&em_ids)
        .fetch_all(pool)
        .await?;
        for (em_id, total, filled) in slots {
            if let Some(ev) = em_to_event.get(&em_id) {
                *total_by_event.entry(*ev).or_default() += total;
                *filled_by_event.entry(*ev).or_default() += filled;
            }
        }
    }

    Ok(events
        .into_iter()
        .map(|e| {
            let total = total_by_event.get(&e.id).copied().unwrap_or(0);
            let filled = filled_by_event.get(&e.id).copied().unwrap_or(0);
            let percent = if total > 0 { filled * 100 / total } else { 0 };
            EventListItem {
                mission_count: mission_count.get(&e.id).copied().unwrap_or(0),
                registered: reg_by_event.get(&e.id).copied().unwrap_or(0),
                filled,
                total_slots: total,
                percent,
                event: e,
            }
        })
        .collect())
}

// --- Event Hub ---

#[derive(Debug, Serialize)]
struct ArmoryFactionDto {
    faction: String,
    items: Vec<MissionArmory>,
}

#[derive(Debug, Serialize)]
struct EventMissionDossier {
    event_mission_id: String,
    mission_id: String,
    title: String,
    terrain: String,
    game_mode: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    briefing: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    thumbnail_url: String,
    #[serde(with = "go_time")]
    start_time: DateTime<Utc>,
    factions: Vec<String>,
    armory_by_faction: Vec<ArmoryFactionDto>,
    filled: i64,
    total: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    my_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    my_slot_id: Option<String>,
}

async fn armory_by_faction(pool: &PgPool, mission_id: Uuid) -> Vec<ArmoryFactionDto> {
    let items: Vec<MissionArmory> = sqlx::query_as(
        "SELECT id, mission_id, faction, category, item_name, quantity, COALESCE(icon, '') AS icon, sort_order FROM mission_armories WHERE mission_id = $1 ORDER BY sort_order ASC",
    )
    .bind(mission_id)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    let mut order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, Vec<MissionArmory>> = HashMap::new();
    for it in items {
        if !groups.contains_key(&it.faction) {
            order.push(it.faction.clone());
        }
        groups.entry(it.faction.clone()).or_default().push(it);
    }
    order
        .into_iter()
        .map(|f| ArmoryFactionDto {
            items: groups.remove(&f).unwrap_or_default(),
            faction: f,
        })
        .collect()
}

/// `GET /api/v1/events/:id` — Event Hub (event + nested mission dossiers).
///
/// @route GET /api/v1/events/:id
pub async fn get_event(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let ev = load_event(&state.pool, &id).await?;
    let me = &user.discord_id;

    let ems: Vec<EventMission> =
        sqlx::query_as("SELECT id, event_id, mission_id, start_time, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM event_missions WHERE event_id = $1 ORDER BY start_time ASC")
            .bind(ev.id)
            .fetch_all(&state.pool)
            .await?;

    let mut missions = Vec::with_capacity(ems.len());
    for em in ems {
        let Some((title, terrain, game_mode, briefing, thumbnail_url)): Option<(String, crate::models::TerrainType, crate::models::GameMode, String, String)> =
            sqlx::query_as("SELECT title, terrain, game_mode, briefing, thumbnail_url FROM missions WHERE id = $1 AND deleted_at IS NULL")
                .bind(em.mission_id)
                .fetch_optional(&state.pool)
                .await?
        else {
            continue;
        };

        let slots: Vec<OrbatSlot> =
            sqlx::query_as("SELECT id, event_mission_id, faction, squad, COALESCE(callsign, '') AS callsign, role, COALESCE(loadout, '') AS loadout, COALESCE(tag, '') AS tag, slot_index, assigned_to, assigned_at FROM orbat_slots WHERE event_mission_id = $1")
                .bind(em.id)
                .fetch_all(&state.pool)
                .await?;
        let mut filled = 0i64;
        let mut faction_seen: HashSet<String> = HashSet::new();
        let mut factions: Vec<String> = Vec::new();
        for s in &slots {
            if s.assigned_to.is_some() {
                filled += 1;
            }
            if faction_seen.insert(s.faction.clone()) {
                factions.push(s.faction.clone());
            }
        }

        // Caller's registration for this mission.
        let reg: Option<(RegistrationState, Option<Uuid>)> = sqlx::query_as(
            "SELECT state, slot_id FROM event_registrations WHERE event_mission_id = $1 AND discord_id = $2",
        )
        .bind(em.id)
        .bind(me)
        .fetch_optional(&state.pool)
        .await?;
        let (my_state, my_slot_id) = match reg {
            Some((st, slot)) => (st.as_str().to_string(), slot.map(|s| s.to_string())),
            None => (String::new(), None),
        };

        missions.push(EventMissionDossier {
            event_mission_id: em.id.to_string(),
            mission_id: em.mission_id.to_string(),
            title,
            terrain: terrain.as_str().to_string(),
            game_mode: game_mode.as_str().to_string(),
            briefing,
            thumbnail_url,
            start_time: em.start_time,
            factions,
            armory_by_faction: armory_by_faction(&state.pool, em.mission_id).await,
            filled,
            total: slots.len() as i64,
            my_state,
            my_slot_id,
        });
    }

    let mut body = serde_json::to_value(&ev).unwrap();
    body.as_object_mut()
        .unwrap()
        .insert("missions".into(), serde_json::to_value(missions).unwrap());
    Ok(Json(body))
}

#[derive(Debug, Deserialize)]
pub struct PatchEventInput {
    start_time: Option<DateTime<Utc>>,
    max_slots: Option<i64>,
    name_override: Option<String>,
    briefing: Option<String>,
    banner_image_url: Option<String>,
    registration_locked: Option<bool>,
    status: Option<String>,
}

/// `PATCH /api/v1/events/:id` — edit an event (admin).
///
/// @route PATCH /api/v1/events/:id
pub async fn update_event(
    State(state): State<AppState>,
    _a: AdminUser,
    Path(id): Path<String>,
    body: Result<Json<PatchEventInput>, JsonRejection>,
) -> Result<Json<Event>, ApiError> {
    let ev = load_event(&state.pool, &id).await?;
    let Json(input) = body.map_err(|_| ApiError::bad_request("invalid body"))?;

    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new("UPDATE events SET updated_at = now()");
    if let Some(t) = input.start_time {
        qb.push(", start_time = ").push_bind(t);
    }
    if let Some(m) = input.max_slots {
        qb.push(", max_slots = ").push_bind(m);
    }
    if let Some(n) = &input.name_override {
        qb.push(", name_override = ").push_bind(n.clone());
    }
    if let Some(b) = &input.briefing {
        qb.push(", briefing = ").push_bind(b.clone());
    }
    if let Some(u) = &input.banner_image_url {
        qb.push(", banner_image_url = ").push_bind(u.clone());
    }
    if let Some(l) = input.registration_locked {
        qb.push(", registration_locked = ").push_bind(l);
    }
    if let Some(s) = &input.status {
        let Some(status) = valid_event_status(s) else {
            return Err(ApiError::bad_request("invalid status"));
        };
        qb.push(", status = ").push_bind(status);
    }
    qb.push(" WHERE id = ").push_bind(ev.id);
    qb.build()
        .execute(&state.pool)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(load_event(&state.pool, &id).await?))
}

/// `DELETE /api/v1/events/:id` — soft-delete an event (admin).
///
/// @route DELETE /api/v1/events/:id
pub async fn delete_event(
    State(state): State<AppState>,
    _a: AdminUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let ev = load_event(&state.pool, &id).await?;
    sqlx::query("UPDATE events SET deleted_at = now() WHERE id = $1")
        .bind(ev.id)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

// --- ORBAT ---

#[derive(Debug, Serialize)]
struct OrbatSlotDto {
    id: String,
    number: i64,
    role: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    loadout: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    tag: String,
    slot_index: i64,
    assigned_to: Option<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    assigned_name: String,
}

#[derive(Debug, Serialize)]
struct OrbatSquadDto {
    faction: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    callsign: String,
    squad: String,
    filled: i64,
    total: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    reserved_by: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    reserved_by_name: String,
    slots: Vec<OrbatSlotDto>,
}

/// `GET /api/v1/event-missions/:emid/orbat` — ORBAT grouped by squad.
///
/// @route GET /api/v1/event-missions/:emid/orbat
pub async fn get_orbat(
    State(state): State<AppState>,
    _u: AuthUser,
    Path(emid): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let slots: Vec<OrbatSlot> = sqlx::query_as(
        "SELECT id, event_mission_id, faction, squad, COALESCE(callsign, '') AS callsign, role, COALESCE(loadout, '') AS loadout, COALESCE(tag, '') AS tag, slot_index, assigned_to, assigned_at FROM orbat_slots WHERE event_mission_id = $1 \
         ORDER BY faction ASC, squad ASC, slot_index ASC",
    )
    .bind(em.id)
    .fetch_all(&state.pool)
    .await?;

    let reservations: Vec<OrbatReservation> =
        sqlx::query_as("SELECT * FROM orbat_reservations WHERE event_mission_id = $1")
            .bind(em.id)
            .fetch_all(&state.pool)
            .await?;
    let reserved_by: HashMap<String, String> = reservations
        .into_iter()
        .map(|r| (r.squad, r.reserved_by))
        .collect();

    // Resolve display names for assignees + reservers.
    let mut ids: HashSet<String> = HashSet::new();
    for s in &slots {
        if let Some(a) = &s.assigned_to {
            ids.insert(a.clone());
        }
    }
    for who in reserved_by.values() {
        ids.insert(who.clone());
    }
    let id_vec: Vec<String> = ids.into_iter().collect();
    let names: HashMap<String, String> =
        sqlx::query_as("SELECT discord_id, COALESCE(username, '') AS username FROM users WHERE discord_id = ANY($1)")
            .bind(&id_vec)
            .fetch_all(&state.pool)
            .await?
            .into_iter()
            .collect();

    let mut order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, OrbatSquadDto> = HashMap::new();
    for s in &slots {
        let g = groups.entry(s.squad.clone()).or_insert_with(|| {
            order.push(s.squad.clone());
            let (rb, rbn) = match reserved_by.get(&s.squad) {
                Some(who) => (who.clone(), names.get(who).cloned().unwrap_or_default()),
                None => (String::new(), String::new()),
            };
            OrbatSquadDto {
                faction: s.faction.clone(),
                callsign: s.callsign.clone(),
                squad: s.squad.clone(),
                filled: 0,
                total: 0,
                reserved_by: rb,
                reserved_by_name: rbn,
                slots: Vec::new(),
            }
        });
        let assigned_name = s
            .assigned_to
            .as_ref()
            .and_then(|a| names.get(a).cloned())
            .unwrap_or_default();
        if s.assigned_to.is_some() {
            g.filled += 1;
        }
        g.total += 1;
        g.slots.push(OrbatSlotDto {
            id: s.id.to_string(),
            number: s.slot_index + 1,
            role: s.role.clone(),
            loadout: s.loadout.clone(),
            tag: s.tag.clone(),
            slot_index: s.slot_index,
            assigned_to: s.assigned_to.clone(),
            assigned_name,
        });
    }
    let out: Vec<OrbatSquadDto> = order
        .into_iter()
        .filter_map(|sq| groups.remove(&sq))
        .collect();
    Ok(Json(json!({ "data": out })))
}

// --- Registration (G7b) ---

#[derive(Debug, Deserialize, Default)]
pub struct RegisterBody {
    #[serde(default)]
    slot_id: String,
}

/// `POST /api/v1/event-missions/:emid/register` — claim a slot / waitlist.
/// Concurrency gate **G7b**: `FOR UPDATE` on the mission row + conditional slot claim.
///
/// @route POST /api/v1/event-missions/:emid/register
pub async fn register_for_event_mission(
    State(state): State<AppState>,
    user: AuthUser,
    Path(emid): Path<String>,
    body: Result<Json<RegisterBody>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let ev: Option<Event> =
        sqlx::query_as("SELECT id, COALESCE(name_override, '') AS name_override, start_time, COALESCE(briefing, '') AS briefing, COALESCE(banner_image_url, '') AS banner_image_url, status, registration_locked, max_slots, created_by, match_id, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM events WHERE id = $1 AND deleted_at IS NULL")
            .bind(em.event_id)
            .fetch_optional(&state.pool)
            .await?;
    let Some(ev) = ev else {
        return Err(ApiError::not_found("event not found"));
    };
    let me = &user.discord_id;
    let is_admin = user.role == "admin";
    if !can_register_status(ev.status) {
        return Err(ApiError::conflict(
            "registration is closed for this operation",
        ));
    }
    if ev.registration_locked && !is_admin {
        return Err(ApiError::forbidden(
            "registration is locked; an admin must assign you",
        ));
    }
    let body = body.ok().map(|Json(b)| b).unwrap_or_default();

    let mut tx = state.pool.begin().await?;
    // Serialize registrations per event mission — the capacity/waitlist decision is
    // check-then-write, so concurrent registrations must queue on the mission row.
    sqlx::query("SELECT id FROM event_missions WHERE id = $1 FOR UPDATE")
        .bind(em.id)
        .fetch_one(&mut *tx)
        .await?;

    let capacity: i64 =
        sqlx::query_scalar("SELECT count(*) FROM orbat_slots WHERE event_mission_id = $1")
            .bind(em.id)
            .fetch_one(&mut *tx)
            .await?;
    let registered: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM event_registrations WHERE event_mission_id = $1 AND state::text = 'registered' AND discord_id <> $2",
    )
    .bind(em.id)
    .bind(me)
    .fetch_one(&mut *tx)
    .await?;

    let mut reg_state = RegistrationState::Registered;
    let mut slot_id: Option<Uuid> = None;

    if !body.slot_id.is_empty() {
        let Ok(sid) = Uuid::parse_str(&body.slot_id) else {
            return Err(ApiError::not_found("slot not found"));
        };
        let slot: Option<OrbatSlot> =
            sqlx::query_as("SELECT id, event_mission_id, faction, squad, COALESCE(callsign, '') AS callsign, role, COALESCE(loadout, '') AS loadout, COALESCE(tag, '') AS tag, slot_index, assigned_to, assigned_at FROM orbat_slots WHERE id = $1 AND event_mission_id = $2")
                .bind(sid)
                .bind(em.id)
                .fetch_optional(&mut *tx)
                .await?;
        let Some(slot) = slot else {
            return Err(ApiError::not_found("slot not found"));
        };
        if slot.assigned_to.as_deref().is_some_and(|a| a != me) {
            return Err(ApiError::conflict("slot already taken"));
        }
        // A reserved squad is held for its leader (or an admin).
        if !is_admin {
            let res: Option<String> = sqlx::query_scalar(
                "SELECT reserved_by FROM orbat_reservations WHERE event_mission_id = $1 AND squad = $2",
            )
            .bind(em.id)
            .bind(&slot.squad)
            .fetch_optional(&mut *tx)
            .await?;
            if let Some(rb) = res
                && rb != *me
            {
                return Err(ApiError::conflict("squad is reserved by a leader"));
            }
        }
        // Conditional claim — only a free slot (or the caller's own) is assignable.
        let upd = sqlx::query(
            "UPDATE orbat_slots SET assigned_to = $1, assigned_at = now() \
             WHERE id = $2 AND event_mission_id = $3 AND (assigned_to IS NULL OR assigned_to = $1)",
        )
        .bind(me)
        .bind(sid)
        .bind(em.id)
        .execute(&mut *tx)
        .await?;
        if upd.rows_affected() != 1 {
            return Err(ApiError::conflict("slot already taken"));
        }
        slot_id = Some(sid);
    } else if capacity > 0 && registered >= capacity {
        reg_state = RegistrationState::Waitlisted;
    }

    let (state_out, slot_out): (RegistrationState, Option<Uuid>) = sqlx::query_as(
        "INSERT INTO event_registrations (event_mission_id, discord_id, slot_id, state) \
         VALUES ($1, $2, $3, $4) \
         ON CONFLICT (event_mission_id, discord_id) DO UPDATE SET slot_id = EXCLUDED.slot_id, state = EXCLUDED.state \
         RETURNING state, slot_id",
    )
    .bind(em.id)
    .bind(me)
    .bind(slot_id)
    .bind(reg_state)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;

    Ok(Json(
        json!({ "state": state_out.as_str(), "slot_id": slot_out }),
    ))
}

/// `DELETE /api/v1/event-missions/:emid/register` — withdraw + promote waitlist.
///
/// @route DELETE /api/v1/event-missions/:emid/register
pub async fn withdraw_from_event_mission(
    State(state): State<AppState>,
    user: AuthUser,
    Path(emid): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let me = &user.discord_id;

    let mut tx = state.pool.begin().await?;
    let reg: Option<(Uuid, Option<Uuid>, RegistrationState)> = sqlx::query_as(
        "SELECT id, slot_id, state FROM event_registrations WHERE event_mission_id = $1 AND discord_id = $2",
    )
    .bind(em.id)
    .bind(me)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((reg_id, reg_slot, reg_state)) = reg else {
        return Err(ApiError::not_found("not registered"));
    };
    if let Some(sid) = reg_slot {
        sqlx::query("UPDATE orbat_slots SET assigned_to = NULL, assigned_at = NULL WHERE id = $1")
            .bind(sid)
            .execute(&mut *tx)
            .await?;
    }
    let was_registered = reg_state == RegistrationState::Registered;
    sqlx::query("DELETE FROM event_registrations WHERE id = $1")
        .bind(reg_id)
        .execute(&mut *tx)
        .await?;
    if was_registered {
        let next: Option<Uuid> = sqlx::query_scalar(
            "SELECT id FROM event_registrations WHERE event_mission_id = $1 AND state::text = 'waitlisted' \
             ORDER BY registered_at ASC LIMIT 1",
        )
        .bind(em.id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(next_id) = next {
            sqlx::query("UPDATE event_registrations SET state = 'registered' WHERE id = $1")
                .bind(next_id)
                .execute(&mut *tx)
                .await?;
        }
    }
    tx.commit().await?;
    Ok(Json(json!({ "withdrawn": true })))
}

// --- Slot assignment (leader) ---

async fn can_manage_squad(
    pool: &PgPool,
    is_admin: bool,
    me: &str,
    em_id: Uuid,
    squad: &str,
) -> bool {
    if is_admin {
        return true;
    }
    let res: Option<String> = sqlx::query_scalar(
        "SELECT reserved_by FROM orbat_reservations WHERE event_mission_id = $1 AND squad = $2",
    )
    .bind(em_id)
    .bind(squad)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();
    res.as_deref() == Some(me)
}

#[derive(Debug, Deserialize)]
pub struct AssignSlotInput {
    #[serde(default)]
    discord_id: String,
}

/// `PUT /api/v1/event-missions/:emid/slots/:slotId/assign` — assign a user (leader/admin).
///
/// @route PUT /api/v1/event-missions/:emid/slots/:slotId/assign
pub async fn assign_slot(
    State(state): State<AppState>,
    leader: LeaderUser,
    Path((emid, slot_id_s)): Path<(String, String)>,
    body: Result<Json<AssignSlotInput>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let Ok(slot_id) = Uuid::parse_str(&slot_id_s) else {
        return Err(ApiError::bad_request("invalid slot id"));
    };
    let Json(input) = body.map_err(|_| ApiError::bad_request("discord_id required"))?;
    if input.discord_id.is_empty() {
        return Err(ApiError::bad_request("discord_id required"));
    }
    let exists: Option<i32> = sqlx::query_scalar("SELECT 1 FROM users WHERE discord_id = $1")
        .bind(&input.discord_id)
        .fetch_optional(&state.pool)
        .await?;
    if exists.is_none() {
        return Err(ApiError::bad_request("user not found"));
    }
    let slot: Option<OrbatSlot> =
        sqlx::query_as("SELECT id, event_mission_id, faction, squad, COALESCE(callsign, '') AS callsign, role, COALESCE(loadout, '') AS loadout, COALESCE(tag, '') AS tag, slot_index, assigned_to, assigned_at FROM orbat_slots WHERE id = $1 AND event_mission_id = $2")
            .bind(slot_id)
            .bind(em.id)
            .fetch_optional(&state.pool)
            .await?;
    let Some(slot) = slot else {
        return Err(ApiError::not_found("slot not found"));
    };
    let is_admin = leader.0.role == "admin";
    if !can_manage_squad(
        &state.pool,
        is_admin,
        &leader.0.discord_id,
        em.id,
        &slot.squad,
    )
    .await
    {
        return Err(ApiError::forbidden(
            "reserve this squad to assign its slots",
        ));
    }

    let mut tx = state.pool.begin().await?;
    sqlx::query("UPDATE orbat_slots SET assigned_to = $1, assigned_at = now() WHERE id = $2")
        .bind(&input.discord_id)
        .bind(slot_id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO event_registrations (event_mission_id, discord_id, slot_id, state) \
         VALUES ($1, $2, $3, 'registered') \
         ON CONFLICT (event_mission_id, discord_id) DO UPDATE SET slot_id = EXCLUDED.slot_id, state = EXCLUDED.state",
    )
    .bind(em.id)
    .bind(&input.discord_id)
    .bind(slot_id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(Json(json!({ "assigned_to": input.discord_id })))
}

/// `DELETE /api/v1/event-missions/:emid/slots/:slotId/assign` — unassign (leader/admin).
///
/// @route DELETE /api/v1/event-missions/:emid/slots/:slotId/assign
pub async fn clear_slot(
    State(state): State<AppState>,
    leader: LeaderUser,
    Path((emid, slot_id_s)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let Ok(slot_id) = Uuid::parse_str(&slot_id_s) else {
        return Err(ApiError::bad_request("invalid slot id"));
    };
    let slot: Option<OrbatSlot> =
        sqlx::query_as("SELECT id, event_mission_id, faction, squad, COALESCE(callsign, '') AS callsign, role, COALESCE(loadout, '') AS loadout, COALESCE(tag, '') AS tag, slot_index, assigned_to, assigned_at FROM orbat_slots WHERE id = $1 AND event_mission_id = $2")
            .bind(slot_id)
            .bind(em.id)
            .fetch_optional(&state.pool)
            .await?;
    let Some(slot) = slot else {
        return Err(ApiError::not_found("slot not found"));
    };
    let is_admin = leader.0.role == "admin";
    if !can_manage_squad(
        &state.pool,
        is_admin,
        &leader.0.discord_id,
        em.id,
        &slot.squad,
    )
    .await
    {
        return Err(ApiError::forbidden(
            "reserve this squad to manage its slots",
        ));
    }
    let mut tx = state.pool.begin().await?;
    sqlx::query("UPDATE orbat_slots SET assigned_to = NULL, assigned_at = NULL WHERE id = $1 AND event_mission_id = $2")
        .bind(slot_id)
        .bind(em.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE event_registrations SET slot_id = NULL WHERE event_mission_id = $1 AND slot_id = $2")
        .bind(em.id)
        .bind(slot_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(Json(json!({ "cleared": true })))
}

// --- Squad reservation (leader) ---

#[derive(Debug, Deserialize)]
pub struct SquadBody {
    #[serde(default)]
    squad: String,
}

/// `POST /api/v1/event-missions/:emid/squads/reserve` — hold a squad (leader).
///
/// @route POST /api/v1/event-missions/:emid/squads/reserve
pub async fn reserve_squad(
    State(state): State<AppState>,
    leader: LeaderUser,
    Path(emid): Path<String>,
    body: Result<Json<SquadBody>, JsonRejection>,
) -> Result<Response, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let Json(input) = body.map_err(|_| ApiError::bad_request("squad is required"))?;
    if input.squad.is_empty() {
        return Err(ApiError::bad_request("squad is required"));
    }
    let me = &leader.0.discord_id;

    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM orbat_slots WHERE event_mission_id = $1 AND squad = $2",
    )
    .bind(em.id)
    .bind(&input.squad)
    .fetch_one(&state.pool)
    .await?;
    if n == 0 {
        return Err(ApiError::not_found("squad not found in this ORBAT"));
    }

    let existing: Option<OrbatReservation> = sqlx::query_as(
        "SELECT * FROM orbat_reservations WHERE event_mission_id = $1 AND squad = $2",
    )
    .bind(em.id)
    .bind(&input.squad)
    .fetch_optional(&state.pool)
    .await?;
    if let Some(existing) = existing {
        if existing.reserved_by != *me {
            return Err(ApiError::conflict("squad is already reserved"));
        }
        return Ok((StatusCode::OK, Json(existing)).into_response());
    }

    let res: OrbatReservation = sqlx::query_as(
        "INSERT INTO orbat_reservations (event_mission_id, squad, reserved_by) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(em.id)
    .bind(&input.squad)
    .bind(me)
    .fetch_one(&state.pool)
    .await?;
    Ok((StatusCode::CREATED, Json(res)).into_response())
}

/// `POST /api/v1/event-missions/:emid/squads/release` — lift a squad hold (leader/admin).
///
/// @route POST /api/v1/event-missions/:emid/squads/release
pub async fn release_squad(
    State(state): State<AppState>,
    leader: LeaderUser,
    Path(emid): Path<String>,
    body: Result<Json<SquadBody>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let Json(input) = body.map_err(|_| ApiError::bad_request("squad is required"))?;
    if input.squad.is_empty() {
        return Err(ApiError::bad_request("squad is required"));
    }
    let res: Option<OrbatReservation> = sqlx::query_as(
        "SELECT * FROM orbat_reservations WHERE event_mission_id = $1 AND squad = $2",
    )
    .bind(em.id)
    .bind(&input.squad)
    .fetch_optional(&state.pool)
    .await?;
    let Some(res) = res else {
        return Err(ApiError::not_found("squad is not reserved"));
    };
    let is_admin = leader.0.role == "admin";
    if res.reserved_by != leader.0.discord_id && !is_admin {
        return Err(ApiError::forbidden(
            "only the reserver or an admin can release this squad",
        ));
    }
    sqlx::query("DELETE FROM orbat_reservations WHERE event_mission_id = $1 AND squad = $2")
        .bind(em.id)
        .bind(&input.squad)
        .execute(&state.pool)
        .await?;
    Ok(Json(json!({ "released": true })))
}

// --- Member directory (leader) ---

#[derive(Debug, Serialize)]
struct MemberDto {
    discord_id: String,
    username: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    avatar_url: String,
}

#[derive(Debug, Deserialize)]
pub struct MemberQuery {
    q: Option<String>,
}

/// `GET /api/v1/members` — slim member directory for leaders (excludes banned).
///
/// @route GET /api/v1/members
pub async fn search_members(
    State(state): State<AppState>,
    _l: LeaderUser,
    Query(q): Query<MemberQuery>,
) -> Result<Json<Value>, ApiError> {
    // COALESCE nullable text → '' to mirror Go/GORM scanning NULL into the string zero.
    let mut qb: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT discord_id, COALESCE(username, ''), COALESCE(avatar_url, '') \
         FROM users WHERE is_banned = false",
    );
    if let Some(search) = q.q.as_deref().filter(|s| !s.is_empty()) {
        let like = format!("%{search}%");
        qb.push(" AND (username ILIKE ").push_bind(like.clone());
        qb.push(" OR discord_handle ILIKE ")
            .push_bind(like)
            .push(")");
    }
    qb.push(" ORDER BY username ASC LIMIT 20");
    let rows: Vec<(String, String, String)> = qb
        .build_query_as()
        .fetch_all(&state.pool)
        .await
        .map_err(ApiError::from)?;
    let out: Vec<MemberDto> = rows
        .into_iter()
        .map(|(discord_id, username, avatar_url)| MemberDto {
            discord_id,
            username,
            avatar_url,
        })
        .collect();
    Ok(Json(json!({ "data": out })))
}
