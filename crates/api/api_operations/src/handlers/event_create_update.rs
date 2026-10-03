//! Event writes validate current authority and serialize schedule, capacity and reservation changes.
//!
//! **Role:** the `POST`, `PATCH` and `DELETE` routes of an event: request shapes, the
//! administrator's authority recheck, and the event-scope locks of a change.
//!
//! **Position:** `POST /api/v1/events` validates through
//! [`EventCreation`] and writes through [`event_creation::create_event`], the service the
//! `staging-fixtures` host tool also writes through; `PATCH` applies the same field validators to
//! the fields it changes under the event scope of `services::event_reservations`.
//!
//! **Signals & state:** none; each route runs one transaction.
//!
//! **Invariants:** every write rechecks the administrator's authority on its own transaction
//! before it changes a row; a change commits with its audit row or not at all.
//!
//! @contract event-schedule.schema.json#/definitions/EventCreation
//! @contract event-schedule.schema.json#/definitions/EventChange

use api_identifiers::{EventId, ModpackId, ServerId};
use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::http::StatusCode;
use axum::response::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer};
use sqlx::{Postgres, QueryBuilder};

use crate::models::{Event, EventStatus};
use crate::services::event_authoring::event_creation::{
    self, EventCreation, EventCreationRequest, check_name_override, require_event_modpack,
    require_server, validated_banner_image_url,
};
use crate::services::event_reservations::event_administration::{
    load_locked_event, lock_event_scope, normalize_schedule_time, release_event_reservations,
    reschedule_missions, validate_capacity,
};
use crate::services::event_reservations::waitlist_promotion::promote_waiting_participants;
use crate::services::event_status_rules::{can_transition, is_pre_start, valid_event_status};
use api_audit_log::required_audit::append_actor_audit;
use api_caller_identity::identity_ownership::lock_accounts;
use api_caller_identity::session_authorization::authorize_on_connection;
use api_foundation::error_handling::api_error::ApiError;
use api_foundation::http::path_parameters::PathParams;
use api_http_layer::middleware::AdminUser;
use api_state::AppState;

fn present_option<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

/// The body of `POST /api/v1/events`: the new event's schedule, presentation and capacity.
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
    #[serde(default)]
    server_id: Option<ServerId>,
    #[serde(default)]
    modpack_id: Option<ModpackId>,
}

/// @route POST /api/v1/events
pub async fn create_event(
    State(state): State<AppState>,
    administrator: AdminUser,
    body: Result<Json<CreateEventInput>, JsonRejection>,
) -> Result<(StatusCode, Json<Event>), ApiError> {
    let Json(input) = body.map_err(ApiError::from_json_rejection)?;
    let creation = EventCreation::new(EventCreationRequest {
        start_time: input.start_time,
        name_override: input.name_override,
        briefing: input.briefing,
        banner_image_url: input.banner_image_url,
        max_slots: input.max_slots,
        registration_locked: input.registration_locked,
        status: input.status,
        server_id: input.server_id,
        modpack_id: input.modpack_id,
    })?;
    let mut tx = state.pool.begin().await?;
    lock_accounts(&mut tx, std::slice::from_ref(&administrator.0.discord_id)).await?;
    let actor =
        authorize_on_connection(&mut tx, &state.cfg, &administrator.0.session_claims).await?;
    if actor.role != "admin" {
        return Err(ApiError::forbidden("insufficient role"));
    }
    let event =
        event_creation::create_event(&mut tx, &creation, &administrator.0.discord_id).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(event)))
}

/// The body of `PATCH /api/v1/events/{id}`: each present field replaces the stored one.
#[derive(Debug, Deserialize)]
pub struct PatchEventInput {
    start_time: Option<DateTime<Utc>>,
    max_slots: Option<i64>,
    name_override: Option<String>,
    briefing: Option<String>,
    banner_image_url: Option<String>,
    registration_locked: Option<bool>,
    status: Option<String>,
    #[serde(default, deserialize_with = "present_option")]
    server_id: Option<Option<ServerId>>,
    #[serde(default, deserialize_with = "present_option")]
    modpack_id: Option<Option<ModpackId>>,
}

