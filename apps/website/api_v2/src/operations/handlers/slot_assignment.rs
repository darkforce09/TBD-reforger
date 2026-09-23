//! Squad managers change seats under the event-scope lock order with current authority,
//! eligibility of the assignee, quota, capacity and seatability of existing place holders.
use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Json, Response};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::administration::services::required_audit::append_actor_audit;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::LeaderUser;
use crate::identity_and_access::services::discord_membership_enrollment::request_event_membership_verification;
use crate::operations::models::event::OrbatReservation;
use crate::operations::services::event_lookup::load_em;
use crate::operations::services::event_reservations::{
    claim_refusals::awaits_membership_verification,
    mutation_authority::{
        load_slot, require_leader, require_registration_open, require_squad_management,
    },
    reservation_release::{clear_seat, release_reasons, release_unused_allocations},
    reservation_scope::{AttachmentScope, ReservationScope},
    scope_snapshot::ScopeSnapshot,
    seat_claims::{ClaimRequest, apply_claim, decide_claim},
    waitlist_promotion::promote_waiting_participants,
};

#[derive(Debug, Deserialize)]
pub struct AssignSlotInput {
    #[serde(default)]
    discord_id: String,
}

/// @route PUT /api/v1/event-missions/:emid/slots/:slotId/assign
pub async fn assign_slot(
    State(state): State<AppState>,
    leader: LeaderUser,
    Path((emid, slot_id_s)): Path<(String, String)>,
    body: Result<Json<AssignSlotInput>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let slot_id =
        Uuid::parse_str(&slot_id_s).map_err(|_| ApiError::bad_request("invalid slot id"))?;
    let Json(input) = body.map_err(|_| ApiError::bad_request("discord_id required"))?;
    if input.discord_id.is_empty() || input.discord_id.len() > 128 {
        return Err(ApiError::bad_request(
            "discord_id required (maximum 128 bytes)",
        ));
    }
    let mut tx = state.pool.begin().await?;
    let scope = ReservationScope::lock(
        &mut tx,
        em.event_id,
        AttachmentScope::Active,
        Some((&leader.0, &state.cfg)),
        std::slice::from_ref(&input.discord_id),
    )
    .await?;
    scope.require_active_mission(em.id)?;
    let actor = scope.actor()?.clone();
    let slot = load_slot(&mut tx, em.id, slot_id).await?;
    require_squad_management(&mut tx, &actor, em.id, &slot.squad).await?;
    let snapshot = ScopeSnapshot::load(&mut tx, &scope, &state.cfg.discord_guild_id).await?;
    if !snapshot
        .facts(&input.discord_id)
        .is_some_and(|facts| facts.available)
    {
        return Err(ApiError::forbidden("account is unavailable"));
    }
    require_registration_open(&scope.event, actor.role == "admin")?;
    let unchanged: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM event_registrations WHERE event_mission_id = $1 AND discord_id = $2
         AND slot_id = $3 AND reservation_state = 'registered' AND withdrawn_at IS NULL AND release_reason IS NULL)",
    )
    .bind(em.id)
    .bind(&input.discord_id)
    .bind(slot_id)
    .fetch_one(&mut *tx)
    .await?;
    if unchanged && slot.assigned_to.as_deref() == Some(&input.discord_id) {
        tx.commit().await?;
        return Ok(Json(json!({ "assigned_to": input.discord_id })));
    }
    let request = ClaimRequest {
        mission: em.id,
        account: &input.discord_id,
        seat: Some(slot_id),
        // Squad management authority was checked above; holds never block their manager.
        bypass_squad_hold: true,
    };
    let decision = match decide_claim(&mut tx, &snapshot, &request).await {
        Ok(decision) => decision,
        Err(error) => {
            if awaits_membership_verification(&error) {
                drop(tx);
                request_event_membership_verification(
                    &state.pool,
                    em.event_id,
                    &input.discord_id,
                    &state.cfg.discord_guild_id,
                )
                .await?;
            }
            return Err(error);
        }
    };
    // Moving the assignee between seats frees no place, so nobody is promoted here.
    apply_claim(&mut tx, scope.event.id, &request, decision).await?;
    append_actor_audit(
        &mut tx,
        &actor.discord_id,
        "event.slot_assigned",
        "orbat_slot",
        &slot_id.to_string(),
        &format!("Assigned account {} to slot {slot_id}", input.discord_id),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(json!({ "assigned_to": input.discord_id })))
}

