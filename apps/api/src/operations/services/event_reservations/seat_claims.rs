//! Self-service registration and manager assignment share one claim decision.
//!
//! A new participant needs current authority for the seat (or, without a seat, for some seat of
//! the attachment), a quota place, event and attachment capacity, and must leave every existing
//! seatless holder seatable. Existing reservations keep their allocation and are never charged a
//! second place. The caller holds the event scope and has checked registration status.

use sqlx::PgConnection;
use uuid::Uuid;

use super::claim_refusals::ClaimRefusal;
use super::participant_allocations::ensure_allocation;
use super::quota_selection::QuotaDecision;
use super::scope_snapshot::ScopeSnapshot;
use crate::core::error_handling::api_error::ApiError;
use crate::operations::models::RegistrationState;
use crate::operations::models::reservation_quota::ReservationQuotaKind;
use crate::operations::services::event_access::evaluation::{
    AccessDenial, MandatoryAccessConstraints, PolicySource, QuotaOpeningGate,
    evaluate_access_with_pending_evidence,
};

/// What a granted claim changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimDecision {
    /// Hold `seat`, or a seatless place when `None`. `new_allocation` names the consumed pool.
    Reserve {
        seat: Option<Uuid>,
        new_allocation: Option<ReservationQuotaKind>,
    },
    /// No place is available now; enter or keep the waiting queue.
    Wait,
}

pub struct ClaimRequest<'a> {
    pub mission: Uuid,
    pub account: &'a str,
    pub seat: Option<Uuid>,
    /// Administrators may seat inside another leader's squad hold.
    pub bypass_squad_hold: bool,
}

/// Evaluate a claim against the locked scope without changing anything.
pub async fn decide_claim(
    connection: &mut PgConnection,
    snapshot: &ScopeSnapshot,
    request: &ClaimRequest<'_>,
) -> Result<ClaimDecision, ApiError> {
    let account = request.account;
    let facts = snapshot
        .facts(account)
        .ok_or_else(|| ApiError::bad_request("user not found"))?;
    if !facts.available {
        return Err(ApiError::forbidden("account is unavailable"));
    }
    if snapshot.mission_slots(request.mission).next().is_none() {
        return Err(ClaimRefusal::NoSeats.into());
    }
    let plan = snapshot.plan()?;
    let existing = snapshot.reservation(request.mission, account);
    let current_seat = existing.and_then(|reservation| reservation.seat);
    // Any seat the claimant occupies in the attachment, recorded or orphaned, is released.
    let occupied_seat = snapshot
        .seats
        .iter()
        .find(|seat| seat.mission == request.mission && seat.occupant.as_deref() == Some(account))
        .map(|seat| seat.id);
    let seatless_holding = existing
        .filter(|reservation| reservation.seat.is_none())
        .map(|reservation| reservation.queued.registration);
    let is_participant = existing.is_some();
    // A participant already holding the event allocation is never charged a second place.
    let place = if plan.holds_allocation(account) {
        None
    } else {
        Some(
            plan.quota_decision(facts.current.tbd_member)
                .map_err(ApiError::internal)?,
        )
    };
    let new_allocation = match place {
        Some(QuotaDecision::Reserved(kind)) => Some(kind),
        _ => None,
    };
    let (quota_opening, capacity_available) = match place {
        None | Some(QuotaDecision::Reserved(_)) => (QuotaOpeningGate::Open, true),
        Some(QuotaDecision::NotYetOpen {
            quota_kind,
            opens_at,
        }) => (
            QuotaOpeningGate::NotYetOpen {
                quota_kind,
                opens_at,
            },
            true,
        ),
        Some(QuotaDecision::Exhausted) => (QuotaOpeningGate::Open, false),
    };
    let constraints = MandatoryAccessConstraints {
        quota_opening,
        capacity_available,
        ..MandatoryAccessConstraints::SATISFIED
    };
    let oracle = &snapshot.eligibility;

    if let Some(seat) = request.seat {
        let slot = snapshot
            .slot(seat)
            .filter(|slot| slot.event_mission_id == request.mission)
            .ok_or_else(|| ApiError::not_found("slot not found"))?;
        if slot
            .assigned_to
            .as_deref()
            .is_some_and(|occupant| occupant != account)
        {
            return Err(ClaimRefusal::SeatTaken.into());
        }
        if !request.bypass_squad_hold {
            let holder: Option<String> = sqlx::query_scalar(
                "SELECT reserved_by FROM orbat_reservations WHERE event_mission_id = $1 AND squad = $2",
            )
            .bind(request.mission)
            .bind(&slot.squad)
            .fetch_optional(&mut *connection)
            .await?;
            if holder.is_some_and(|holder| holder != account) {
                return Err(ClaimRefusal::SquadHeldByLeader.into());
            }
        }
        if current_seat == Some(seat) {
            return Ok(ClaimDecision::Reserve {
                seat: Some(seat),
                new_allocation: None,
            });
        }
        let (event_policy, squad_policy, slot_policy) = oracle.context.slot_policy_chain(slot);
        let effective = oracle.context.effective_slot_policy(slot).0;
        let decision = evaluate_access_with_pending_evidence(
            event_policy,
            squad_policy,
            slot_policy,
            &facts.current,
            facts.pending_evidence_admits(effective, &oracle.main_guild),
            constraints,
        )
        .map_err(ApiError::internal)?;
        if let Some(denial) = decision.denial {
            return Err(ClaimRefusal::Denied {
                denial,
                policy_source: decision.policy_source,
            }
            .into());
        }
        if !plan.mission_has_room(request.mission, account) {
            return Err(ClaimRefusal::MissionFull.into());
        }
        if !plan.holders_remain_seatable(
            oracle,
            request.mission,
            seatless_holding,
            None,
            Some(seat),
            occupied_seat.filter(|occupied| *occupied != seat),
        ) {
            return Err(ClaimRefusal::SeatNeededByHolder.into());
        }
        return Ok(ClaimDecision::Reserve {
            seat: Some(seat),
            new_allocation,
        });
    }

    if seatless_holding.is_some() {
        return Ok(ClaimDecision::Reserve {
            seat: None,
            new_allocation: None,
        });
    }
    // Without a seat the participant needs some seat of the attachment the policy admits.
    let admitted = snapshot
        .mission_slots(request.mission)
        .any(|slot| oracle.context.slot_admits(slot, &facts.current));
    if !admitted && !is_participant {
        let pending = snapshot.mission_slots(request.mission).any(|slot| {
            facts.pending_evidence_admits(
                oracle.context.effective_slot_policy(slot).0,
                &oracle.main_guild,
            )
        });
        let denial = if pending {
            AccessDenial::MembershipVerificationRequired
        } else {
            AccessDenial::Policy
        };
        return Err(ClaimRefusal::Denied {
            denial,
            policy_source: PolicySource::Event,
        }
        .into());
    }
    if let QuotaOpeningGate::NotYetOpen {
        quota_kind,
        opens_at,
    } = quota_opening
    {
        return Err(ClaimRefusal::Denied {
            denial: AccessDenial::QuotaNotOpen {
                quota_kind,
                opens_at,
            },
            policy_source: PolicySource::Event,
        }
        .into());
    }
    let joins_holders = plan.holders_remain_seatable(
        oracle,
        request.mission,
        None,
        Some(account),
        None,
        occupied_seat,
    );
    if !capacity_available || !plan.mission_has_room(request.mission, account) || !joins_holders {
        if is_participant {
            return Err(ClaimRefusal::SeatNeededByHolder.into());
        }
        return Ok(ClaimDecision::Wait);
    }
    Ok(ClaimDecision::Reserve {
        seat: None,
        new_allocation,
    })
}

