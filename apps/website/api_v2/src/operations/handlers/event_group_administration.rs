//! Administrator routes for event groups: manager-maintained rosters and verified partner-guild
//! groups. Provenance is recorded on every group and roster entry; partner membership is never
//! self-asserted and comes only from bot-authenticated Discord observations.

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use sqlx::{PgConnection, types::Json as SqlJson};
use uuid::Uuid;

use super::event_access_administration::finish_access_change;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AdminUser;
use crate::identity_and_access::services::discord_membership_enrollment::enroll_event_partner_guild;
use crate::operations::models::event_access_administration::{
    AccessChangeOutcome, AccessRevisionPrecondition, EventGroupChange, EventGroupCreation,
};
use crate::operations::models::event_group::EventGroupSource;
use crate::operations::services::access_administration::persistence::{
    advance_access_revision, group_is_referenced, validate_group,
};
use crate::operations::services::event_access::context::EventAccessContext;
use crate::operations::services::event_reservations::event_administration::lock_event_scope;

fn ids(event: &str, group: &str) -> Result<(Uuid, Uuid), ApiError> {
    Ok((
        Uuid::parse_str(event).map_err(|_| ApiError::bad_request("invalid id"))?,
        Uuid::parse_str(group).map_err(|_| ApiError::bad_request("invalid group id"))?,
    ))
}

fn body<T>(input: Result<Json<T>, JsonRejection>) -> Result<T, ApiError> {
    input.map(|Json(value)| value).map_err(|rejection| {
        ApiError::bad_request(format!("invalid body: {}", rejection.body_text()))
    })
}

fn precondition(
    input: Result<Query<AccessRevisionPrecondition>, QueryRejection>,
) -> Result<i64, ApiError> {
    input
        .map(|Query(value)| value.expected_access_revision)
        .map_err(|_| ApiError::bad_request("expected_access_revision query parameter is required"))
}

/// The group's current source, if it belongs to the event and has not been removed.
async fn group_source(
    connection: &mut PgConnection,
    event: Uuid,
    group: Uuid,
) -> Result<EventGroupSource, ApiError> {
    let source: Option<SqlJson<EventGroupSource>> = sqlx::query_scalar(
        "SELECT source FROM event_groups WHERE id = $1 AND event_id = $2 AND deleted_at IS NULL",
    )
    .bind(group)
    .bind(event)
    .fetch_optional(connection)
    .await?;
    source
        .map(|source| source.0)
        .ok_or_else(|| ApiError::not_found("group not found in event"))
}

/// @route POST /api/v1/events/:id/groups
pub async fn create_event_group(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
    input: Result<Json<EventGroupCreation>, JsonRejection>,
) -> Result<(StatusCode, Json<AccessChangeOutcome>), ApiError> {
    let creation = body(input)?;
    validate_group(&creation.name, &creation.source)?;
    let event = Uuid::parse_str(&id).map_err(|_| ApiError::bad_request("invalid id"))?;
    let mut tx = state.pool.begin().await?;
    let scope = lock_event_scope(&mut tx, event, &admin.0, &state.cfg).await?;
    advance_access_revision(&mut tx, event, creation.expected_access_revision).await?;
    let group: Uuid = sqlx::query_scalar(
        "INSERT INTO event_groups (event_id, name, source, created_by) VALUES ($1, $2, $3, $4) RETURNING id",
    )
    .bind(event)
    .bind(&creation.name)
    .bind(SqlJson(&creation.source))
    .bind(&scope.actor()?.discord_id)
    .fetch_one(&mut *tx)
    .await?;
    if let EventGroupSource::PartnerGuild { guild_id, .. } = &creation.source {
        enroll_event_partner_guild(&mut tx, event, guild_id).await?;
    }
    let message = format!("Created event group {group} ({})", creation.name);
    let outcome =
        finish_access_change(&mut tx, &state, &scope, "event.group_created", &message).await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(outcome)))
}

/// @route PATCH /api/v1/events/:id/groups/:groupId
pub async fn update_event_group(
    State(state): State<AppState>,
    admin: AdminUser,
    Path((id, group_id)): Path<(String, String)>,
    input: Result<Json<EventGroupChange>, JsonRejection>,
) -> Result<Json<AccessChangeOutcome>, ApiError> {
    let change = body(input)?;
    let (event, group) = ids(&id, &group_id)?;
    let mut tx = state.pool.begin().await?;
    let scope = lock_event_scope(&mut tx, event, &admin.0, &state.cfg).await?;
    let current = group_source(&mut tx, event, group).await?;
    let name: String = sqlx::query_scalar("SELECT name FROM event_groups WHERE id = $1")
        .bind(group)
        .fetch_one(&mut *tx)
        .await?;
    let name = change.name.unwrap_or(name);
    let source = change.source.unwrap_or(current);
    validate_group(&name, &source)?;
    advance_access_revision(&mut tx, event, change.expected_access_revision).await?;
    sqlx::query(
        "UPDATE event_groups SET name = $2, source = $3, updated_at = clock_timestamp() WHERE id = $1",
    )
    .bind(group)
    .bind(&name)
    .bind(SqlJson(&source))
    .execute(&mut *tx)
    .await?;
    if let EventGroupSource::PartnerGuild { guild_id, .. } = &source {
        enroll_event_partner_guild(&mut tx, event, guild_id).await?;
    }
    let message = format!("Changed event group {group} ({name})");
    let outcome =
        finish_access_change(&mut tx, &state, &scope, "event.group_changed", &message).await?;
    tx.commit().await?;
    Ok(Json(outcome))
}

