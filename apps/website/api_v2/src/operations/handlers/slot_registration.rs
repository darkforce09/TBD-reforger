//! Self-service reservation changes preserve attendance and follow the event-scope lock order.
//! Capacity includes unresolved historical allocations until an explicit release resolves them.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::response::Json;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::administration::services::required_audit::append_actor_audit;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AuthUser;
use crate::identity_and_access::services::discord_membership_enrollment::request_event_membership_verification;
use crate::operations::models::RegistrationState;
use crate::operations::models::reservation_response::ReservationResponse;
use crate::operations::services::event_lookup::load_em;
use crate::operations::services::event_reservations::{
    claim_refusals::awaits_membership_verification,
    mutation_authority::require_registration_open,
    reservation_release::{release_reasons, release_registration, release_unused_allocations},
    reservation_scope::{AttachmentScope, ReservationScope},
    scope_snapshot::ScopeSnapshot,
    seat_claims::{ClaimRequest, apply_claim, decide_claim, release_other_seats},
    waitlist_promotion::promote_waiting_participants,
};

#[derive(Debug, Deserialize)]
pub struct RegisterBody {
    slot_id: String,
}

/// @route POST /api/v1/event-missions/:emid/register
pub async fn register_for_event_mission(
    State(state): State<AppState>,
    user: AuthUser,
    Path(emid): Path<String>,
    body: Result<Json<RegisterBody>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let Json(body) = body.map_err(|_| {
        ApiError::bad_request("slot_id is required (send \"\" to register without a seat)")
    })?;
    let want: Option<Uuid> = if body.slot_id.is_empty() {
        None
    } else {
        Some(Uuid::parse_str(&body.slot_id).map_err(|_| ApiError::not_found("slot not found"))?)
    };

    let mut tx = state.pool.begin().await?;
    let scope = ReservationScope::lock(
        &mut tx,
        em.event_id,
        AttachmentScope::Active,
        Some((&user, &state.cfg)),
        &[],
    )
    .await?;
    scope.require_active_mission(em.id)?;
    let actor = scope.actor()?;
    let is_admin = actor.role == "admin";
    require_registration_open(&scope.event, is_admin)?;
    let me = actor.discord_id.clone();
    let previous: Option<(RegistrationState, Option<Uuid>, bool)> = sqlx::query_as(
        "SELECT reservation_state, slot_id, withdrawn_at IS NULL AND release_reason IS NULL
         FROM event_registrations WHERE event_mission_id = $1 AND discord_id = $2",
    )
    .bind(em.id)
    .bind(&me)
    .fetch_optional(&mut *tx)
    .await?;
    let snapshot = ScopeSnapshot::load(&mut tx, &scope, &state.cfg.discord_guild_id).await?;
    let request = ClaimRequest {
        mission: em.id,
        account: &me,
        seat: want,
        bypass_squad_hold: is_admin,
    };
    let decision = match decide_claim(&mut tx, &snapshot, &request).await {
        Ok(decision) => decision,
        Err(error) => {
            if awaits_membership_verification(&error) {
                // The refused transaction rolls back; enrollment commits on its own.
                drop(tx);
                request_event_membership_verification(
                    &state.pool,
                    em.event_id,
                    &me,
                    &state.cfg.discord_guild_id,
                )
                .await?;
            }
            return Err(error);
        }
    };
    // A claim never frees a place: the claimant stays a participant of the attachment.
    let (registration, released, repaired_occupancy) =
        apply_claim(&mut tx, scope.event.id, &request, decision).await?;
    if released > 0
        || repaired_occupancy
        || previous != Some((registration.reservation_state, registration.slot_id, true))
    {
        append_actor_audit(
            &mut tx,
            &me,
            "event.registration_changed",
            "event_mission",
            &em.id.to_string(),
            &format!(
                "Reservation {} with slot {:?}",
                registration.reservation_state.as_str(),
                registration.slot_id
            ),
        )
        .await?;
    }
    tx.commit().await?;
    let response = ReservationResponse {
        state: registration.state,
        reservation_state: registration.reservation_state,
        attendance_state: registration.attendance_state,
        slot_id: registration.slot_id,
    };
    Ok(Json(serde_json::to_value(response).map_err(|_| {
        ApiError::internal("reservation response serialization failed")
    })?))
}

/// @route DELETE /api/v1/event-missions/:emid/register
pub async fn withdraw_from_event_mission(
    State(state): State<AppState>,
    user: AuthUser,
    Path(emid): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let em = load_em(&state.pool, &emid).await?;
    let mut tx = state.pool.begin().await?;
    let scope = ReservationScope::lock(
        &mut tx,
        em.event_id,
        AttachmentScope::Active,
        Some((&user, &state.cfg)),
        &[],
    )
    .await?;
    scope.require_active_mission(em.id)?;
    let me = scope.actor()?.discord_id.clone();
    let registration: Option<(Uuid, RegistrationState)> = sqlx::query_as(
        "SELECT id, reservation_state FROM event_registrations WHERE event_mission_id = $1 AND discord_id = $2",
    )
    .bind(em.id)
    .bind(&me)
    .fetch_optional(&mut *tx)
    .await?;
    let released_seats = release_other_seats(&mut tx, em.id, &me, None).await?;
    let changed = match registration {
        Some((id, _)) => {
            release_registration(&mut tx, id, release_reasons::PARTICIPANT_WITHDREW).await?
        }
        None if released_seats == 0 => return Err(ApiError::not_found("not registered")),
        None => false,
    };
    let was_active = registration.is_some_and(|(_, reservation)| {
        matches!(
            reservation,
            RegistrationState::Registered | RegistrationState::LegacyUnknown
        )
    });
    if changed || released_seats > 0 {
        release_unused_allocations(
            &mut tx,
            scope.event.id,
            std::slice::from_ref(&me),
            release_reasons::PARTICIPANT_WITHDREW,
        )
        .await?;
        if was_active || released_seats > 0 {
            promote_waiting_participants(&mut tx, &scope, &state.cfg.discord_guild_id).await?;
        }
        let message = match registration {
            None => "Released orphaned seats",
            Some((_, RegistrationState::Withdrawn)) => {
                "Released remaining seats; retained original withdrawal reason"
            }
            Some(_) => "Participant withdrew reservation",
        };
        append_actor_audit(
            &mut tx,
            &me,
            "event.registration_withdrawn",
            "event_mission",
            &em.id.to_string(),
            message,
        )
        .await?;
    }
    tx.commit().await?;
    Ok(Json(json!({ "withdrawn": true })))
}
