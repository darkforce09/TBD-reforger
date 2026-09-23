//! Event writes validate current authority and serialize schedule, capacity and reservation changes.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer};
use sqlx::{Postgres, QueryBuilder};
use uuid::Uuid;

use crate::administration::services::required_audit::append_actor_audit;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AdminUser;
use crate::core::text::http_url_guard::is_http_url;
use crate::operations::models::{Event, EventStatus};
use crate::operations::services::event_reservations::event_administration::{
    load_locked_event, lock_event_scope, normalize_schedule_time, release_event_reservations,
    reschedule_missions, validate_capacity,
};
use crate::operations::services::event_reservations::waitlist_promotion::promote_waiting_participants;
use crate::operations::services::event_status_rules::{
    can_transition, is_pre_start, valid_event_status,
};

fn validated_banner_image_url(raw: &str) -> Result<String, ApiError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || is_http_url(trimmed) {
        return Ok(trimmed.to_string());
    }
    Err(ApiError::bad_request(
        "banner_image_url must be an absolute http:// or https:// URL",
    ))
}

fn check_name_override(n: &str) -> Result<(), ApiError> {
    if !n.is_empty() && n.trim().is_empty() {
        return Err(ApiError::bad_request(
            "name_override must not be blank — send \"\" to clear it and fall back to the \
             mission's title",
        ));
    }
    Ok(())
}

fn present_option<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

async fn require_server<'e, E: sqlx::Executor<'e, Database = Postgres>>(
    executor: E,
    id: Uuid,
) -> Result<(), ApiError> {
    let found: Option<Uuid> = sqlx::query_scalar("SELECT id FROM servers WHERE id = $1")
        .bind(id)
        .fetch_optional(executor)
        .await?;
    if found.is_none() {
        return Err(ApiError::bad_request(
            "server_id does not name a known server",
        ));
    }
    Ok(())
}

async fn require_event_modpack<'e, E: sqlx::Executor<'e, Database = Postgres>>(
    executor: E,
    id: Uuid,
) -> Result<(), ApiError> {
    let found: Option<Uuid> = sqlx::query_scalar("SELECT id FROM modpacks WHERE id = $1")
        .bind(id)
        .fetch_optional(executor)
        .await?;
    if found.is_none() {
        return Err(ApiError::bad_request(
            "modpack_id does not name a known modpack",
        ));
    }
    Ok(())
}

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
    server_id: Option<Uuid>,
    #[serde(default)]
    modpack_id: Option<Uuid>,
}

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
    let start_time = normalize_schedule_time(start_time)?;
    let Some(status) = valid_event_status(&input.status) else {
        return Err(ApiError::bad_request("invalid status"));
    };
    if !is_pre_start(status) {
        return Err(ApiError::bad_request(
            "an event may only be created as scheduled, open or locked",
        ));
    }
    check_name_override(&input.name_override)?;
    let banner_image_url = validated_banner_image_url(&input.banner_image_url)?;
    if let Some(sid) = input.server_id {
        require_server(&state.pool, sid).await?;
    }
    if let Some(mid) = input.modpack_id {
        require_event_modpack(&state.pool, mid).await?;
    }
    let mut tx = state.pool.begin().await?;
    crate::identity_and_access::services::identity_ownership::lock_accounts(
        &mut tx,
        std::slice::from_ref(&_a.0.discord_id),
    )
    .await?;
    let actor =
        crate::identity_and_access::services::session_authorization::authorize_on_connection(
            &mut tx,
            &state.cfg,
            &_a.0.session_claims,
        )
        .await?;
    if actor.role != "admin" {
        return Err(ApiError::forbidden("insufficient role"));
    }
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO events (name_override, start_time, briefing, banner_image_url, status, \
         registration_locked, max_slots, created_by, server_id, modpack_id, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, now(), now()) RETURNING id",
    )
    .bind(&input.name_override)
    .bind(start_time)
    .bind(&input.briefing)
    .bind(&banner_image_url)
    .bind(status)
    .bind(input.registration_locked)
    .bind(input.max_slots)
    .bind(&_a.0.discord_id)
    .bind(input.server_id)
    .bind(input.modpack_id)
    .fetch_one(&mut *tx).await?;
    let ev = load_locked_event(&mut tx, id).await?;
    append_actor_audit(
        &mut tx,
        &_a.0.discord_id,
        "event.created",
        "event",
        &id.to_string(),
        "Event created with TBD-member access",
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(ev)))
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
    #[serde(default, deserialize_with = "present_option")]
    server_id: Option<Option<Uuid>>,
    #[serde(default, deserialize_with = "present_option")]
    modpack_id: Option<Option<Uuid>>,
}

/// @route PATCH /api/v1/events/:id
pub async fn update_event(
    State(state): State<AppState>,
    _a: AdminUser,
    Path(id): Path<String>,
    body: Result<Json<PatchEventInput>, JsonRejection>,
) -> Result<Json<Event>, ApiError> {
    let Json(mut input) = body.map_err(|_| ApiError::bad_request("invalid body"))?;
    input.start_time = input.start_time.map(normalize_schedule_time).transpose()?;
    let event_id = Uuid::parse_str(&id).map_err(|_| ApiError::bad_request("invalid id"))?;
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
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let event_id = Uuid::parse_str(&id).map_err(|_| ApiError::bad_request("invalid id"))?;
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
