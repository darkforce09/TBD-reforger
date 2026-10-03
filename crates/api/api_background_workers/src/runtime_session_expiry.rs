//! Ends runtime sessions whose game runtime stopped reporting, marks their servers offline and
//! republishes those servers' live status.
//!
//! **Role:** ends the silent runtime sessions every [`RUNTIME_SESSION_EXPIRY_INTERVAL`] and
//! republishes their servers' status.
//! **Position:** armed by [`crate::worker_set::spawn_all`]; each pass is
//! `api_server_infrastructure::services::runtime_sessions::expire_silent_runtime_sessions`, then
//! `status_broadcast::publish_server_status_by_id` per server. Integration suites call
//! [`expire_runtime_sessions`] directly.
//! **Signals & state:** one Tokio task owning a state clone.
//! **Invariants:** every server whose session ended is republished in the same pass; a failed pass
//! is logged and the next one retries; a closed pool stops the task.

use std::time::Duration;

use tokio::task::JoinHandle;

use api_foundation::error_handling::api_error::ApiError;
use api_server_infrastructure::services::runtime_sessions::expire_silent_runtime_sessions;
use api_server_infrastructure::services::status_broadcast::publish_server_status_by_id;
use api_state::AppState;

/// How often silent runtime sessions are ended; a missed tick is skipped.
pub const RUNTIME_SESSION_EXPIRY_INTERVAL: Duration = Duration::from_secs(15);

/// Spawn the expiry: an [`expire_runtime_sessions`] pass every
/// [`RUNTIME_SESSION_EXPIRY_INTERVAL`] until the pool closes. A failed pass is logged; the next
/// tick retries.
pub fn start_runtime_session_expiry(state: AppState) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut schedule = tokio::time::interval(RUNTIME_SESSION_EXPIRY_INTERVAL);
        schedule.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            schedule.tick().await;
            if state.pool.is_closed() {
                break;
            }
            if let Err(error) = expire_runtime_sessions(&state).await {
                tracing::error!(%error, "runtime session expiry failed");
            }
        }
    })
}

/// One expiry pass. Returns how many sessions ended.
pub async fn expire_runtime_sessions(state: &AppState) -> crate::Result<usize> {
    Ok(expire_and_republish(state).await?)
}

/// The body of [`expire_runtime_sessions`], in the services' own error.
async fn expire_and_republish(state: &AppState) -> Result<usize, ApiError> {
    let servers = expire_silent_runtime_sessions(&state.pool).await?;
    for server in &servers {
        publish_server_status_by_id(&state.pool, &state.hub, *server).await?;
    }
    Ok(servers.len())
}
