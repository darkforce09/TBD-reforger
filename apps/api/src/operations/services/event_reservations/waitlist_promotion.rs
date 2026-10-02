//! Deterministic waitlist promotion inside the caller's locked event scope.
//!
//! Candidates are the event's waiting registrations in `(queue_entered_at, id)` order. Each
//! promotion takes an actual seat and, for a new participant, a quota place in the same
//! transaction, with a system audit record and its publication entry.

use sqlx::PgConnection;
use uuid::Uuid;

use super::participant_allocations::ensure_allocation;
use super::reservation_planning::PlannedPromotion;
use super::reservation_scope::ReservationScope;
use super::scope_snapshot::ScopeSnapshot;
use crate::administration::services::required_audit::append_system_audit;
use crate::core::error_handling::api_error::ApiError;
use crate::operations::services::event_status_rules::can_register_status;

/// Promote every waiting participant a place has become available for. Registration that is
/// closed or locked by an administrator promotes nobody.
pub async fn promote_waiting_participants(
    connection: &mut PgConnection,
    scope: &ReservationScope,
    main_guild: &str,
) -> Result<Vec<PlannedPromotion>, ApiError> {
    promote_waiting_candidates(connection, scope, main_guild, None).await
}

/// Promote in queue order, considering only one attachment's waiting entries when `mission`
/// is given. Returns the promotions applied in this transaction.
pub async fn promote_waiting_candidates(
    connection: &mut PgConnection,
    scope: &ReservationScope,
    main_guild: &str,
    mission: Option<Uuid>,
) -> Result<Vec<PlannedPromotion>, ApiError> {
    if !can_register_status(scope.event.status) || scope.event.registration_locked {
        return Ok(Vec::new());
    }
    let snapshot = ScopeSnapshot::load(connection, scope, main_guild).await?;
    let mut plan = snapshot.plan()?;
    let candidates: Vec<_> = snapshot
        .waiting
        .iter()
        .filter(|candidate| mission.is_none_or(|mission| candidate.mission == mission))
        .cloned()
        .collect();
    let promotions = plan
        .plan_promotions(&snapshot.eligibility, &candidates)
        .map_err(ApiError::internal)?;
    for promotion in &promotions {
        apply_promotion(connection, scope, promotion).await?;
    }
    Ok(promotions)
}

async fn apply_promotion(
    connection: &mut PgConnection,
    scope: &ReservationScope,
    promotion: &PlannedPromotion,
) -> Result<(), ApiError> {
    let allocation = ensure_allocation(
        connection,
        scope.event.id,
        &promotion.account,
        promotion.new_allocation,
    )
    .await?;
    let seated = sqlx::query(
        "UPDATE orbat_slots SET assigned_to = $1, assigned_at = clock_timestamp()
         WHERE id = $2 AND event_mission_id = $3 AND assigned_to IS NULL",
    )
    .bind(&promotion.account)
    .bind(promotion.seat)
    .bind(promotion.mission)
    .execute(&mut *connection)
    .await?;
    let registered = sqlx::query(
        "UPDATE event_registrations SET reservation_state = 'registered', slot_id = $2,
             allocation_id = $3, withdrawn_at = NULL, release_reason = NULL
         WHERE id = $1 AND reservation_state = 'waitlisted'",
    )
    .bind(promotion.registration)
    .bind(promotion.seat)
    .bind(allocation)
    .execute(&mut *connection)
    .await?;
    if seated.rows_affected() != 1 || registered.rows_affected() != 1 {
        return Err(ApiError::internal(
            "planned promotion no longer matches the locked scope",
        ));
    }
    let quota = match promotion.new_allocation {
        Some(kind) => format!("a new {} place", kind.as_str()),
        None => "the participant's existing event place".to_owned(),
    };
    append_system_audit(
        connection,
        "event.waitlist_promoted",
        "event_registration",
        &promotion.registration.to_string(),
        &format!(
            "Allocated seat {} and {quota} to the earliest eligible waiting participant",
            promotion.seat
        ),
    )
    .await?;
    Ok(())
}
