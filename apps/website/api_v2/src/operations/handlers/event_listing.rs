//! The two event reads: the calendar list with its per-event fill decoration, and the Event
//! Hub dossier for a single event.
//!
//! Both filter and report the EFFECTIVE status ([`crate::operations::services::event_status_rules`]),
//! so an operation that started while the convergence sweep was between ticks is listed and
//! rendered as `live` rather than as whatever the stored column still says.

use std::collections::{HashMap, HashSet};

use axum::extract::{Path, Query, State};
use axum::response::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::http::pagination::PageParams;
use crate::core::middleware::AuthUser;
use crate::core::wire_format::go_time;
use crate::missions::models::mission::MissionArmory;
use crate::operations::models::{Event, EventMission, OrbatSlot, RegistrationState};
use crate::operations::services::event_lookup::load_event;
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
    // The `upcoming` filter tests the EFFECTIVE status, so an operation that started while
    // the sweep was between ticks is still listed as upcoming/live rather than vanishing,
    // and one whose end horizon has passed drops off even if the column still says `live`.
    let (count_sql, select_sql): (String, String) = match q.scope.as_deref().unwrap_or("upcoming") {
        "past" => (
            "SELECT count(*) FROM events e WHERE e.deleted_at IS NULL AND e.start_time <= now()"
                .to_string(),
            format!(
                "SELECT {} FROM events e WHERE e.deleted_at IS NULL AND e.start_time <= now() \
                 ORDER BY e.start_time DESC LIMIT $1 OFFSET $2",
                &*EVENT_COLUMNS
            ),
        ),
        "all" => (
            "SELECT count(*) FROM events e WHERE e.deleted_at IS NULL".to_string(),
            format!(
                "SELECT {} FROM events e WHERE e.deleted_at IS NULL \
                 ORDER BY e.start_time ASC LIMIT $1 OFFSET $2",
                &*EVENT_COLUMNS
            ),
        ),
        _ => (
            format!(
                "SELECT count(*) FROM events e WHERE e.deleted_at IS NULL \
                 AND (e.start_time > now() OR ({})::text = 'live')",
                &*EFFECTIVE_STATUS_SQL
            ),
            format!(
                "SELECT {} FROM events e WHERE e.deleted_at IS NULL \
                 AND (e.start_time > now() OR ({})::text = 'live') \
                 ORDER BY e.start_time ASC LIMIT $1 OFFSET $2",
                &*EVENT_COLUMNS, &*EFFECTIVE_STATUS_SQL
            ),
        ),
    };

    let total: i64 = sqlx::query_scalar(sql(count_sql))
        .fetch_one(&state.pool)
        .await
        .map_err(ApiError::from)?;
    let events: Vec<Event> = sqlx::query_as(sql(select_sql))
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
        // Two of these five columns are nullable and three are not, so the `COALESCE`s are
        // per-column and not a blanket wrap. Checked against `information_schema`, not against
        // what the DDL looks like: `title` is NOT NULL, and `terrain`/`game_mode` are NOT NULL
        // enums that could not be coalesced to `''` even if they were — a `COALESCE` on any of
        // the three would assert a nullability the schema does not have. `briefing` and
        // `thumbnail_url` are `is_nullable = YES` with **no DEFAULT**, decoded positionally into
        // a tuple of plain `String`s, and either one NULL takes the whole Event Hub down:
        // *"error occurred while decoding column 3: unexpected null; try decoding as an
        // `Option`"* for `briefing`, the same at *column 4* for `thumbnail_url`. Positional, so
        // the message names an index rather than a column.
        //
        // Not latent: `seeds/mock_data.sql:25` omits `thumbnail_url` from its INSERT column
        // list, so all four seeded missions carry NULL there. Measured end-to-end on a clean
        // database with nothing but that seed applied — create an event, attach a seeded
        // mission, `GET /api/v1/events/{id}` → **500**. A developer takes the Event Hub down by
        // seeding, with no reason to suspect this query.
        //
        // `''` is the whole fallback for both, and deliberately not a sentinel standing in for a
        // fact. `EventMissionDossier` carries `skip_serializing_if = "String::is_empty"` on both
        // fields, so `''` **omits the key** — on the wire it is absence, not a value, which is
        // exactly the true statement ("this mission has no briefing" / "no thumbnail"). The
        // nearby sibling-column candidates are both wrong on their own terms:
        // `events.briefing` and `events.banner_image_url` belong to the *container*, are
        // already served at the top level of this same response, and substituting them would
        // report the event's briefing as the mission's and paint every mission on an event with
        // the same banner. The thumbnail case is the worse of the two: a confidently-wrong image
        // is less recoverable than a missing one, because omitting the key is precisely what
        // lets the client render its own "no thumbnail" placeholder.
        //
        // Consistent with every other read of these columns, and with the write side: `PATCH
        // /missions/:id` binds `briefing`/`thumbnail_url` straight from the request, so the API
        // itself already stores `''`. NULL and `''` are one observable state; this makes the
        // read agree. `Option` is NOT the fix — see
        // `match_telemetry::models::match_record::Match` for the recorded rejection.
        let Some((title, terrain, game_mode, briefing, thumbnail_url)): Option<(
            String,
            crate::missions::models::mission::TerrainType,
            crate::missions::models::mission::GameMode,
            String,
            String,
        )> = sqlx::query_as(
            "SELECT title, terrain, game_mode, COALESCE(briefing, '') AS briefing, \
                 COALESCE(thumbnail_url, '') AS thumbnail_url \
                 FROM missions WHERE id = $1 AND deleted_at IS NULL",
        )
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
