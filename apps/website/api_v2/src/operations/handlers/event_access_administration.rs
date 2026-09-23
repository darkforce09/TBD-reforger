//! Administrator routes for event, squad and slot access policies and reservation quotas.
//!
//! Every change locks the event scope, reauthorizes the administrator after the lock wait,
//! checks the access revision the form was prepared against, re-evaluates current reservations
//! (releasing only confirmed ineligibility, promoting waiting participants) and writes a
//! required audit record in the same transaction.

use axum::extract::rejection::{JsonRejection, QueryRejection};
use axum::extract::{Path, Query, State};
use axum::response::Json;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::administration::services::required_audit::append_actor_audit;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AdminUser;
use crate::operations::models::event_access_administration::{
    AccessChangeOutcome, AccessPolicyChange, AccessRevisionPrecondition, EventAccessAdministration,
    ParticipantAccessExplanation, ReservationQuotaChange,
};
use crate::operations::services::access_administration::participant_explanation::explain_participants;
use crate::operations::services::access_administration::persistence::{
    advance_access_revision, load_access_administration, store_event_policy,
    store_reservation_quotas, store_slot_policy, store_squad_policy, validate_policy,
};
use crate::operations::services::event_lookup::{load_em, load_event};
use crate::operations::services::event_reservations::eligibility_reevaluation::{
    ReevaluationCause, reevaluate_event_reservations,
};
use crate::operations::services::event_reservations::event_administration::{
    allocation_usage, lock_event_scope,
};
use crate::operations::services::event_reservations::reevaluation_queue::schedule_pool_openings;
use crate::operations::services::event_reservations::reservation_scope::ReservationScope;

