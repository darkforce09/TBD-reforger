//! Settles every server's deployment in flight: confirms it from the runtime session that
//! reported its artifact, or fails it when that session loaded something else, when its fleet
//! command ended without effect, or when its deadline passed.

use std::time::Duration;

use sqlx::PgPool;
use tokio::task::JoinHandle;

use crate::missions::services::mission_deployments::deployment_settlement::reconcile_mission_deployments;

pub const MISSION_DEPLOYMENT_RECONCILIATION_INTERVAL: Duration = Duration::from_secs(5);

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
