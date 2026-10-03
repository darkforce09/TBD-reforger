//! Attaching a mission to an event and detaching it again.
//!
//! **Role:** the attach and detach routes of an event: the request shape, the pre-checks that
//! answer a missing event or mission first, and the administrator session the writes run for.
//!
//! **Position:** an attach *snapshots* the mission's ORBAT into `orbat_slots` rows owned by the
//! new `event_missions` row through
//! [`mission_attachment`](crate::services::event_authoring::mission_attachment), the
//! one writer of that snapshot, which the `staging-fixtures` host tool also writes through:
//! [`AttachmentTemplate`] refuses an unreadable template, a template that seats nobody and a
//! faction that cannot match its armory before [`attach_mission`] writes anything. A detach
//! releases the attachment's reservations.
//!
//! **Signals & state:** none; each route runs one transaction.
//!
//! **Invariants:** the administrator's authority is rechecked under the event-scope locks before
//! any write; an attachment or detachment commits with its audit row or not at all.
//!
//! @contract event-schedule.schema.json#/definitions/EventMissionAttachment

use api_identifiers::{EventMissionId, MissionId};
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::models::EventMission;
use crate::services::event_authoring::mission_attachment::{
    AttachmentAuthority, AttachmentTemplate, attach_mission,
};
use crate::services::event_lookup::load_event;
use crate::services::event_reservations::event_administration::{
    lock_event_scope, normalize_schedule_time,
};
use crate::services::event_reservations::reservation_release::{
    release_mission_registrations, release_reasons, release_unused_allocations,
};
use crate::services::event_reservations::waitlist_promotion::promote_waiting_participants;
use api_audit_log::required_audit::append_actor_audit;
use api_foundation::error_handling::api_error::ApiError;
use api_foundation::http::path_parameters::PathParams;
use api_http_layer::middleware::AdminUser;
use api_state::AppState;
use mission_model::orbat::OrbatSquadTemplate;

/// The body of `POST /api/v1/events/{id}/missions`: the mission, its start and its ORBAT.
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
/// Re-attach after detach is the same path. `idx_event_mission` is unique on
/// `(event_id, mission_id)` — a second attach of a mission still on the event is a **409**,
/// not a 500 from an unmapped unique violation ([`attach_mission`] maps it).
///
/// @route POST /api/v1/events/:id/missions
pub async fn add_event_mission(
    State(state): State<AppState>,
    administrator: AdminUser,
    PathParams(id): PathParams<String>,
    body: Result<Json<AddMissionInput>, JsonRejection>,
) -> Result<(StatusCode, Json<EventMission>), ApiError> {
    let ev = load_event(&state.pool, &id).await?;
    let Json(input) = body.map_err(ApiError::from_json_rejection)?;
    let Some(start_time) = input.start_time else {
        return Err(ApiError::bad_request(
            "mission_id and start_time are required",
        ));
    };
    let start_time = normalize_schedule_time(start_time)?;
    let Ok(mission_id) = input.mission_id.parse::<MissionId>() else {
        return Err(ApiError::bad_request("invalid mission_id"));
    };
    let exists: Option<MissionId> =
        sqlx::query_scalar("SELECT id FROM missions WHERE id = $1 AND deleted_at IS NULL")
            .bind(mission_id)
            .fetch_optional(&state.pool)
            .await?;
    if exists.is_none() {
        return Err(ApiError::not_found("mission not found"));
    }
    let template = AttachmentTemplate::resolve(&state.pool, mission_id, input.orbat).await?;
    let authority = AttachmentAuthority::AdministratorSession {
        administrator: &administrator.0,
        config: &state.cfg,
    };
    let mut tx = state.pool.begin().await?;
    let em = attach_mission(
        &mut tx, ev.id, mission_id, start_time, &template, &authority,
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(em)))
}

/// `DELETE /api/v1/events/:id/missions/:emid` — detach a mission (admin).
///
/// @route DELETE /api/v1/events/:id/missions/:emid
pub async fn remove_event_mission(
    State(state): State<AppState>,
    administrator: AdminUser,
    PathParams((id, emid)): PathParams<(String, String)>,
) -> Result<StatusCode, ApiError> {
    let ev = load_event(&state.pool, &id).await?;
    let Ok(em_id) = emid.parse::<EventMissionId>() else {
        return Err(ApiError::bad_request("invalid mission id"));
    };
    let mut tx = state.pool.begin().await?;
    let mut scope = lock_event_scope(&mut tx, ev.id, &administrator.0, &state.cfg).await?;
    if !scope.active_missions.contains(&em_id) {
        return Err(ApiError::not_found("mission not found in event"));
    }
    let released = release_mission_registrations(
        &mut tx,
        std::slice::from_ref(&em_id),
        release_reasons::MISSION_REMOVED,
    )
    .await?;
    release_unused_allocations(&mut tx, ev.id, &released, release_reasons::MISSION_REMOVED).await?;
    sqlx::query("UPDATE event_missions SET deleted_at = clock_timestamp() WHERE id = $1")
        .bind(em_id)
        .execute(&mut *tx)
        .await?;
    // Places released by the removal may admit participants waiting for the remaining missions.
    scope.active_missions.retain(|mission| *mission != em_id);
    promote_waiting_participants(&mut tx, &scope, &state.cfg.discord_guild_id).await?;
    append_actor_audit(&mut tx, &administrator.0.discord_id, "event.mission_removed", "event_mission", &em_id.to_string(), "Removed mission from operational views and released reservations; signup and attendance history remain available").await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
#[path = "tests/event_mission_attachment.rs"]
mod tests;
