//! The Event Hub dossier for one event, projected for the viewer's access.
//!
//! A viewer the event policy admits sees every operational mission. A viewer admitted only by a
//! squad or slot policy sees the event summary without its briefing, and only the missions,
//! seats and factions those policies admit. An event the viewer may not see is indistinguishable
//! from a missing one. Status is the effective status of [`crate::operations::services::event_status_rules`].

use std::collections::{BTreeSet, HashMap};

use axum::extract::{Path, State};
use axum::response::Json;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AuthUser;
use crate::core::wire_format::{rfc3339_utc, rfc3339_utc_opt};
use crate::missions::models::mission::MissionArmory;
use crate::operations::models::event_viewer_access::{
    EventViewerAccess, EventVisibilityLevel, ReservationQuotaAvailability,
};
use crate::operations::models::reservation_quota::ReservationQuotaKind;
use crate::operations::models::{EventMission, RegistrationState};
use crate::operations::services::event_access::evaluation::policy_admits;
use crate::operations::services::event_access::slot_eligibility::load_policy_slots;
use crate::operations::services::event_access::visibility::{EventVisibility, viewer_event_access};
use crate::operations::services::event_lookup::load_event;
use crate::operations::services::event_reservations::event_administration::allocation_usage;
use crate::operations::services::event_reservations::participant_allocations::load_reservation_quotas;
use crate::operations::services::event_reservations::quota_availability::quota_availability;

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
    #[serde(with = "rfc3339_utc")]
    start_time: DateTime<Utc>,
    factions: Vec<String>,
    armory_by_faction: Vec<ArmoryFactionDto>,
    filled: i64,
    total: i64,
    /// Some seat of this mission admits the viewer under current membership authority.
    viewer_eligible: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    my_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    my_reservation_state: Option<RegistrationState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    my_attendance_state: Option<RegistrationState>,
    #[serde(skip_serializing_if = "Option::is_none")]
    my_slot_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    my_release_reason: Option<String>,
    /// When the viewer's reservation was first released; retained with the withdrawn signup.
    #[serde(with = "rfc3339_utc_opt", skip_serializing_if = "Option::is_none")]
    my_withdrawn_at: Option<DateTime<Utc>>,
    /// One-based position in this mission's waiting queue.
    #[serde(skip_serializing_if = "Option::is_none")]
    my_waiting_position: Option<i64>,
}

/// `shown` limits the armory to the factions of the seats a partial viewer may see.
async fn armory_by_faction(
    connection: &mut sqlx::PgConnection,
    mission_id: Uuid,
    shown: Option<&BTreeSet<String>>,
) -> Result<Vec<ArmoryFactionDto>, ApiError> {
    let items: Vec<MissionArmory> = sqlx::query_as(
        "SELECT id, mission_id, faction, category, item_name, quantity, COALESCE(icon, '') AS icon, sort_order FROM mission_armories WHERE mission_id = $1 ORDER BY sort_order ASC",
    )
    .bind(mission_id)
    .fetch_all(connection)
    .await?;
    let mut order: Vec<String> = Vec::new();
    let mut groups: HashMap<String, Vec<MissionArmory>> = HashMap::new();
    for item in items
        .into_iter()
        .filter(|item| shown.is_none_or(|factions| factions.contains(&item.faction)))
    {
        if !groups.contains_key(&item.faction) {
            order.push(item.faction.clone());
        }
        groups.entry(item.faction.clone()).or_default().push(item);
    }
    Ok(order
        .into_iter()
        .map(|faction| ArmoryFactionDto {
            items: groups.remove(&faction).unwrap_or_default(),
            faction,
        })
        .collect())
}

#[derive(sqlx::FromRow)]
struct ViewerRegistration {
    state: RegistrationState,
    slot_id: Option<Uuid>,
    reservation_state: RegistrationState,
    attendance_state: Option<RegistrationState>,
    release_reason: Option<String>,
    withdrawn_at: Option<DateTime<Utc>>,
    waiting_position: Option<i64>,
}

