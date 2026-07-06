//! Background refresh-token purge — Rust port of `services/token_purge.go`.
//!
//! Deletes refresh-token rows more than 7 days past expiry. Revoked-but-unexpired
//! rows are kept — they are the reuse-detection tripwire (see handlers::auth::refresh).

use chrono::{Duration, Utc};
use sqlx::PgPool;
use tokio::task::JoinHandle;

/// Retention past `expires_at` before a row is hard-deleted.
const RETENTION_DAYS: i64 = 7;
/// Re-sweep cadence after the immediate boot sweep.
const PURGE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(6 * 60 * 60);

/// Handle to the background sweeper task (dropped/aborted on shutdown).
pub type PurgeHandle = JoinHandle<()>;

/// Hard-delete refresh-token rows that expired more than the retention window ago;
/// returns the number removed.
pub async fn purge_expired_refresh_tokens(pool: &PgPool) -> sqlx::Result<u64> {
    let cutoff = Utc::now() - Duration::days(RETENTION_DAYS);
    let res = sqlx::query("DELETE FROM refresh_tokens WHERE expires_at < $1")
        .bind(cutoff)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// Spawn the sweeper: an immediate sweep, then every 6h until the runtime stops.
pub fn start_refresh_token_purge(pool: PgPool) -> PurgeHandle {
    tokio::spawn(async move {
        sweep(&pool).await;
        let mut ticker = tokio::time::interval(PURGE_INTERVAL);
        ticker.tick().await; // consume the immediate first tick (already swept)
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
