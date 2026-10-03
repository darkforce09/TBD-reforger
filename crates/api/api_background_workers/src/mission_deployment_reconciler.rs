//! Settles every server's deployment in flight: confirms it from the runtime session that
//! reported its artifact, or fails it when that session loaded something else, when its fleet
//! command ended without effect, or when its deadline passed.
//!
//! **Role:** settles the mission deployments in flight every
//! [`MISSION_DEPLOYMENT_RECONCILIATION_INTERVAL`].
//! **Position:** armed by [`crate::worker_set::spawn_all`]; each pass is
//! `api_missions::services::mission_deployments::deployment_settlement::reconcile_mission_deployments`.
//! **Signals & state:** one Tokio task owning a pool clone.
//! **Invariants:** a missed tick is skipped, never replayed; a failed pass is logged and the next
//! one retries; a closed pool stops the task.

use std::time::Duration;

use sqlx::PgPool;
use tokio::task::JoinHandle;

use api_missions::services::mission_deployments::deployment_settlement::reconcile_mission_deployments;

/// How often the deployments in flight are settled; a missed tick is skipped.
pub const MISSION_DEPLOYMENT_RECONCILIATION_INTERVAL: Duration = Duration::from_secs(5);

/// Spawn the reconciler: one settlement pass every
/// [`MISSION_DEPLOYMENT_RECONCILIATION_INTERVAL`] until the pool closes. A failed pass is logged;
/// the next tick retries.
pub fn start_mission_deployment_reconciliation(pool: PgPool) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut schedule = tokio::time::interval(MISSION_DEPLOYMENT_RECONCILIATION_INTERVAL);
        schedule.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            schedule.tick().await;
            if pool.is_closed() {
                break;
            }
            if let Err(error) = reconcile_mission_deployments(&pool).await {
                tracing::error!(error = %error.message, "mission deployment reconciliation failed");
            }
        }
    })
}