fn event_id(raw: &str) -> Result<Uuid, ApiError> {
    Uuid::parse_str(raw).map_err(|_| ApiError::bad_request("invalid id"))
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

/// Active attachments read in a consistent snapshot, for read-only views.
async fn active_missions(
    connection: &mut PgConnection,
    event: Uuid,
) -> Result<Vec<Uuid>, ApiError> {
    Ok(sqlx::query_scalar(
        "SELECT id FROM event_missions WHERE event_id = $1 AND deleted_at IS NULL ORDER BY id",
    )
    .bind(event)
    .fetch_all(connection)
    .await?)
}

/// Re-evaluate, audit and return the refreshed view of a committed-in-transaction change.
pub(super) async fn finish_access_change(
    connection: &mut PgConnection,
    state: &AppState,
    scope: &ReservationScope,
    action: &str,
    message: &str,
) -> Result<AccessChangeOutcome, ApiError> {
    let outcome = reevaluate_event_reservations(
        connection,
        scope,
        &state.cfg.discord_guild_id,
        ReevaluationCause::ManagerAccessChange,
    )
    .await?;
    append_actor_audit(
        connection,
        &scope.actor()?.discord_id,
        action,
        "event",
        &scope.event.id.to_string(),
        &format!(
            "{message}; released {} and promoted {} reservation(s)",
            outcome.releases.len(),
            outcome.promotions.len()
        ),
    )
    .await?;
    let access =
        load_access_administration(connection, &scope.event, &scope.active_missions).await?;
    Ok(AccessChangeOutcome {
        access,
        released_registrations: outcome.releases.iter().map(|r| r.registration).collect(),
        promoted_registrations: outcome.promotions.iter().map(|p| p.registration).collect(),
    })
}

/// @route GET /api/v1/events/:id/access
pub async fn get_event_access(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<String>,
) -> Result<Json<EventAccessAdministration>, ApiError> {
    let event = load_event(&state.pool, &id).await?;
    let mut tx = state.pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await?;
    let missions = active_missions(&mut tx, event.id).await?;
    let view = load_access_administration(&mut tx, &event, &missions).await?;
    tx.commit().await?;
    Ok(Json(view))
}

/// @route GET /api/v1/events/:id/access/participants
pub async fn get_event_access_participants(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<String>,
) -> Result<Json<Vec<ParticipantAccessExplanation>>, ApiError> {
    let event = load_event(&state.pool, &id).await?;
    let mut tx = state.pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await?;
    let missions = active_missions(&mut tx, event.id).await?;
    let participants =
        explain_participants(&mut tx, event.id, &missions, &state.cfg.discord_guild_id).await?;
    tx.commit().await?;
    Ok(Json(participants))
}

/// @route PUT /api/v1/events/:id/access-policy
pub async fn put_event_access_policy(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
    input: Result<Json<AccessPolicyChange>, JsonRejection>,
) -> Result<Json<AccessChangeOutcome>, ApiError> {
    let change = body(input)?;
    let mut tx = state.pool.begin().await?;
    let scope = lock_event_scope(&mut tx, event_id(&id)?, &admin.0, &state.cfg).await?;
    validate_policy(
        &mut tx,
        scope.event.id,
        &change.policy,
        &state.cfg.discord_guild_id,
    )
    .await?;
    advance_access_revision(&mut tx, scope.event.id, change.expected_access_revision).await?;
    store_event_policy(&mut tx, scope.event.id, &change.policy).await?;
    let outcome = finish_access_change(
        &mut tx,
        &state,
        &scope,
        "event.access_policy_changed",
        "Changed the event access policy",
    )
    .await?;
    tx.commit().await?;
    Ok(Json(outcome))
}

/// Squad and slot policy routes address an attachment; its event supplies the lock scope.
async fn lock_attachment_scope(
    connection: &mut PgConnection,
    state: &AppState,
    admin: &AdminUser,
    emid: &str,
) -> Result<(ReservationScope, Uuid), ApiError> {
    let em = load_em(&state.pool, emid).await?;
    let scope = lock_event_scope(connection, em.event_id, &admin.0, &state.cfg).await?;
    scope.require_active_mission(em.id)?;
    Ok((scope, em.id))
}

/// @route PUT /api/v1/event-missions/:emid/squads/:faction/:squad/access-policy
pub async fn put_squad_access_policy(
    State(state): State<AppState>,
    admin: AdminUser,
    Path((emid, faction, squad)): Path<(String, String, String)>,
    input: Result<Json<AccessPolicyChange>, JsonRejection>,
) -> Result<Json<AccessChangeOutcome>, ApiError> {
    let change = body(input)?;
    let mut tx = state.pool.begin().await?;
    let (scope, mission) = lock_attachment_scope(&mut tx, &state, &admin, &emid).await?;
    validate_policy(
        &mut tx,
        scope.event.id,
        &change.policy,
        &state.cfg.discord_guild_id,
    )
    .await?;
    advance_access_revision(&mut tx, scope.event.id, change.expected_access_revision).await?;
    store_squad_policy(&mut tx, mission, &faction, &squad, Some(&change.policy)).await?;
    let message = format!("Set the access policy of squad {faction}/{squad} in mission {mission}");
    let outcome = finish_access_change(
        &mut tx,
        &state,
        &scope,
        "event.squad_access_policy_changed",
        &message,
    )
    .await?;
    tx.commit().await?;
    Ok(Json(outcome))
}

/// @route DELETE /api/v1/event-missions/:emid/squads/:faction/:squad/access-policy
pub async fn delete_squad_access_policy(
    State(state): State<AppState>,
    admin: AdminUser,
    Path((emid, faction, squad)): Path<(String, String, String)>,
    query: Result<Query<AccessRevisionPrecondition>, QueryRejection>,
) -> Result<Json<AccessChangeOutcome>, ApiError> {
    let expected = precondition(query)?;
    let mut tx = state.pool.begin().await?;
    let (scope, mission) = lock_attachment_scope(&mut tx, &state, &admin, &emid).await?;
    advance_access_revision(&mut tx, scope.event.id, expected).await?;
    store_squad_policy(&mut tx, mission, &faction, &squad, None).await?;
    let message = format!("Squad {faction}/{squad} in mission {mission} inherits access again");
    let outcome = finish_access_change(
        &mut tx,
        &state,
        &scope,
        "event.squad_access_policy_removed",
        &message,
    )
    .await?;
    tx.commit().await?;
    Ok(Json(outcome))
}

/// @route PUT /api/v1/event-missions/:emid/slots/:slotId/access-policy
pub async fn put_slot_access_policy(
    State(state): State<AppState>,
    admin: AdminUser,
    Path((emid, slot_id)): Path<(String, String)>,
    input: Result<Json<AccessPolicyChange>, JsonRejection>,
) -> Result<Json<AccessChangeOutcome>, ApiError> {
    let change = body(input)?;
    let slot = Uuid::parse_str(&slot_id).map_err(|_| ApiError::bad_request("invalid slot id"))?;
    let mut tx = state.pool.begin().await?;
    let (scope, mission) = lock_attachment_scope(&mut tx, &state, &admin, &emid).await?;
    validate_policy(
        &mut tx,
        scope.event.id,
        &change.policy,
        &state.cfg.discord_guild_id,
    )
    .await?;
    advance_access_revision(&mut tx, scope.event.id, change.expected_access_revision).await?;
    store_slot_policy(&mut tx, mission, slot, Some(&change.policy)).await?;
    let message = format!("Set the access policy of slot {slot}");
    let outcome = finish_access_change(
        &mut tx,
        &state,
        &scope,
        "event.slot_access_policy_changed",
        &message,
    )
    .await?;
    tx.commit().await?;
    Ok(Json(outcome))
}

/// @route DELETE /api/v1/event-missions/:emid/slots/:slotId/access-policy
pub async fn delete_slot_access_policy(
    State(state): State<AppState>,
    admin: AdminUser,
    Path((emid, slot_id)): Path<(String, String)>,
    query: Result<Query<AccessRevisionPrecondition>, QueryRejection>,
) -> Result<Json<AccessChangeOutcome>, ApiError> {
    let expected = precondition(query)?;
    let slot = Uuid::parse_str(&slot_id).map_err(|_| ApiError::bad_request("invalid slot id"))?;
    let mut tx = state.pool.begin().await?;
    let (scope, mission) = lock_attachment_scope(&mut tx, &state, &admin, &emid).await?;
    advance_access_revision(&mut tx, scope.event.id, expected).await?;
    store_slot_policy(&mut tx, mission, slot, None).await?;
    let message = format!("Slot {slot} inherits access again");
    let outcome = finish_access_change(
        &mut tx,
        &state,
        &scope,
        "event.slot_access_policy_removed",
        &message,
    )
    .await?;
    tx.commit().await?;
    Ok(Json(outcome))
}

/// @route PUT /api/v1/events/:id/reservation-quotas
pub async fn put_reservation_quotas(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
    input: Result<Json<ReservationQuotaChange>, JsonRejection>,
) -> Result<Json<AccessChangeOutcome>, ApiError> {
    let change = body(input)?;
    let mut tx = state.pool.begin().await?;
    let scope = lock_event_scope(&mut tx, event_id(&id)?, &admin.0, &state.cfg).await?;
    let usage = allocation_usage(&mut tx, scope.event.id, &scope.active_missions).await?;
    let max_slots = u32::try_from(scope.event.max_slots)
        .map_err(|_| ApiError::internal("stored event capacity is out of range"))?;
    usage
        .validate_limits(&change.reservation_quotas, max_slots)
        .map_err(ApiError::conflict)?;
    advance_access_revision(&mut tx, scope.event.id, change.expected_access_revision).await?;
    store_reservation_quotas(&mut tx, scope.event.id, &change.reservation_quotas).await?;
    schedule_pool_openings(&mut tx, scope.event.id).await?;
    let outcome = finish_access_change(
        &mut tx,
        &state,
        &scope,
        "event.reservation_quotas_changed",
        "Changed member, guest and open reservation pools",
    )
    .await?;
    tx.commit().await?;
    Ok(Json(outcome))
}