/// Clearing a seat retains the participant's allocated reservation until explicit withdrawal.
/// @route DELETE /api/v1/event-missions/:emid/slots/:slotId/assign
pub async fn clear_slot(
    State(state): State<AppState>,
    leader: LeaderUser,
    Path((emid, slot_id_s)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let slot_id =
        Uuid::parse_str(&slot_id_s).map_err(|_| ApiError::bad_request("invalid slot id"))?;
    let mut tx = state.pool.begin().await?;
    let scope = ReservationScope::lock(
        &mut tx,
        em.event_id,
        AttachmentScope::Active,
        Some((&leader.0, &state.cfg)),
        &[],
    )
    .await?;
    scope.require_active_mission(em.id)?;
    let actor = scope.actor()?.clone();
    let slot = load_slot(&mut tx, em.id, slot_id).await?;
    require_squad_management(&mut tx, &actor, em.id, &slot.squad).await?;
    let (occupant, repaired) = clear_seat(&mut tx, em.id, slot_id).await?;
    if let Some(occupant) = &occupant {
        release_unused_allocations(
            &mut tx,
            scope.event.id,
            std::slice::from_ref(occupant),
            release_reasons::SEAT_CLEARED,
        )
        .await?;
        promote_waiting_participants(&mut tx, &scope, &state.cfg.discord_guild_id).await?;
    }
    if occupant.is_some() || repaired > 0 {
        append_actor_audit(
            &mut tx,
            &actor.discord_id,
            "event.slot_cleared",
            "orbat_slot",
            &slot_id.to_string(),
            "Cleared assigned seat; reservation allocation remains until withdrawal",
        )
        .await?;
    }
    tx.commit().await?;
    Ok(Json(json!({ "cleared": true })))
}

#[derive(Debug, Deserialize)]
pub struct SquadBody {
    #[serde(default)]
    squad: String,
}

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
    let mut tx = state.pool.begin().await?;
    let scope = ReservationScope::lock(
        &mut tx,
        em.event_id,
        AttachmentScope::Active,
        Some((&leader.0, &state.cfg)),
        &[],
    )
    .await?;
    scope.require_active_mission(em.id)?;
    let actor = scope.actor()?.clone();
    require_leader(&actor)?;
    require_registration_open(&scope.event, actor.role == "admin")?;
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM orbat_slots WHERE event_mission_id = $1 AND squad = $2)",
    )
    .bind(em.id)
    .bind(&input.squad)
    .fetch_one(&mut *tx)
    .await?;
    if !exists {
        return Err(ApiError::not_found("squad not found in this ORBAT"));
    }
    let existing: Option<OrbatReservation> = sqlx::query_as(
        "SELECT * FROM orbat_reservations WHERE event_mission_id = $1 AND squad = $2",
    )
    .bind(em.id)
    .bind(&input.squad)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(existing) = existing {
        if existing.reserved_by != actor.discord_id {
            return Err(ApiError::conflict("squad is already reserved"));
        }
        tx.commit().await?;
        return Ok((StatusCode::OK, Json(existing)).into_response());
    }
    let res: OrbatReservation = sqlx::query_as("INSERT INTO orbat_reservations (event_mission_id, squad, reserved_by) VALUES ($1, $2, $3) RETURNING *")
        .bind(em.id).bind(&input.squad).bind(&actor.discord_id).fetch_one(&mut *tx).await?;
    append_actor_audit(
        &mut tx,
        &actor.discord_id,
        "event.squad_reserved",
        "event_mission",
        &em.id.to_string(),
        &format!("Reserved squad {}", input.squad),
    )
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(res)).into_response())
}

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
    let mut tx = state.pool.begin().await?;
    let scope = ReservationScope::lock(
        &mut tx,
        em.event_id,
        AttachmentScope::Active,
        Some((&leader.0, &state.cfg)),
        &[],
    )
    .await?;
    scope.require_active_mission(em.id)?;
    let actor = scope.actor()?.clone();
    require_leader(&actor)?;
    let res: Option<OrbatReservation> = sqlx::query_as(
        "SELECT * FROM orbat_reservations WHERE event_mission_id = $1 AND squad = $2",
    )
    .bind(em.id)
    .bind(&input.squad)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(res) = res else {
        return Err(ApiError::not_found("squad is not reserved"));
    };
    if res.reserved_by != actor.discord_id && actor.role != "admin" {
        return Err(ApiError::forbidden(
            "only the reserver or an admin can release this squad",
        ));
    }
    sqlx::query("DELETE FROM orbat_reservations WHERE event_mission_id = $1 AND squad = $2")
        .bind(em.id)
        .bind(&input.squad)
        .execute(&mut *tx)
        .await?;
    append_actor_audit(
        &mut tx,
        &actor.discord_id,
        "event.squad_released",
        "event_mission",
        &em.id.to_string(),
        &format!("Released squad {}", input.squad),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(json!({ "released": true })))
}
