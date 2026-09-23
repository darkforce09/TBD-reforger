//! Drains durable event re-evaluation requests: membership and account changes, and pool
//! opening times. Each pass runs one event-first transaction; leases keep replicas apart.

use std::time::Duration;

use tokio::task::JoinHandle;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::operations::services::event_reservations::eligibility_reevaluation::{
    ReevaluationCause, reevaluate_event_reservations,
};
use crate::operations::services::event_reservations::reevaluation_queue::{
    ReevaluationLease, claim_due_reevaluation, complete_reevaluation, fail_reevaluation,
    schedule_pool_openings,
};
use crate::operations::services::event_reservations::reservation_scope::{
    AttachmentScope, ReservationScope,
};

pub const REEVALUATION_POLL_INTERVAL: Duration = Duration::from_secs(1);

pub fn start_event_reservation_reevaluation(state: AppState) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut schedule = tokio::time::interval(REEVALUATION_POLL_INTERVAL);
        schedule.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            schedule.tick().await;
            if state.pool.is_closed() {
                break;
            }
            if let Err(error) = drain_due_reevaluations(&state).await {
                tracing::error!(error = %error.message, "event reservation re-evaluation failed");
            }
        }
    })
}

/// Process every request that is due now. Returns how many passes completed.
pub async fn drain_due_reevaluations(state: &AppState) -> Result<usize, ApiError> {
    let mut completed = 0;
    while let Some(lease) = claim_due_reevaluation(&state.pool).await? {
        match reevaluate_leased_event(state, &lease).await {
            Ok(()) => completed += 1,
            Err(error) => {
                fail_reevaluation(&state.pool, &lease, &error.message).await?;
                return Err(error);
            }
        }
    }
    Ok(completed)
}

async fn reevaluate_leased_event(
    state: &AppState,
    lease: &ReevaluationLease,
) -> Result<(), ApiError> {
    let mut tx = state.pool.begin().await?;
    match ReservationScope::lock(&mut tx, lease.event_id, AttachmentScope::Active, None, &[]).await
    {
        Ok(scope) => {
            reevaluate_event_reservations(
                &mut tx,
                &scope,
                &state.cfg.discord_guild_id,
                ReevaluationCause::ParticipantEligibility,
            )
            .await?;
            complete_reevaluation(&mut tx, lease).await?;
            schedule_pool_openings(&mut tx, lease.event_id).await?;
        }
        // A deleted event has nothing left to re-evaluate.
        Err(error) if error.status == axum::http::StatusCode::NOT_FOUND => {
            complete_reevaluation(&mut tx, lease).await?;
        }
        Err(error) => return Err(error),
    }
    tx.commit().await?;
    Ok(())
}
