//! Expires unclaimed fleet commands, returns lapsed claims to the queue and marks commands whose
//! executor stopped reporting mid-effect as indeterminate.

use std::time::Duration;

use sqlx::PgPool;
use tokio::task::JoinHandle;

use crate::server_infrastructure::services::fleet_commands::command_reconciliation::reconcile_fleet_commands;

pub const FLEET_COMMAND_RECONCILIATION_INTERVAL: Duration = Duration::from_secs(5);

pub fn start_fleet_command_reconciliation(pool: PgPool) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut schedule = tokio::time::interval(FLEET_COMMAND_RECONCILIATION_INTERVAL);
        schedule.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            schedule.tick().await;
            if pool.is_closed() {
                break;
            }
            if let Err(error) = reconcile_fleet_commands(&pool).await {
                tracing::error!(error = %error.message, "fleet command reconciliation failed");
            }
        }
    })
}
