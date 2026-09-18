//! Durable, cross-process rate limiting on Postgres — the L2 tier behind the in-memory L1
//! limiters in `rate_limiting.rs`.
//!
//! # What the in-memory tier alone cannot do
//!
//! `rate_limiting.rs` holds `governor` keyed limiters in `AppState`. Those are in-memory
//! and single-instance: every restart hands an abuser a fresh full bucket, and two API processes
//! each enforce the limit separately, so N processes means N× the intended rate. The buckets here
//! live in the database, so they survive a restart and are shared by every process pointed at the
//! same one.
//!
//! # Why Postgres and not Redis
//!
//! This deployment runs exactly one datastore (`scripts/deploy/tbd-reforger.service` plus the
//! compose Postgres on 5434). A second one is a new process to run, monitor, back up and secure in
//! order to hold a few hundred float counters. [`PgRateLimiter::check`] is one statement, and
//! `ON CONFLICT DO UPDATE` takes the row lock, so refill-and-spend is atomic across processes
//! without an advisory lock or a transaction round trip.
//!
//! # The three pieces this module depends on
//!
//! * the table — `migrations/0021_rate_limit_buckets.sql`, which is [`RATE_LIMIT_BUCKETS_DDL`]
//!   verbatim (pinned by `tests/durable_rate_limit.rs::migration_0020_is_the_ddl_constant_verbatim`,
//!   so the bytes the tests prove and the bytes the migration lands cannot drift);
//! * the wiring — [`crate::core::middleware::RateLimitState`], mounted by
//!   [`crate::core::http_router::router`]. The L1 `IpLimiter`s stay in front, narrowed to the
//!   strict prefixes; `rate_limiting.rs`'s header is the policy and its justification;
//! * the `prune` tick — [`crate::background_workers::ratelimit_cleanup_worker::start_rate_limit_prune`],
//!   armed by [`crate::background_workers::spawn_all`]
//!   beside the leaderboard refresher.

use std::net::IpAddr;
use std::time::Duration;

use sqlx::PgPool;

/// The table the limiter needs, verbatim, so the migration that lands it and the test
/// that proves the limiter are the same bytes.
///
/// `tokens` is a float because the bucket refills continuously; `updated_at` carries
/// the last spend so refill is `elapsed * rate` with no background job. The index is
/// for [`PgRateLimiter::prune`], which is the only scan.
pub const RATE_LIMIT_BUCKETS_DDL: &str = "\
CREATE TABLE IF NOT EXISTS public.rate_limit_buckets (
    bucket_key  text PRIMARY KEY,
    tokens      double precision NOT NULL,
    updated_at  timestamptz      NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS rate_limit_buckets_updated_at_idx
    ON public.rate_limit_buckets (updated_at);";

/// Refill-and-spend in one statement.
///
/// `$1` key, `$2` burst (bucket capacity), `$3` refill tokens/second. The `WHERE` on
/// the `DO UPDATE` is what makes this a limiter rather than a counter: when the
/// refilled balance is under one token the update matches no row, `RETURNING` yields
/// nothing, and the caller sees a refusal. `ON CONFLICT DO UPDATE` holds the row lock
/// for the duration, so two processes cannot both spend the last token.
const SPEND_SQL: &str = "\
INSERT INTO public.rate_limit_buckets AS b (bucket_key, tokens, updated_at)
VALUES ($1, $2::float8 - 1, now())
ON CONFLICT (bucket_key) DO UPDATE
   SET tokens = LEAST($2::float8,
                      b.tokens + EXTRACT(EPOCH FROM (now() - b.updated_at)) * $3::float8) - 1,
       updated_at = now()
 WHERE LEAST($2::float8,
             b.tokens + EXTRACT(EPOCH FROM (now() - b.updated_at)) * $3::float8) >= 1
RETURNING b.tokens";

/// A token bucket whose state lives in Postgres: survives restart, shared by every
/// process pointed at the same database.
#[derive(Clone)]
pub struct PgRateLimiter {
    pool: PgPool,
    burst: f64,
    refill_per_second: f64,
}

impl PgRateLimiter {
    /// `refill_per_second` sustained rate, `burst` bucket capacity — same units as
    /// `IpLimiter::new`, so the existing 20/40 and 1/10 settings port unchanged.
    pub fn new(pool: PgPool, refill_per_second: u32, burst: u32) -> Self {
        Self {
            pool,
            burst: f64::from(burst.max(1)),
            refill_per_second: f64::from(refill_per_second),
        }
    }

    /// True when a token was available and has been spent.
    ///
    /// The error is deliberately **not** folded into `true`: a limiter that opens up
    /// when its store is unreachable is a limiter that cannot refuse, which is the
    /// same defect class as a health check that cannot fail. Callers decide, loudly.
    pub async fn check(&self, key: &str) -> Result<bool, sqlx::Error> {
        let row: Option<(f64,)> = sqlx::query_as(SPEND_SQL)
            .bind(key)
            .bind(self.burst)
            .bind(self.refill_per_second)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.is_some())
    }

    /// Seconds a refused client should wait for one token, from this limiter's own
    /// refill rate. Always at least 1 — `Retry-After: 0` is an invitation to retry
    /// immediately, which is the opposite of the instruction.
    pub fn retry_after_secs(&self) -> u64 {
        if self.refill_per_second <= 0.0 {
            return 1;
        }
        ((1.0 / self.refill_per_second).ceil() as u64).max(1)
    }

    /// Drop buckets untouched for `older_than`. A full bucket is indistinguishable
    /// from no bucket, so this is pure garbage collection — never a grant of quota.
    /// Returns the number of rows removed.
    pub async fn prune(&self, older_than: Duration) -> Result<u64, sqlx::Error> {
        let res = sqlx::query(
            "DELETE FROM public.rate_limit_buckets \
             WHERE updated_at < now() - make_interval(secs => $1)",
        )
        .bind(older_than.as_secs_f64())
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected())
    }
}

/// `scope|ip` — the scope keeps the strict and global buckets independent for one IP,
/// exactly as the two separate `IpLimiter`s do.
///
/// The IP is whatever `client_identity::client_ip` resolved: the connection peer, or the
/// client behind it when that peer is a configured `TRUSTED_PROXIES` entry. So on the deployed
/// stack these rows read `strict|<member's public address>` rather than `strict|127.0.0.1` for
/// the whole community. With no trusted proxy configured they are the peer.
pub fn bucket_key(scope: &str, ip: IpAddr) -> String {
    format!("{scope}|{ip}")
}