/// @route PATCH /api/v1/events/:id
pub async fn update_event(
    State(state): State<AppState>,
    _a: AdminUser,
    PathParams(id): PathParams<String>,
    body: Result<Json<PatchEventInput>, JsonRejection>,
) -> Result<Json<Event>, ApiError> {
    let Json(mut input) = body.map_err(ApiError::from_json_rejection)?;
    input.start_time = input.start_time.map(normalize_schedule_time).transpose()?;
    let event_id: EventId = id
        .parse()
        .map_err(|_| ApiError::bad_request("invalid id"))?;
    let mut tx = state.pool.begin().await?;
    let mut scope = lock_event_scope(&mut tx, event_id, &_a.0, &state.cfg).await?;
    let ev = scope.event.clone();
    let ev = &ev;
    if let Some(maximum) = input.max_slots {
        validate_capacity(&mut tx, &scope, maximum).await?;
    }

    let mut requested: Option<EventStatus> = None;
    if let Some(s) = &input.status {
        let Some(to) = valid_event_status(s) else {
            return Err(ApiError::bad_request("invalid status"));
        };
        requested = Some(to);
        if !can_transition(ev.status, to) {
            return Err(ApiError::conflict(format!(
                "cannot move an event from {} to {}",
                ev.status.as_str(),
                to.as_str()
            )));
        }
        if to != ev.status && is_pre_start(to) {
            let start = input.start_time.unwrap_or(ev.start_time);
            let in_future: bool = sqlx::query_scalar("SELECT $1 > statement_timestamp()")
                .bind(start)
                .fetch_one(&mut *tx)
                .await?;
            if !in_future {
                return Err(ApiError::conflict(format!(
                    "cannot move an event to {} once its start time has passed — \
                     reschedule it in the same request to postpone it",
                    to.as_str()
                )));
            }
        }
    }

    if let Some(n) = &input.name_override {
        check_name_override(n)?;
    }
    if let Some(Some(sid)) = input.server_id {
        require_server(&mut *tx, sid).await?;
    }
    if let Some(Some(mid)) = input.modpack_id {
        require_event_modpack(&mut *tx, mid).await?;
    }
    let banner_image_url = input
        .banner_image_url
        .as_deref()
        .map(validated_banner_image_url)
        .transpose()?;

    if let Some(start) = input.start_time {
        reschedule_missions(&mut tx, &scope, start).await?;
    }
    if requested == Some(EventStatus::Cancelled) {
        release_event_reservations(&mut tx, &scope, "event_cancelled").await?;
    }
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
    if let Some(u) = &banner_image_url {
        qb.push(", banner_image_url = ").push_bind(u.clone());
    }
    if let Some(l) = input.registration_locked {
        qb.push(", registration_locked = ").push_bind(l);
    }
    if let Some(status) = requested {
        qb.push(", status = ").push_bind(status);
    }
    if let Some(sid) = input.server_id {
        qb.push(", server_id = ").push_bind(sid);
    }
    if let Some(mid) = input.modpack_id {
        qb.push(", modpack_id = ").push_bind(mid);
    }
    qb.push(" WHERE id = ").push_bind(ev.id);
    qb.build().execute(&mut *tx).await.map_err(ApiError::from)?;

    let updated = load_locked_event(&mut tx, ev.id).await?;
    // More capacity, unlocked or reopened registration may admit waiting participants now.
    scope.event = updated.clone();
    promote_waiting_participants(&mut tx, &scope, &state.cfg.discord_guild_id).await?;
    append_actor_audit(
        &mut tx,
        &_a.0.discord_id,
        "event.updated",
        "event",
        &id,
        "Event settings and dependent schedule updated",
    )
    .await?;
    tx.commit().await?;
    Ok(Json(updated))
}

/// @route DELETE /api/v1/events/:id
pub async fn delete_event(
    State(state): State<AppState>,
    _a: AdminUser,
    PathParams(id): PathParams<String>,
) -> Result<StatusCode, ApiError> {
    let event_id: EventId = id
        .parse()
        .map_err(|_| ApiError::bad_request("invalid id"))?;
    let mut tx = state.pool.begin().await?;
    let scope = lock_event_scope(&mut tx, event_id, &_a.0, &state.cfg).await?;
    release_event_reservations(&mut tx, &scope, "event_deleted").await?;
    sqlx::query("UPDATE events SET deleted_at = now() WHERE id = $1")
        .bind(event_id)
        .execute(&mut *tx)
        .await?;
    append_actor_audit(
        &mut tx,
        &_a.0.discord_id,
        "event.deleted",
        "event",
        &id,
        "Event removed from operational views; reservations released and history retained",
    )
    .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
#[path = "tests/event_create_update.rs"]
mod tests;
