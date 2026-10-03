-- The table the durable rate limiter needs.
--
-- `core::middleware::durable_ratelimit::PgRateLimiter` binds this shape, and declares it as
-- `const RATE_LIMIT_BUCKETS_DDL` in `src/core/middleware/durable_ratelimit.rs`, specifically so
-- the bytes the tests prove and the bytes this migration lands cannot drift.
--
-- The DDL below is that constant, verbatim.
-- `tests/durable_rate_limit.rs::migration_0021_is_the_ddl_constant_verbatim` reads both and
-- fails if they ever stop matching, so "the migration was edited but the limiter still binds the
-- old shape" is not a state this tree can reach.
--
-- Shape notes (from the constant's own doc): `tokens` is `double precision` because the bucket
-- refills continuously; `updated_at` carries the last spend, so the refill is `elapsed * rate`
-- with no background job. The index serves `PgRateLimiter::prune`, which is the only scan — see
-- `background_workers::ratelimit_cleanup_worker`, armed in `bin/api.rs` beside the leaderboard
-- refresher.
--
-- `IF NOT EXISTS` is kept (rather than dropped now that a migration owns the table) because the
-- observability integration suite creates the same table from the same constant against its own
-- database.

CREATE TABLE IF NOT EXISTS public.rate_limit_buckets (
    bucket_key  text PRIMARY KEY,
    tokens      double precision NOT NULL,
    updated_at  timestamptz      NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS rate_limit_buckets_updated_at_idx
    ON public.rate_limit_buckets (updated_at);
