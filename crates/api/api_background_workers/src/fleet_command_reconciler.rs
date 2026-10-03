//! Expires unclaimed fleet commands, returns lapsed claims to the queue and marks commands whose
//! executor stopped reporting mid-effect as indeterminate.
//!
//! **Role:** reconciles the fleet command ledger every [`FLEET_COMMAND_RECONCILIATION_INTERVAL`].
//! **Position:** armed by [`crate::worker_set::spawn_all`]; each pass is
//! `api_server_infrastructure::services::fleet_commands::command_reconciliation::reconcile_fleet_commands`.
//! **Signals & state:** one Tokio task owning a pool clone.
//! **Invariants:** a missed tick is skipped, never replayed; a failed pass is logged and the next
//! one retries; a closed pool stops the task.

use std::time::Duration;

use sqlx::PgPool;
use tokio::task::JoinHandle;

use api_server_infrastructure::services::fleet_commands::command_reconciliation::reconcile_fleet_commands;

/// How often the ledger is reconciled; a missed tick is skipped.
pub const FLEET_COMMAND_RECONCILIATION_INTERVAL: Duration = Duration::from_secs(5);

/// Spawn the reconciler: one reconciliation pass every [`FLEET_COMMAND_RECONCILIATION_INTERVAL`]
/// until the pool closes. A failed pass is logged; the next tick retries.
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