/// The registration row after applying a claim, as the wire response reports it.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct AppliedRegistration {
    pub id: Uuid,
    pub state: RegistrationState,
    pub reservation_state: RegistrationState,
    pub attendance_state: Option<RegistrationState>,
    pub slot_id: Option<Uuid>,
}

/// Seats the account occupies in `mission` other than `keep` become free.
pub async fn release_other_seats(
    connection: &mut PgConnection,
    mission: Uuid,
    account: &str,
    keep: Option<Uuid>,
) -> Result<u64, ApiError> {
    Ok(sqlx::query(
        "UPDATE orbat_slots SET assigned_to = NULL, assigned_at = NULL
         WHERE event_mission_id = $1 AND assigned_to = $2 AND id IS DISTINCT FROM $3",
    )
    .bind(mission)
    .bind(account)
    .bind(keep)
    .execute(connection)
    .await?
    .rows_affected())
}

/// Apply a granted decision. Identical retries change no row and so record no history.
pub async fn apply_claim(
    connection: &mut PgConnection,
    event_id: Uuid,
    request: &ClaimRequest<'_>,
    decision: ClaimDecision,
) -> Result<(AppliedRegistration, u64, bool), ApiError> {
    let account = request.account;
    let (state, seat, allocation) = match decision {
        ClaimDecision::Reserve {
            seat,
            new_allocation,
        } => {
            let allocation =
                ensure_allocation(connection, event_id, account, new_allocation).await?;
            (RegistrationState::Registered, seat, Some(allocation))
        }
        ClaimDecision::Wait => (RegistrationState::Waitlisted, None, None),
    };
    let released = release_other_seats(connection, request.mission, account, seat).await?;
    let mut repaired_occupancy = false;
    if let Some(seat) = seat {
        let occupied: Option<Option<String>> =
            sqlx::query_scalar("SELECT assigned_to FROM orbat_slots WHERE id = $1")
                .bind(seat)
                .fetch_optional(&mut *connection)
                .await?;
        repaired_occupancy = occupied.flatten().as_deref() != Some(account);
        let updated = sqlx::query(
            "UPDATE orbat_slots SET assigned_to = $1, assigned_at = CASE
                 WHEN assigned_to IS DISTINCT FROM $1 THEN clock_timestamp() ELSE assigned_at END
             WHERE id = $2 AND event_mission_id = $3 AND (assigned_to IS NULL OR assigned_to = $1)",
        )
        .bind(account)
        .bind(seat)
        .bind(request.mission)
        .execute(&mut *connection)
        .await?;
        if updated.rows_affected() != 1 {
            return Err(ClaimRefusal::SeatTaken.into());
        }
    }
    let registration: AppliedRegistration = sqlx::query_as(
        "INSERT INTO event_registrations (event_mission_id, discord_id, slot_id, reservation_state, allocation_id)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (event_mission_id, discord_id) DO UPDATE SET slot_id = EXCLUDED.slot_id,
             reservation_state = EXCLUDED.reservation_state, allocation_id = EXCLUDED.allocation_id,
             withdrawn_at = NULL, release_reason = NULL, queue_entered_at = CASE
                 WHEN EXCLUDED.reservation_state = 'waitlisted' AND event_registrations.reservation_state <> 'waitlisted'
                 THEN clock_timestamp() ELSE event_registrations.queue_entered_at END
         RETURNING id, state, reservation_state, attendance_state, slot_id",
    )
    .bind(request.mission)
    .bind(account)
    .bind(seat)
    .bind(state)
    .bind(allocation)
    .fetch_one(&mut *connection)
    .await?;
    Ok((registration, released, repaired_occupancy))
}
