//! Bounded REST requests; durable PostgreSQL leases prevent replica duplication.

use crate::core::application_state::AppState;
use crate::identity_and_access::services::discord_rest_reconciliation::{
    enroll_accounts, reconcile_one,
};
use std::time::Duration;
use tokio::task::{JoinHandle, JoinSet};

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
