//! Re-evaluate an event's active reservations inside its locked scope.
//!
//! Only confirmed ineligibility releases a reservation: a ban or deletion, or a policy that last
//! verified membership facts no longer satisfy. Stale or unavailable Discord data never evicts.
//! Releasing a reservation clears its ORBAT seat but never ends a live life in a running game.
//! Places that become free are offered to waiting participants in the same transaction.

use sqlx::PgConnection;

use super::participant_allocations::release_allocation_if_unused;
use super::reservation_planning::{PlannedPromotion, PlannedRelease, ReleaseCause};
use super::reservation_release::{release_reasons, release_registration};
use super::reservation_scope::ReservationScope;
use super::scope_snapshot::ScopeSnapshot;
use super::waitlist_promotion::promote_waiting_participants;
use crate::administration::services::required_audit::append_system_audit;
use crate::core::error_handling::api_error::ApiError;
use crate::operations::models::EventStatus;

/// What prompted the re-evaluation; it selects the recorded release reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReevaluationCause {
    /// Membership, role, roster or account availability changed outside the event.
    ParticipantEligibility,
    /// A manager changed this event's access policy, groups or quotas.
    ManagerAccessChange,
}

#[derive(Debug, Default)]
pub struct ReevaluationOutcome {
    pub releases: Vec<PlannedRelease>,
    pub promotions: Vec<PlannedPromotion>,
}

fn release_reason(release: &PlannedRelease, cause: ReevaluationCause) -> &'static str {
    match (release.cause, cause) {
        (ReleaseCause::AccountUnavailable, _) => release_reasons::ACCOUNT_UNAVAILABLE,
        (ReleaseCause::PolicyDenied, ReevaluationCause::ManagerAccessChange) => {
            release_reasons::ACCESS_POLICY_CHANGED
        }
        (ReleaseCause::PolicyDenied, ReevaluationCause::ParticipantEligibility) => {
            release_reasons::ELIGIBILITY_LOST
        }
    }
}

/// Completed and cancelled events keep their reservations as history.
pub async fn reevaluate_event_reservations(
    connection: &mut PgConnection,
    scope: &ReservationScope,
    main_guild: &str,
    cause: ReevaluationCause,
) -> Result<ReevaluationOutcome, ApiError> {
    if matches!(
        scope.event.status,
        EventStatus::Completed | EventStatus::Cancelled
    ) {
        return Ok(ReevaluationOutcome::default());
    }
    let snapshot = ScopeSnapshot::load(connection, scope, main_guild).await?;
    let mut plan = snapshot.plan()?;
    let releases = plan
        .plan_eligibility_releases(&snapshot.eligibility, &snapshot.reservations)
        .map_err(ApiError::internal)?;
    for release in &releases {
        let reason = release_reason(release, cause);
        release_registration(connection, release.registration, reason).await?;
        if let Some(seat) = release.seat {
            sqlx::query(
                "UPDATE orbat_slots SET assigned_to = NULL, assigned_at = NULL
                 WHERE id = $1 AND assigned_to = $2",
            )
            .bind(seat)
            .bind(&release.account)
            .execute(&mut *connection)
            .await?;
        }
        release_allocation_if_unused(connection, scope.event.id, &release.account, reason).await?;
        append_system_audit(
            connection,
            "event.reservation_released",
            "event_registration",
            &release.registration.to_string(),
            &format!("Released reservation because of {reason}; history and attendance retained"),
        )
        .await?;
    }
    let promotions = promote_waiting_participants(connection, scope, main_guild).await?;
    Ok(ReevaluationOutcome {
        releases,
        promotions,
    })
}
