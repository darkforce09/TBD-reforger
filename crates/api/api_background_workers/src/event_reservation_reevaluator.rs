//! Drains durable event re-evaluation requests: membership and account changes, and pool
//! opening times. Each pass runs one event-first transaction; leases keep replicas apart.
//!
//! **Role:** drains the due event reservation re-evaluation requests every second.
//! **Position:** armed by [`crate::worker_set::spawn_all`]; each request is leased from
//! `api_member_activity::reevaluation_queue` and re-evaluated by
//! `api_operations::services::event_reservations::eligibility_reevaluation`. Integration suites
//! call [`drain_due_reevaluations`] directly.
//! **Signals & state:** one Tokio task owning a state clone; the requests and their leases live in
//! Postgres.
//! **Invariants:** each request is re-evaluated in one transaction that locks the event first; a
//! deleted event completes its request; a failed request is marked failed and stops the pass; a
//! closed pool stops the task.

use std::time::Duration;

use tokio::task::JoinHandle;

use api_foundation::error_handling::api_error::ApiError;
use api_member_activity::reevaluation_queue::{
    ReevaluationLease, claim_due_reevaluation, complete_reevaluation, fail_reevaluation,
    schedule_pool_openings,
};
use api_operations::services::event_reservations::eligibility_reevaluation::{
    ReevaluationCause, reevaluate_event_reservations,
};
use api_operations::services::event_reservations::reservation_scope::{
    AttachmentScope, ReservationScope,
};
use api_state::AppState;

/// How often the reevaluator looks for due requests; a missed tick is skipped.
pub const REEVALUATION_POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Spawn the reevaluator: a [`drain_due_reevaluations`] pass every
/// [`REEVALUATION_POLL_INTERVAL`] until the pool closes. A failed pass is logged; the next tick
/// retries.
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
                tracing::error!(%error, "event reservation re-evaluation failed");
            }
        }
    })
}

/// Process every request that is due now. Returns how many passes completed; a failed request is
/// marked failed and ends the drain with its error.
pub async fn drain_due_reevaluations(state: &AppState) -> crate::Result<usize> {
    Ok(drain_leased_reevaluations(state).await?)
}

/// The body of [`drain_due_reevaluations`], in the services' own error.
async fn drain_leased_reevaluations(state: &AppState) -> Result<usize, ApiError> {
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
