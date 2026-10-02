//! Refresh-token retention.
//!
//! Deletes refresh-token rows more than 7 days past expiry. Revoked-but-unexpired rows are
//! kept — they are the reuse-detection tripwire that
//! [`crate::identity_and_access::handlers::session_tokens::refresh`] trips on.
//!
//! The interval task that calls this lives in
//! [`crate::background_workers::token_purge_worker`].

use chrono::{Duration, Utc};
use sqlx::PgPool;

/// Retention past `expires_at` before a row is hard-deleted.
const RETENTION_DAYS: i64 = 7;

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
