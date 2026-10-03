//! Home dashboard aggregation: many best-effort, null-safe lookups composed into one response.
//!
//! **Role:** the `GET /api/v1/dashboard` handler.
//! **Position:** reads the events, attachments and slots of `api_operations`, the mission title
//! and terrain of `api_missions`, the current modpack and the announcements of
//! `api_community_content`, and the fleet overview of [`crate::services::fleet_overview`];
//! called by the router through [`crate::routes()`].
//! **Signals & state:** none; one set of reads per request.
//! **Invariants:** a missing next event, assignment or modpack is `null`, never a failed
//! dashboard; every emitted time is RFC 3339 UTC.
//!
//! @contract command-center.schema.json#/definitions/Dashboard
//! @contract command-center.schema.json#/definitions/DashboardEvent
//! @contract command-center.schema.json#/definitions/DashboardAssignment

use axum::extract::State;
use axum::response::Json;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{Value, json};

use crate::services::fleet_overview::load_fleet_overview;
use api_community_content::models::announcement::Announcement;
use api_community_content::services::modpack_lookup::load_current_modpack;
use api_foundation::error_handling::api_error::ApiError;
use api_http_layer::middleware::AuthUser;
use api_missions::services::mission_lookup::mission_title_terrain;
use api_operations::models::{Event, EventMission, OrbatSlot};
use api_state::AppState;
use fleet_wire_contract::rfc3339_timestamps::rfc3339_utc;

#[derive(Debug, Serialize)]
struct EventSummary {
    event_id: String,
    name: String,
    terrain: String,
    #[serde(with = "rfc3339_utc")]
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

/// `GET /api/v1/dashboard` — next op, my assignment, the configured fleet, modpack, news.
///
/// @route GET /api/v1/dashboard
pub async fn get_dashboard(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Value>, ApiError> {
    let me = &user.discord_id;
    let pool = &state.pool;

    // Next upcoming operation.
    //
    // Prefer an event the caller themselves created when one is upcoming. The global
    // `ORDER BY start_time ASC LIMIT 1` alone is a ratchet on never-pruned gate DBs: any
    // residue row with an earlier start_time steals the slot. Prefer-mine keeps the
    // community fallback for users who have never created an event (enlisted browsing the
    // home bento) while letting an admin/mission_maker fixture win deterministically.
    let next_event: Option<EventSummary> = {
        let ev: Option<Event> = {
            let mine: Option<Event> = sqlx::query_as(
                "SELECT id, COALESCE(name_override, '') AS name_override, start_time, COALESCE(briefing, '') AS briefing, COALESCE(banner_image_url, '') AS banner_image_url, status, registration_locked, max_slots, created_by, server_id, modpack_id, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM events WHERE start_time > now() \
                 AND status::text IN ('scheduled', 'open', 'live') AND deleted_at IS NULL \
                 AND created_by = $1 \
                 ORDER BY start_time ASC LIMIT 1",
            )
            .bind(me)
            .fetch_optional(pool)
            .await?;
            match mine {
                Some(ev) => Some(ev),
                None => {
                    sqlx::query_as(
                        "SELECT id, COALESCE(name_override, '') AS name_override, start_time, COALESCE(briefing, '') AS briefing, COALESCE(banner_image_url, '') AS banner_image_url, status, registration_locked, max_slots, created_by, server_id, modpack_id, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM events WHERE start_time > now() \
                         AND status::text IN ('scheduled', 'open', 'live') AND deleted_at IS NULL \
                         ORDER BY start_time ASC LIMIT 1",
                    )
                    .fetch_optional(pool)
                    .await?
                }
            }
        };
        match ev {
            Some(ev) => {
                let em: Option<EventMission> = sqlx::query_as(
                    "SELECT id, event_id, mission_id, start_time, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM event_missions WHERE deleted_at IS NULL AND event_id = $1 ORDER BY start_time ASC LIMIT 1",
                )
                .bind(ev.id)
                .fetch_optional(pool)
                .await?;
                let mt = match &em {
                    Some(em) => mission_title_terrain(pool, em.mission_id).await?,
                    None => None,
                };
                let registered: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM event_registrations \
                     JOIN event_missions ON event_missions.id = event_registrations.event_mission_id \
                     WHERE event_missions.deleted_at IS NULL AND event_missions.event_id = $1 \
                       AND event_registrations.reservation_state::text IN ('registered', 'waitlisted', 'legacy_unknown')",
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
        // `orbat_slots.callsign`/`loadout`/`tag` are NULLABLE columns behind non-optional
        // `String` fields, so the nullable ones MUST be coalesced here — the column list is
        // spelled out for exactly that reason and is the same one the other `OrbatSlot` read
        // sites use (`api_operations/src/handlers/{member_service_record,orbat_view,event_listing,
        // slot_assignment,slot_registration}.rs`). A bare `orbat_slots.*` 500s the whole
        // dashboard on a real NULL (measured: *"error occurred while decoding column `tag`:
        // unexpected null"* for the two operator users holding a NULL-`tag` slot). Do NOT
        // "fix" that by making the model fields `Option` — see the rejection recorded on
        // `api_match_telemetry::models::match_record::Match`. Every column is table-qualified
        // because the joins make `id`/`start_time` ambiguous.
        let slot: Option<OrbatSlot> = sqlx::query_as(
            "SELECT orbat_slots.id, orbat_slots.event_mission_id, orbat_slots.faction, \
             orbat_slots.squad, COALESCE(orbat_slots.callsign, '') AS callsign, orbat_slots.role, \
             COALESCE(orbat_slots.loadout, '') AS loadout, COALESCE(orbat_slots.tag, '') AS tag, \
             orbat_slots.slot_index, orbat_slots.assigned_to, orbat_slots.assigned_at \
             FROM orbat_slots \
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
                    sqlx::query_as("SELECT id, event_id, mission_id, start_time, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM event_missions WHERE deleted_at IS NULL AND id = $1")
                        .bind(slot.event_mission_id)
                        .fetch_optional(pool)
                        .await?;
                let (event_id, mission_id) = em
                    .as_ref()
                    .map(|em| (em.event_id, em.mission_id))
                    .unwrap_or_else(|| (uuid::Uuid::nil().into(), uuid::Uuid::nil().into()));
                let ev_name: String = sqlx::query_scalar(
                    "SELECT COALESCE(name_override, '') FROM events WHERE id = $1",
                )
                .bind(event_id)
                .fetch_optional(pool)
                .await?
                .unwrap_or_default();
                let name = if ev_name.is_empty() {
                    mission_title_terrain(pool, mission_id)
                        .await?
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

    // The configured fleet: every active server with its status, and the fleet totals.
    let fleet = load_fleet_overview(pool).await?;

    let current_modpack = load_current_modpack(pool).await?;

    let recent: Vec<Announcement> = sqlx::query_as(
        "SELECT id, title, body, COALESCE(snippet, '') AS snippet, tag, COALESCE(thumbnail_url, '') AS thumbnail_url, author_id, status, is_pinned, pushed_to_discord, COALESCE(discord_message_id, '') AS discord_message_id, published_at, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM announcements WHERE status = 'published' AND deleted_at IS NULL \
         ORDER BY published_at DESC LIMIT 3",
    )
    .fetch_all(pool)
    .await?;

    Ok(Json(json!({
        "next_event": next_event,
        "my_assignment": my_assignment,
        "fleet": fleet,
        "current_modpack": current_modpack,
        "recent_announcements": recent,
    })))
}
