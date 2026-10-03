//! Bounded REST requests; durable PostgreSQL leases prevent replica duplication.
//!
//! **Role:** re-reads members' Discord guild membership: enrols due accounts every 30 seconds and
//! starts one reconciliation request every 40 milliseconds.
//! **Position:** armed by [`crate::worker_set::spawn_all`]; the enrolment and each request are
//! `api_identity_and_access::services::discord_rest_reconciliation::{enroll_accounts,
//! reconcile_one}`.
//! **Signals & state:** one Tokio task owning a state clone and a `JoinSet` of at most 32 requests
//! in flight.
//! **Invariants:** at most 32 requests run at once; the Postgres leases the service takes keep two
//! API processes off the same account; a closed pool stops the task and aborts the requests in
//! flight.

use api_identity_and_access::services::discord_rest_reconciliation::{
    enroll_accounts, reconcile_one,
};
use api_state::AppState;
use std::time::Duration;
use tokio::task::{JoinHandle, JoinSet};

/// Spawn the reconciler: enrol the accounts due a membership check every 30 seconds, and start
/// one reconciliation request every 40 milliseconds while fewer than 32 are in flight, until the
/// pool closes. A failed enrolment or request is logged; the next tick retries.
pub fn start_membership_reconciliation(state: AppState) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut requests = JoinSet::new();
        let mut schedule = tokio::time::interval(Duration::from_millis(40));
        schedule.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut enrollment = tokio::time::interval(Duration::from_secs(30));
        loop {
            tokio::select! {
                _ = enrollment.tick() => {
                    if state.pool.is_closed() { break; }
                    if let Err(error) = enroll_accounts(&state.pool, &state.cfg.discord_guild_id).await {
                        tracing::error!(%error, "Discord membership enrollment failed");
                    }
                }
                _ = schedule.tick(), if requests.len() < 32 => {
                    if state.pool.is_closed() { break; }
                    let state = state.clone();
                    requests.spawn(async move { reconcile_one(&state).await });
                }
                result = requests.join_next(), if !requests.is_empty() => {
                    match result {
                        Some(Ok(Err(error))) => tracing::error!(%error, "Discord membership reconciliation failed"),
                        Some(Err(error)) => tracing::error!(%error, "Discord membership request task failed"),
                        _ => {}
                    }
                }
            }
        }
        requests.abort_all();
    })
}
