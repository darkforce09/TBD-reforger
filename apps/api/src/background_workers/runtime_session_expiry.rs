//! Ends runtime sessions whose game runtime stopped reporting, marks their servers offline and
//! republishes those servers' live status.

use std::time::Duration;

use tokio::task::JoinHandle;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::server_infrastructure::services::runtime_sessions::expire_silent_runtime_sessions;
use crate::server_infrastructure::services::status_broadcast::publish_server_status_by_id;

pub const RUNTIME_SESSION_EXPIRY_INTERVAL: Duration = Duration::from_secs(15);

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
                tracing::error!(error = %error.message, "runtime session expiry failed");
            }
        }
    })
}

/// One expiry pass. Returns how many sessions ended.
pub async fn expire_runtime_sessions(state: &AppState) -> Result<usize, ApiError> {
    let servers = expire_silent_runtime_sessions(&state.pool).await?;
    for server in &servers {
        publish_server_status_by_id(&state.pool, &state.hub, *server).await?;
    }
    Ok(servers.len())
}