/// `GET /api/v1/events/:id` — Event Hub (event + nested mission dossiers).
///
/// @route GET /api/v1/events/:id
pub async fn get_event(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let event = load_event(&state.pool, &id).await?;
    let mut tx = state.pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await?;
    let access = viewer_event_access(
        &mut tx,
        &user.discord_id,
        user.role == "admin",
        &[event.id],
        &state.cfg.discord_guild_id,
    )
    .await?
    .remove(&event.id)
    .filter(|access| access.visibility.is_visible())
    .ok_or_else(|| ApiError::not_found("event not found"))?;
    let attachments: Vec<EventMission> = sqlx::query_as(
        "SELECT id, event_id, mission_id, start_time, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM event_missions WHERE deleted_at IS NULL AND event_id = $1 ORDER BY start_time ASC",
    )
    .bind(event.id)
    .fetch_all(&mut *tx)
    .await?;
    let attachment_ids: Vec<Uuid> = attachments.iter().map(|em| em.id).collect();
    let slots = load_policy_slots(&mut tx, &attachment_ids).await?;
    let mut missions = Vec::with_capacity(attachments.len());
    for em in attachments {
        // `briefing` and `thumbnail_url` are nullable without defaults; `''` omits the key,
        // which is the true statement that the mission has none.
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
        .fetch_optional(&mut *tx)
        .await?
        else {
            continue;
        };
        let shown: Vec<_> = slots
            .iter()
            .filter(|slot| slot.event_mission_id == em.id && access.visibility.shows_slot(slot.id))
            .collect();
        if shown.is_empty() && access.visibility != EventVisibility::Full {
            continue;
        }
        let mut factions: Vec<String> = Vec::new();
        for slot in &shown {
            if !factions.contains(&slot.faction) {
                factions.push(slot.faction.clone());
            }
        }
        let viewer_eligible = shown
            .iter()
            .any(|slot| access.context.slot_admits(slot, &access.facts.current));
        let registration: Option<ViewerRegistration> = sqlx::query_as(
            "SELECT r.state, r.slot_id, r.reservation_state, r.attendance_state, r.release_reason,
                r.withdrawn_at, CASE WHEN r.reservation_state = 'waitlisted' THEN (
                    SELECT count(*) FROM event_registrations queued
                    WHERE queued.event_mission_id = r.event_mission_id AND queued.reservation_state = 'waitlisted'
                      AND (queued.queue_entered_at, queued.id) <= (r.queue_entered_at, r.id)) END AS waiting_position
             FROM event_registrations r WHERE r.event_mission_id = $1 AND r.discord_id = $2",
        )
        .bind(em.id)
        .bind(&user.discord_id)
        .fetch_optional(&mut *tx)
        .await?;
        let shown_factions: BTreeSet<String> = factions.iter().cloned().collect();
        let mine = registration.as_ref();
        missions.push(EventMissionDossier {
            event_mission_id: em.id.to_string(),
            mission_id: em.mission_id.to_string(),
            title,
            terrain: terrain.as_str().to_string(),
            game_mode: game_mode.as_str().to_string(),
            briefing,
            thumbnail_url,
            start_time: em.start_time,
            armory_by_faction: armory_by_faction(
                &mut tx,
                em.mission_id,
                (access.visibility != EventVisibility::Full).then_some(&shown_factions),
            )
            .await?,
            factions,
            filled: shown
                .iter()
                .filter(|slot| slot.assigned_to.is_some())
                .count() as i64,
            total: shown.len() as i64,
            viewer_eligible,
            my_state: mine
                .map(|r| r.state.as_str().to_owned())
                .unwrap_or_default(),
            my_reservation_state: mine.map(|r| r.reservation_state),
            my_attendance_state: mine.and_then(|r| r.attendance_state),
            my_slot_id: mine.and_then(|r| r.slot_id).map(|slot| slot.to_string()),
            my_release_reason: mine.and_then(|r| r.release_reason.clone()),
            my_withdrawn_at: mine.and_then(|r| r.withdrawn_at),
            my_waiting_position: mine.and_then(|r| r.waiting_position),
        });
    }
    let usage = allocation_usage(&mut tx, event.id, &attachment_ids).await?;
    let quotas = load_reservation_quotas(&mut tx, event.id).await?;
    let reservation_quotas: Vec<ReservationQuotaAvailability> =
        quota_availability(&quotas, usage, access.facts.observed_at);
    let remaining_event_places = (event.max_slots > 0)
        .then(|| (event.max_slots as u64).saturating_sub(usage.total().unwrap_or(u64::MAX)));
    let admitted_now = policy_admits(&access.context.policy, &access.facts.current)
        || slots
            .iter()
            .any(|slot| access.context.slot_admits(slot, &access.facts.current));
    let pending = !admitted_now
        && (access
            .facts
            .pending_evidence_admits(&access.context.policy, &state.cfg.discord_guild_id)
            || slots.iter().any(|slot| {
                access.facts.pending_evidence_admits(
                    access.context.effective_slot_policy(slot).0,
                    &state.cfg.discord_guild_id,
                )
            }));
    let viewer_access = EventViewerAccess {
        visibility: match access.visibility {
            EventVisibility::Full => EventVisibilityLevel::Full,
            _ => EventVisibilityLevel::Partial,
        },
        quota_class: if access.facts.current.tbd_member {
            ReservationQuotaKind::Member
        } else {
            ReservationQuotaKind::Guest
        },
        membership_verification_pending: pending,
    };
    tx.commit().await?;

    let partial = access.visibility != EventVisibility::Full;
    let mut body = serde_json::to_value(&event)
        .map_err(|_| ApiError::internal("event serialization failed"))?;
    let object = body
        .as_object_mut()
        .ok_or_else(|| ApiError::internal("event serialization failed"))?;
    if partial {
        // The event briefing belongs to the event policy's audience.
        object.remove("briefing");
    }
    let to_value = |value: serde_json::Result<Value>| {
        value.map_err(|_| ApiError::internal("event hub serialization failed"))
    };
    object.insert("missions".into(), to_value(serde_json::to_value(missions))?);
    object.insert(
        "viewer_access".into(),
        to_value(serde_json::to_value(viewer_access))?,
    );
    object.insert(
        "reservation_quotas".into(),
        to_value(serde_json::to_value(reservation_quotas))?,
    );
    object.insert(
        "remaining_event_places".into(),
        to_value(serde_json::to_value(remaining_event_places))?,
    );
    Ok(Json(body))
}
