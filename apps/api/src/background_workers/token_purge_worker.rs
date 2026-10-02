//! Scheduled hard-delete of refresh-token rows long past expiry.
//!
//! One boot sweep, then a fixed cadence. A late or skipped sweep costs table size only: an
//! expired token is already refused at `/auth/refresh`, so this reclaims space and never
//! enforces anything.

use std::time::Duration;

use sqlx::PgPool;
use tokio::task::JoinHandle;

use crate::identity_and_access::services::refresh_token_purge::purge_expired_refresh_tokens;

/// Re-sweep cadence after the immediate boot sweep.
pub const PURGE_INTERVAL: Duration = Duration::from_secs(6 * 60 * 60);

/// Handle to the background sweeper task (aborted on drop at shutdown).
pub type PurgeHandle = JoinHandle<()>;

/// Spawn the sweeper: an immediate sweep, then every [`PURGE_INTERVAL`] until the runtime
/// stops. A failed sweep is logged and the next tick retries rather than killing the task.
pub fn start_refresh_token_purge(pool: PgPool) -> PurgeHandle {
    tokio::spawn(async move {
        sweep(&pool).await;
        let mut ticker = tokio::time::interval(PURGE_INTERVAL);
        // `interval` fires immediately on first `tick`; the boot sweep above already ran.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            sweep(&pool).await;
        }
    })
}

async fn sweep(pool: &PgPool) {
    match purge_expired_refresh_tokens(pool).await {
        Ok(n) => tracing::info!("refresh token purge: {n} rows"),
        Err(e) => tracing::error!(error = %e, "refresh token purge failed"),
    }
}