/// @route DELETE /api/v1/events/:id/groups/:groupId
pub async fn delete_event_group(
    State(state): State<AppState>,
    admin: AdminUser,
    Path((id, group_id)): Path<(String, String)>,
    query: Result<Query<AccessRevisionPrecondition>, QueryRejection>,
) -> Result<Json<AccessChangeOutcome>, ApiError> {
    let expected = precondition(query)?;
    let (event, group) = ids(&id, &group_id)?;
    let mut tx = state.pool.begin().await?;
    let scope = lock_event_scope(&mut tx, event, &admin.0, &state.cfg).await?;
    group_source(&mut tx, event, group).await?;
    if group_is_referenced(&EventAccessContext::load(&mut tx, event).await?, group) {
        return Err(ApiError::conflict(
            "an access policy still names this group; remove it from every policy first",
        ));
    }
    advance_access_revision(&mut tx, event, expected).await?;
    sqlx::query("UPDATE event_groups SET deleted_at = clock_timestamp() WHERE id = $1")
        .bind(group)
        .execute(&mut *tx)
        .await?;
    let message = format!("Removed event group {group}");
    let outcome =
        finish_access_change(&mut tx, &state, &scope, "event.group_removed", &message).await?;
    tx.commit().await?;
    Ok(Json(outcome))
}

/// @route PUT /api/v1/events/:id/groups/:groupId/members/:discordId
pub async fn add_event_group_member(
    State(state): State<AppState>,
    admin: AdminUser,
    Path((id, group_id, discord_id)): Path<(String, String, String)>,
    input: Result<Json<AccessRevisionPrecondition>, JsonRejection>,
) -> Result<Json<AccessChangeOutcome>, ApiError> {
    let expected = body(input)?.expected_access_revision;
    let (event, group) = ids(&id, &group_id)?;
    let mut tx = state.pool.begin().await?;
    let scope = lock_event_scope(&mut tx, event, &admin.0, &state.cfg).await?;
    if !matches!(
        group_source(&mut tx, event, group).await?,
        EventGroupSource::ManagedRoster {}
    ) {
        return Err(ApiError::conflict(
            "partner-guild groups follow verified Discord membership and have no roster",
        ));
    }
    let known: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM users WHERE discord_id = $1 AND deleted_at IS NULL)",
    )
    .bind(&discord_id)
    .fetch_one(&mut *tx)
    .await?;
    if !known {
        return Err(ApiError::bad_request("user not found"));
    }
    advance_access_revision(&mut tx, event, expected).await?;
    sqlx::query(
        "INSERT INTO event_group_roster (group_id, discord_id, added_by) VALUES ($1, $2, $3)
         ON CONFLICT (group_id, discord_id) DO UPDATE SET removed_at = NULL,
             added_by = EXCLUDED.added_by, system_origin = NULL, added_at = clock_timestamp()
         WHERE event_group_roster.removed_at IS NOT NULL",
    )
    .bind(group)
    .bind(&discord_id)
    .bind(&scope.actor()?.discord_id)
    .execute(&mut *tx)
    .await?;
    let message = format!("Added {discord_id} to the roster of group {group}");
    let outcome = finish_access_change(
        &mut tx,
        &state,
        &scope,
        "event.group_member_added",
        &message,
    )
    .await?;
    tx.commit().await?;
    Ok(Json(outcome))
}

/// @route DELETE /api/v1/events/:id/groups/:groupId/members/:discordId
pub async fn remove_event_group_member(
    State(state): State<AppState>,
    admin: AdminUser,
    Path((id, group_id, discord_id)): Path<(String, String, String)>,
    query: Result<Query<AccessRevisionPrecondition>, QueryRejection>,
) -> Result<Json<AccessChangeOutcome>, ApiError> {
    let expected = precondition(query)?;
    let (event, group) = ids(&id, &group_id)?;
    let mut tx = state.pool.begin().await?;
    let scope = lock_event_scope(&mut tx, event, &admin.0, &state.cfg).await?;
    group_source(&mut tx, event, group).await?;
    advance_access_revision(&mut tx, event, expected).await?;
    let removed = sqlx::query(
        "UPDATE event_group_roster SET removed_at = clock_timestamp()
         WHERE group_id = $1 AND discord_id = $2 AND removed_at IS NULL",
    )
    .bind(group)
    .bind(&discord_id)
    .execute(&mut *tx)
    .await?;
    if removed.rows_affected() == 0 {
        return Err(ApiError::not_found("account is not on this roster"));
    }
    let message = format!("Removed {discord_id} from the roster of group {group}");
    let outcome = finish_access_change(
        &mut tx,
        &state,
        &scope,
        "event.group_member_removed",
        &message,
    )
    .await?;
    tx.commit().await?;
    Ok(Json(outcome))
}
