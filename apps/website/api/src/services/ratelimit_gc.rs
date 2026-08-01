//! Garbage collection for the durable rate limiter's bucket table (T-578).
//!
//! `rate_limit_buckets` gains one row per `(scope, client IP)` that has ever reached a
//! strict-prefix route. Nothing removes them, so without this task the table grows for the life of
//! the deployment — small, but unbounded, and unbounded is the part that matters.
//!
//! # Why deleting a bucket is never an amnesty
//!
//! A bucket that has not been touched for [`RATE_LIMIT_BUCKET_TTL`] has, by definition, refilled
//! to capacity: the strict policy refills one token per second into a ten-token bucket, so the
//! worst case (a bucket left at zero) is full 10 seconds later, and the TTL is an hour. A full
//! bucket and an absent bucket are indistinguishable to [`crate::app::durable_ratelimit::PgRateLimiter::check`]
//! — the `INSERT … ON CONFLICT` path seeds a full bucket and spends from it, exactly as the update
//! path would. So this is pure reclamation and can never hand a throttled client its quota back
//! early. That property is what makes it safe to run on a timer with no coordination.
//!
//! Shaped after `db::start_leaderboard_refresh`: one immediate sweep, then every interval, failures
//! logged and retried on the next tick.

use std::time::Duration;

use sqlx::PgPool;
use tokio::task::JoinHandle;

use crate::app::durable_ratelimit::PgRateLimiter;

/// Drop buckets untouched for an hour. See the module header for why this cannot grant quota:
/// the strict bucket refills fully in ten seconds, so an hour-old row is a full row.
pub const RATE_LIMIT_BUCKET_TTL: Duration = Duration::from_secs(60 * 60);

/// How often to sweep. Matching the TTL keeps the table at most ~2 h of distinct client IPs on
/// the auth + ingest surface, which is a few rows on this deployment.
pub const RATE_LIMIT_PRUNE_INTERVAL: Duration = Duration::from_secs(60 * 60);

/// Spawn the bucket-table sweeper: one immediate prune, then every `interval`.
///
/// The limiter is constructed with the burst/refill it will never use — `prune` only touches
/// `updated_at` — so the numbers here are deliberately not the policy numbers and must not be read
/// as such.
pub fn start_rate_limit_prune(pool: PgPool, ttl: Duration, interval: Duration) -> JoinHandle<()> {
    tokio::spawn(async move {
        let limiter = PgRateLimiter::new(pool, 1, 1);
        prune_once(&limiter, ttl).await;
        let mut ticker = tokio::time::interval(interval);
        // `interval` fires immediately on first `tick`; the sweep above already happened.
        ticker.tick().await;
        loop {
            ticker.tick().await;
            prune_once(&limiter, ttl).await;
        }
    })
}

async fn prune_once(limiter: &PgRateLimiter, ttl: Duration) {
    match limiter.prune(ttl).await {
        Ok(0) => {}
        Ok(n) => tracing::debug!(removed = n, "rate-limit buckets pruned"),
        // Not fatal and not silent: the limiter still refuses correctly with a fat table, so a
        // failed sweep is a disk-space problem, not a security one. The next tick retries.
        Err(e) => tracing::warn!(error = %e, "rate-limit bucket prune failed"),
    }
}
