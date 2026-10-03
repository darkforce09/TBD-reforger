//! Scheduled convergence of the stored `events.status` column.
//!
//! Every read derives an event's effective status from `now()` inside Postgres
//! ([`api_operations::services::event_status_rules`] documents the derivation), so this worker decides
//! nothing: it exists so the stored column — what an operator sees in `psql`, and what the
//! audit trail records — agrees with the derived answer. A slow, late, or entirely absent pass
//! cannot let anyone register for a started operation.
//!
//! Multiple API instances all sweep; `sweep_once` takes a Postgres advisory lock so only one
//! does the work at a time, and its writes are conditional on the state they leave, so a
//! double run updates zero rows and cannot double-audit.
//!
//! **Role:** converges the stored `events.status` column every [`LIFECYCLE_INTERVAL`].
//! **Position:** armed by [`crate::worker_set::spawn_all`]; each pass is
//! `api_operations::services::event_lifecycle_sweep::sweep_once`.
//! **Signals & state:** one Tokio task owning a pool clone.
//! **Invariants:** the sweep decides nothing a read relies on; a failed pass is logged and the next
//! one retries.

use std::time::Duration;

use sqlx::PgPool;
use tokio::task::JoinHandle;

use api_operations::services::event_lifecycle_sweep::sweep_once;

/// How often the convergence sweep runs. Tight enough that the calendar is never more than
/// a minute stale, cheap enough to be free — `events` is a community ops calendar
/// (hundreds of rows), and a late or skipped sweep changes no decision.
pub const LIFECYCLE_INTERVAL: Duration = Duration::from_secs(60);

/// Handle to the lifecycle sweeper; dropping it detaches the task, aborting it stops it.
pub type LifecycleHandle = JoinHandle<()>;

/// Spawn the lifecycle sweeper: an immediate pass, then every [`LIFECYCLE_INTERVAL`].
/// A failed pass is logged and the next tick retries rather than killing the task.
pub fn start_event_lifecycle(pool: PgPool) -> LifecycleHandle {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(LIFECYCLE_INTERVAL);
        loop {
            ticker.tick().await;
            match sweep_once(&pool).await {
                Ok((started, completed)) if !started.is_empty() || !completed.is_empty() => {
                    tracing::info!(
                        started = started.len(),
                        completed = completed.len(),
                        "event lifecycle sweep"
                    );
                }
                Ok(_) => {}
                Err(e) => tracing::error!(error = %e, "event lifecycle sweep failed"),
            }
        }
    })
}
