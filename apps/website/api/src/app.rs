//! HTTP application assembly — the router + global middleware chain. Shared by the
//! `api` binary and the test/differential harnesses so they exercise one router.
//!
//! # Observability (T-280)
//!
//! Before T-280 this crate had **no** metrics of any kind: zero prometheus /
//! opentelemetry / sentry dependencies, no `/metrics` route, and a `/healthz` that
//! pinged the database and nothing else. This module now carries three things:
//!
//! * [`metrics::Registry`] — a dependency-free Prometheus text-format registry, one
//!   instance per [`router`] call (so tests are hermetic and there is no process-global
//!   mutable state), scraped at `GET /metrics` behind the existing `X-Service-Token`.
//! * [`observe`] — the middleware that feeds it, mounted **outside** the panic-catcher
//!   and the rate limiter so a 500-from-panic and a 429-from-throttle are both counted.
//! * [`healthz`] — a multi-check report (database + migration state) that **can go
//!   red on either check independently**. A health probe that cannot fail is worse than
//!   none, so both checks are tested against a deliberately broken pool. **T-580** put that
//!   report behind `X-Service-Token` and left a public `{"status": …}` + 200/503 for the
//!   credential-less probers; see [`healthz`] for the full reasoning.
//!
//! [`durable_ratelimit`] is the second half of T-280. **T-578 wired it**: the DDL is now
//! `migrations/0021_rate_limit_buckets.sql`, the limiter is the L2 tier in
//! `middleware/ratelimit.rs` (mounted below by [`router`]), and the `prune` tick is
//! [`crate::services::start_rate_limit_prune`].

use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Router;
use axum::extract::{DefaultBodyLimit, MatchedPath, Request, State};
use axum::http::{StatusCode, header};
use axum::middleware::{Next, from_fn, from_fn_with_state};
use axum::response::{IntoResponse, Json, Response};
use axum::routing::{get, post};
use serde_json::json;
use sqlx::PgPool;
use tower_http::catch_panic::CatchPanicLayer;
use tower_http::services::{ServeDir, ServeFile};

use crate::config::Config;
use crate::state::AppState;
use crate::{handlers, middleware};

/// Prometheus text-format metrics — registry, instrumentation, exposition (T-280).
///
/// # Why there is no metrics crate in `Cargo.toml`
///
/// The obvious move is `metrics` + `metrics-exporter-prometheus`. It was rejected, and
/// the reasons are recorded so the trade can be re-made knowingly rather than re-argued:
///
/// 1. **Nothing in this family is in `Cargo.lock`** — not `metrics`, not `prometheus`,
///    not `opentelemetry`, not `sentry`. Adding one is not a version bump, it is a new
///    subtree (`metrics-util` → `crossbeam-*`, `sketches-ddsketch`, `hashbrown`) and a
///    lockfile edit. `Cargo.lock` is shared with `website-frontend`, which builds to
///    `wasm32-unknown-unknown`; the brief's "keep the wasm/frontend build unaffected" is
///    *provably* satisfied by changing neither the lockfile nor the dependency graph.
/// 2. **The surface actually needed is small and fully testable.** Counters, one
///    histogram family, a handful of gauges, and the 0.0.4 text exposition — ~200 lines,
///    every line covered by the tests at the bottom of this file, including the
///    cardinality cap that a third-party recorder would not give us either.
/// 3. **Cardinality is the real risk, not arithmetic.** It is handled here by
///    construction: the `route` label is axum's [`MatchedPath`] template (`/missions/{id}`,
///    never a UUID) plus [`Registry::MAX_SERIES`] as a hard backstop with its own
///    `tbd_metrics_series_dropped_total` counter.
///
/// If a later slice needs OTLP export or handler-level instrumentation from modules that
/// do not have the `Arc<Registry>`, that is the moment to take the dependency — and the
/// natural home for the handle is an `AppState` field, i.e. `src/state.rs`, which is not
/// this slice's file.
pub mod metrics {
    use std::collections::BTreeMap;
    use std::fmt::Write as _;
    use std::sync::RwLock;
    use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    /// Number of latency histogram buckets (see [`LATENCY_BUCKETS_S`]).
    pub const N_BUCKETS: usize = 12;

    /// Histogram bucket upper bounds in **seconds**, ascending. Rendered cumulatively
    /// (`le=`), as the exposition format requires, plus the implicit `+Inf` bucket.
    pub const LATENCY_BUCKETS_S: [f64; N_BUCKETS] = [
        0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
    ];

    /// `route` label for a request that matched no route template (404s, static
    /// fallbacks). A literal, so an unrouted scan cannot mint one series per URL.
    pub const UNMATCHED_ROUTE: &str = "<unmatched>";

    /// The status a throttled request carries, mirrored into `tbd_http_rate_limited_total`.
    const TOO_MANY_REQUESTS: u16 = 429;

    /// Prometheus text exposition content type (format version 0.0.4).
    pub const CONTENT_TYPE: &str = "text/plain; version=0.0.4; charset=utf-8";

    /// A cumulative-bucket latency histogram. Buckets are incremented for every bound the
    /// observation falls under, so rendering is a straight read with no prefix sum.
    struct Histogram {
        buckets: [AtomicU64; N_BUCKETS],
        /// Sum in microseconds — integer atomics, converted to seconds at render.
        sum_micros: AtomicU64,
        count: AtomicU64,
    }

    impl Histogram {
        fn new() -> Self {
            Self {
                buckets: std::array::from_fn(|_| AtomicU64::new(0)),
                sum_micros: AtomicU64::new(0),
                count: AtomicU64::new(0),
            }
        }

        fn observe(&self, secs: f64) {
            for (slot, upper) in self.buckets.iter().zip(LATENCY_BUCKETS_S) {
                if secs <= upper {
                    slot.fetch_add(1, Ordering::Relaxed);
                }
            }
            self.sum_micros
                .fetch_add((secs * 1_000_000.0) as u64, Ordering::Relaxed);
            self.count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Every label-keyed family, behind one lock. Values are atomics so the hot path only
    /// needs a **shared** read lock; the write lock is taken exactly once per new series.
    #[derive(Default)]
    struct Tables {
        /// `tbd_http_requests_total{method,route,status}`
        requests: BTreeMap<(String, String, u16), AtomicU64>,
        /// `tbd_http_request_duration_seconds{method,route}`
        latency: BTreeMap<(String, String), Histogram>,
        /// `tbd_http_rate_limited_total{route}`
        limited: BTreeMap<String, AtomicU64>,
    }

    /// Runtime values sampled at scrape time rather than accumulated (pool depth, a live
    /// database ping). Passed to [`Registry::render`] so the registry stays I/O-free.
    #[derive(Debug, Clone, Copy)]
    pub struct Scrape {
        /// 1 when a `SELECT 1` round-tripped within the probe budget, else 0.
        pub db_up: bool,
        /// How long that ping took. Meaningless when `db_up` is false.
        pub db_ping: Duration,
        /// `PgPool::size()` — connections currently owned by the pool.
        pub pool_connections: u32,
        /// `PgPool::num_idle()` — of those, how many are free.
        pub pool_idle: usize,
    }

    /// One registry per [`super::router`] call.
    ///
    /// Deliberately **not** a `static`: a process-global recorder makes every test that
    /// asserts a count depend on which other tests ran first, which is precisely the
    /// "green over something it never examined" shape this ticket exists to avoid. The
    /// cost is that only code holding the `Arc` can record — see the module header.
    pub struct Registry {
        start: Instant,
        start_unix_s: u64,
        tables: RwLock<Tables>,
        in_flight: AtomicI64,
        dropped: AtomicU64,
    }

    impl Default for Registry {
        fn default() -> Self {
            Self::new()
        }
    }

    impl Registry {
        /// Hard ceiling on distinct label-sets **per family**. `route` is already a
        /// bounded template set, so hitting this means something is minting labels;
        /// dropping the sample and counting the drop beats unbounded memory growth.
        pub const MAX_SERIES: usize = 1024;

        pub fn new() -> Self {
            Self {
                start: Instant::now(),
                start_unix_s: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or_default(),
                tables: RwLock::new(Tables::default()),
                in_flight: AtomicI64::new(0),
                dropped: AtomicU64::new(0),
            }
        }

        /// How long this router has been assembled.
        pub fn uptime(&self) -> Duration {
            self.start.elapsed()
        }

        /// Requests currently in the middleware stack.
        pub fn in_flight(&self) -> i64 {
            self.in_flight.load(Ordering::Relaxed)
        }

        /// Samples discarded because a family was already at [`Self::MAX_SERIES`].
        pub fn dropped_series(&self) -> u64 {
            self.dropped.load(Ordering::Relaxed)
        }

        /// Enter a request: bumps the in-flight gauge and returns a guard that decrements
        /// on drop. A guard rather than a matching call because the panic path unwinds
        /// through this middleware, and a leaked gauge climbs forever.
        pub fn enter(self: &std::sync::Arc<Self>) -> InFlightGuard {
            self.in_flight.fetch_add(1, Ordering::Relaxed);
            InFlightGuard(self.clone())
        }

        /// Record one completed request.
        pub fn record(&self, method: &str, route: &str, status: u16, elapsed: Duration) {
            let req_key = (method.to_owned(), route.to_owned(), status);
            let lat_key = (method.to_owned(), route.to_owned());
            self.ensure_series(&req_key, &lat_key, route, status);

            let t = self.read();
            if let Some(c) = t.requests.get(&req_key) {
                c.fetch_add(1, Ordering::Relaxed);
            }
            if let Some(h) = t.latency.get(&lat_key) {
                h.observe(elapsed.as_secs_f64());
            }
            if status == TOO_MANY_REQUESTS
                && let Some(l) = t.limited.get(route)
            {
                l.fetch_add(1, Ordering::Relaxed);
            }
        }

        /// Create any missing series, honouring [`Self::MAX_SERIES`]. Takes the write
        /// lock only when at least one family is genuinely missing its key.
        fn ensure_series(
            &self,
            req_key: &(String, String, u16),
            lat_key: &(String, String),
            route: &str,
            status: u16,
        ) {
            {
                let t = self.read();
                let limited_ok = status != TOO_MANY_REQUESTS || t.limited.contains_key(route);
                if t.requests.contains_key(req_key) && t.latency.contains_key(lat_key) && limited_ok
                {
                    return;
                }
            }
            let mut t = self.write();
            let mut dropped = 0u64;
            if !t.requests.contains_key(req_key) {
                if t.requests.len() < Self::MAX_SERIES {
                    t.requests.insert(req_key.clone(), AtomicU64::new(0));
                } else {
                    dropped += 1;
                }
            }
            if !t.latency.contains_key(lat_key) {
                if t.latency.len() < Self::MAX_SERIES {
                    t.latency.insert(lat_key.clone(), Histogram::new());
                } else {
                    dropped += 1;
                }
            }
            if status == TOO_MANY_REQUESTS && !t.limited.contains_key(route) {
                if t.limited.len() < Self::MAX_SERIES {
                    t.limited.insert(route.to_owned(), AtomicU64::new(0));
                } else {
                    dropped += 1;
                }
            }
            if dropped > 0 {
                self.dropped.fetch_add(dropped, Ordering::Relaxed);
            }
        }

        /// Current value of `tbd_http_requests_total` for one label-set (test/assertion
        /// helper — the exposition text is the contract, this is the cheap read).
        pub fn requests_total(&self, method: &str, route: &str, status: u16) -> u64 {
            let key = (method.to_owned(), route.to_owned(), status);
            self.read()
                .requests
                .get(&key)
                .map(|c| c.load(Ordering::Relaxed))
                .unwrap_or(0)
        }

        /// Current value of `tbd_http_rate_limited_total` for one route.
        pub fn rate_limited_total(&self, route: &str) -> u64 {
            self.read()
                .limited
                .get(route)
                .map(|c| c.load(Ordering::Relaxed))
                .unwrap_or(0)
        }

        /// Render the whole registry in Prometheus text exposition format 0.0.4.
        pub fn render(&self, s: &Scrape) -> String {
            let t = self.read();
            let mut out = String::with_capacity(4096);

            out.push_str("# HELP tbd_build_info Build metadata for the running API; always 1.\n");
            out.push_str("# TYPE tbd_build_info gauge\n");
            let _ = writeln!(
                out,
                "tbd_build_info{{version=\"{}\"}} 1",
                esc(env!("CARGO_PKG_VERSION"))
            );

            out.push_str("# HELP tbd_process_start_time_seconds Unix start time of this router.\n");
            out.push_str("# TYPE tbd_process_start_time_seconds gauge\n");
            let _ = writeln!(out, "tbd_process_start_time_seconds {}", self.start_unix_s);

            out.push_str("# HELP tbd_uptime_seconds Seconds since this router was assembled.\n");
            out.push_str("# TYPE tbd_uptime_seconds gauge\n");
            let _ = writeln!(out, "tbd_uptime_seconds {:.3}", self.uptime().as_secs_f64());

            out.push_str("# HELP tbd_http_requests_in_flight Requests currently being served.\n");
            out.push_str("# TYPE tbd_http_requests_in_flight gauge\n");
            let _ = writeln!(out, "tbd_http_requests_in_flight {}", self.in_flight());

            out.push_str("# HELP tbd_http_requests_total Completed HTTP requests.\n");
            out.push_str("# TYPE tbd_http_requests_total counter\n");
            for ((method, route, status), c) in &t.requests {
                let _ = writeln!(
                    out,
                    "tbd_http_requests_total{{method=\"{}\",route=\"{}\",status=\"{}\"}} {}",
                    esc(method),
                    esc(route),
                    status,
                    c.load(Ordering::Relaxed)
                );
            }

            out.push_str("# HELP tbd_http_request_duration_seconds Request latency.\n");
            out.push_str("# TYPE tbd_http_request_duration_seconds histogram\n");
            for ((method, route), h) in &t.latency {
                let (m, r) = (esc(method), esc(route));
                for (slot, upper) in h.buckets.iter().zip(LATENCY_BUCKETS_S) {
                    let _ = writeln!(
                        out,
                        "tbd_http_request_duration_seconds_bucket{{method=\"{m}\",route=\"{r}\",le=\"{upper}\"}} {}",
                        slot.load(Ordering::Relaxed)
                    );
                }
                let count = h.count.load(Ordering::Relaxed);
                let _ = writeln!(
                    out,
                    "tbd_http_request_duration_seconds_bucket{{method=\"{m}\",route=\"{r}\",le=\"+Inf\"}} {count}"
                );
                let _ = writeln!(
                    out,
                    "tbd_http_request_duration_seconds_sum{{method=\"{m}\",route=\"{r}\"}} {:.6}",
                    h.sum_micros.load(Ordering::Relaxed) as f64 / 1_000_000.0
                );
                let _ = writeln!(
                    out,
                    "tbd_http_request_duration_seconds_count{{method=\"{m}\",route=\"{r}\"}} {count}"
                );
            }

            out.push_str(
                "# HELP tbd_http_rate_limited_total Requests refused by the rate limiter (429).\n",
            );
            out.push_str("# TYPE tbd_http_rate_limited_total counter\n");
            for (route, c) in &t.limited {
                let _ = writeln!(
                    out,
                    "tbd_http_rate_limited_total{{route=\"{}\"}} {}",
                    esc(route),
                    c.load(Ordering::Relaxed)
                );
            }

            out.push_str("# HELP tbd_db_up 1 when the database answered SELECT 1 at scrape.\n");
            out.push_str("# TYPE tbd_db_up gauge\n");
            let _ = writeln!(out, "tbd_db_up {}", u8::from(s.db_up));

            out.push_str("# HELP tbd_db_ping_seconds Duration of the scrape-time SELECT 1.\n");
            out.push_str("# TYPE tbd_db_ping_seconds gauge\n");
            let _ = writeln!(out, "tbd_db_ping_seconds {:.6}", s.db_ping.as_secs_f64());

            out.push_str("# HELP tbd_db_pool_connections sqlx pool connections by state.\n");
            out.push_str("# TYPE tbd_db_pool_connections gauge\n");
            let idle = s.pool_idle as u64;
            let in_use = u64::from(s.pool_connections).saturating_sub(idle);
            let _ = writeln!(out, "tbd_db_pool_connections{{state=\"idle\"}} {idle}");
            let _ = writeln!(out, "tbd_db_pool_connections{{state=\"in_use\"}} {in_use}");

            out.push_str(
                "# HELP tbd_metrics_series_dropped_total Samples dropped at the cardinality cap.\n",
            );
            out.push_str("# TYPE tbd_metrics_series_dropped_total counter\n");
            let _ = writeln!(
                out,
                "tbd_metrics_series_dropped_total {}",
                self.dropped_series()
            );

            out
        }

        /// A poisoned metrics lock must not take the API down — the data is a counter, not
        /// an invariant. Recover the guard and carry on.
        fn read(&self) -> std::sync::RwLockReadGuard<'_, Tables> {
            self.tables.read().unwrap_or_else(|e| e.into_inner())
        }

        fn write(&self) -> std::sync::RwLockWriteGuard<'_, Tables> {
            self.tables.write().unwrap_or_else(|e| e.into_inner())
        }
    }

    /// Decrements `tbd_http_requests_in_flight` on drop — including on unwind.
    pub struct InFlightGuard(std::sync::Arc<Registry>);

    impl Drop for InFlightGuard {
        fn drop(&mut self) {
            self.0.in_flight.fetch_sub(1, Ordering::Relaxed);
        }
    }

    /// Escape a Prometheus label value (`\`, `"`, newline). Our labels are HTTP methods
    /// and route templates and contain none of these — this exists so that stays true by
    /// enforcement rather than by assumption.
    fn esc(v: &str) -> String {
        let mut out = String::with_capacity(v.len());
        for c in v.chars() {
            match c {
                '\\' => out.push_str("\\\\"),
                '"' => out.push_str("\\\""),
                '\n' => out.push_str("\\n"),
                _ => out.push(c),
            }
        }
        out
    }
}

/// Durable, cross-process rate limiting on Postgres (T-280; **wired at T-578**).
///
/// # The defect this addresses
///
/// `middleware/ratelimit.rs` is a `governor` keyed limiter held in `AppState`. It is
/// in-memory and single-instance: every restart hands an abuser a fresh full bucket, and
/// two API processes each enforce the limit separately, so N processes means N× the
/// intended rate. Both are stated in that file's own header as known caveats.
///
/// # Why this is Postgres and not Redis
///
/// Redis is the usual answer and it is the wrong one here: this deployment already runs
/// exactly one datastore (`scripts/deploy/tbd-reforger.service` + the compose Postgres on
/// 5434), and a second one is a new process to run, monitor, back up and secure in order
/// to hold a few hundred float counters. [`PgRateLimiter::check`] is one statement, and
/// `ON CONFLICT DO UPDATE` takes the row lock, so the refill-and-spend is atomic across
/// processes without an advisory lock or a transaction round trip.
///
/// # Wiring (T-578) — where each of the three pieces landed
///
/// T-280 shipped this module implemented and proven and deliberately **unwired**, because the
/// table belongs in `apps/website/api/migrations/`, which was a sibling slice's file that wave.
/// All three pieces it named now exist:
///
/// * the table — `migrations/0021_rate_limit_buckets.sql`, which is
///   [`RATE_LIMIT_BUCKETS_DDL`] verbatim (pinned by
///   `tests/t578_ratelimit.rs::migration_0020_is_the_ddl_constant_verbatim`, so the bytes the
///   tests prove and the bytes the migration lands still cannot drift);
/// * the wiring — [`middleware::RateLimitState`], mounted in [`router`]. T-280's sketch replaced
///   the two `IpLimiter`s outright; what shipped is its own parenthesised alternative, "keep them
///   as an L1 in front", narrowed to the strict prefixes. `middleware/ratelimit.rs`'s header is
///   the policy and its justification;
/// * the `prune` tick — [`crate::services::start_rate_limit_prune`], armed in `src/bin/api.rs`
///   beside the leaderboard refresher, exactly as this doc asked.
pub mod durable_ratelimit {
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
        /// refill rate (T-578). Always at least 1 — `Retry-After: 0` is an invitation to
        /// retry immediately, which is the opposite of the instruction.
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
    /// exactly as the two separate `IpLimiter`s do today.
    ///
    /// **T-625 — which IP.** Whatever `middleware::ratelimit::client_ip` resolved: the connection
    /// peer, or the client behind it when that peer is a configured `TRUSTED_PROXIES` entry. So on
    /// the deployed stack these rows now read `strict|<member's public address>` rather than
    /// `strict|127.0.0.1` for the whole community. With no trusted proxy configured they are the
    /// peer, unchanged.
    pub fn bucket_key(scope: &str, ip: IpAddr) -> String {
        format!("{scope}|{ip}")
    }
}

/// The `/api/v1` route tree. Auth tiers are enforced per-handler by the extractor
/// each takes (`AuthUser`, the role-gated newtypes, `ServiceAuth`). Grows per phase.
fn api_routes(dev: bool, version_limit: usize) -> Router<AppState> {
    let mut r = Router::new()
        .route("/auth/discord/login", get(handlers::oauth::discord_login))
        .route(
            "/auth/discord/callback",
            get(handlers::oauth::discord_callback),
        )
        .route("/auth/refresh", post(handlers::auth::refresh))
        .route("/auth/logout", post(handlers::auth::logout))
        .route(
            "/me",
            get(handlers::me::get_me).patch(handlers::me::update_me),
        )
        .route(
            "/me/link",
            post(handlers::me::create_link_code).delete(handlers::me::unlink),
        )
        .route("/me/link/status", get(handlers::me::link_status))
        .route(
            "/ingest/link-confirm",
            post(handlers::me::ingest_link_confirm),
        )
        // Content reads (member tier via each handler's AuthUser extractor).
        .route(
            "/announcements",
            get(handlers::announcements::list_announcements),
        )
        .route(
            "/announcements/{id}",
            get(handlers::announcements::get_announcement),
        )
        .route("/wiki", get(handlers::wiki::list_wiki))
        .route(
            "/wiki/{slug}",
            get(handlers::wiki::get_wiki_page).put(handlers::wiki::upsert_wiki_page),
        )
        .route(
            "/vehicle-database",
            get(handlers::wiki::list_vehicles).post(handlers::wiki::create_vehicle),
        )
        // T-271: admin writes (create / replace / delete / set-current). Auth tier is
        // per-handler via AdminUser — same pattern as /wiki/{slug} PUT and /vehicle-database POST.
        .route(
            "/modpacks",
            get(handlers::modpacks::list_modpacks).post(handlers::modpacks::create_modpack),
        )
        .route(
            "/modpacks/current",
            get(handlers::modpacks::get_current_modpack),
        )
        .route(
            "/modpacks/{id}",
            axum::routing::put(handlers::modpacks::replace_modpack)
                .delete(handlers::modpacks::delete_modpack),
        )
        .route(
            "/modpacks/{id}/set-current",
            post(handlers::modpacks::set_current_modpack),
        )
        // T-586: the T-235 admin write side, registered. The handlers, their validation, their
        // audit writes and their lifecycle tests all landed in T-235 — only these two entries were
        // missing, so `create_server` / `update_server` / `deactivate_server` each carried an
        // `@route` tag to a door that was not in the wall. Nothing caught it because GO-7 (the
        // `@route`-vs-router check) did not survive the Go→Rust rewrite;
        // `scripts/verify-route-tags.sh` is that check, restored, and it fails on this file.
        //
        // The tier is per-handler (`AdminUser`), so it travels with the handler rather than with
        // the registration. Admin and NOT `MissionMakerUser`: a `servers` row is infrastructure,
        // not mission content, and the neighbours split on exactly that line — `/factions` writes
        // are `MissionMakerUser` because a faction is authored content, while `/modpacks`,
        // `/wiki/{slug}`, `/vehicle-database` and `/admin/servers/{id}/rcon` are all `AdminUser`.
        // This row carries the `inet` + port that RCON dials and that the game-server ingest path
        // keys off, so it belongs with the second group.
        //
        // The writes live at `/servers`, not `/admin/servers`: that is what the `@route` tags
        // claim, and it is what the crate already does for every other admin write on a resource
        // the whole authenticated site can read. `/admin/*` is reserved for resources only an admin
        // may READ at all (users, audit-logs, leave-requests, rcon).
        .route(
            "/servers",
            get(handlers::servers::list_servers).post(handlers::servers::create_server),
        )
        .route(
            "/servers/{id}",
            axum::routing::patch(handlers::servers::update_server)
                .delete(handlers::servers::deactivate_server),
        )
        .route(
            "/servers/{id}/status",
            get(handlers::servers::get_server_status),
        )
        .route(
            "/servers/{id}/status/stream",
            get(handlers::leaderboards::stream_server_status),
        )
        .route("/registry", get(handlers::registry::list_registry))
        .route(
            "/registry/compat",
            get(handlers::registry::list_registry_compat),
        )
        .route(
            "/factions",
            get(handlers::factions::list_factions).post(handlers::factions::create_faction),
        )
        .route(
            "/factions/{id}",
            get(handlers::factions::get_faction)
                .put(handlers::factions::update_faction)
                .delete(handlers::factions::delete_faction),
        )
        .route("/dashboard", get(handlers::dashboard::get_dashboard))
        .route(
            "/leaderboards",
            get(handlers::leaderboards::get_leaderboards),
        )
        .route(
            "/users/{discordId}/stats",
            get(handlers::leaderboards::get_user_stats),
        )
        .route(
            "/me/deployments",
            get(handlers::deployments::get_my_deployments),
        )
        .route(
            "/me/leave-requests",
            get(handlers::deployments::list_my_leave).post(handlers::deployments::submit_leave),
        )
        // Admin: LOA review + audit console.
        .route(
            "/admin/leave-requests",
            get(handlers::deployments::list_all_leave),
        )
        .route(
            "/admin/leave-requests/{id}",
            axum::routing::patch(handlers::deployments::review_leave),
        )
        .route("/admin/audit-logs", get(handlers::audit::list_audit_logs))
        .route(
            "/admin/audit-logs/stream",
            get(handlers::audit::stream_audit_logs),
        )
        .route(
            "/admin/audit-logs/export.csv",
            get(handlers::audit::export_audit_logs_csv),
        )
        // Mission library + editor.
        .route(
            "/missions",
            get(handlers::missions::list_missions).post(handlers::missions::create_mission),
        )
        .route(
            "/missions/{id}",
            get(handlers::missions::get_mission)
                .patch(handlers::missions::update_mission)
                .delete(handlers::missions::delete_mission),
        )
        .route(
            "/missions/{id}/submit",
            post(handlers::missions::submit_mission),
        )
        .route(
            "/missions/{id}/versions",
            // The version POST carries the compiled editor payload (hundreds of MB) —
            // override the global 1 MB body cap for this route only (Go: per-route BodyLimit).
            post(handlers::missions::create_version).layer(DefaultBodyLimit::max(version_limit)),
        )
        .route(
            "/missions/{id}/versions/{vid}",
            get(handlers::missions::get_version),
        )
        // T-532 — re-point current_version_id at a prior mission_versions row (rollback tip).
        .route(
            "/missions/{id}/versions/{vid}/set-current",
            post(handlers::missions::set_current_version),
        )
        .route(
            "/missions/{id}/armory",
            get(handlers::missions::get_armory).put(handlers::missions::set_armory),
        )
        .route(
            "/missions/{id}/bookmark",
            post(handlers::missions::bookmark_mission).delete(handlers::missions::remove_bookmark),
        )
        .route(
            "/missions/{id}/export",
            get(handlers::missions::export_mission),
        )
        .route(
            "/missions/{id}/compiled",
            get(handlers::missions::get_compiled_mission),
        )
        // Events (campaign) + ORBAT + registration.
        .route(
            "/events",
            get(handlers::events::list_events).post(handlers::events::create_event),
        )
        .route(
            "/events/{id}",
            get(handlers::events::get_event)
                .patch(handlers::events::update_event)
                .delete(handlers::events::delete_event),
        )
        .route(
            "/events/{id}/missions",
            post(handlers::events::add_event_mission),
        )
        .route(
            "/events/{id}/missions/{emid}",
            axum::routing::delete(handlers::events::remove_event_mission),
        )
        .route(
            "/event-missions/{emid}/orbat",
            get(handlers::events::get_orbat),
        )
        .route(
            "/event-missions/{emid}/register",
            post(handlers::events::register_for_event_mission)
                .delete(handlers::events::withdraw_from_event_mission),
        )
        .route(
            "/event-missions/{emid}/slots/{slotId}/assign",
            axum::routing::put(handlers::events::assign_slot).delete(handlers::events::clear_slot),
        )
        .route(
            "/event-missions/{emid}/squads/reserve",
            post(handlers::events::reserve_squad),
        )
        .route(
            "/event-missions/{emid}/squads/release",
            post(handlers::events::release_squad),
        )
        .route("/members", get(handlers::events::search_members))
        // Game-server telemetry ingest (service-token).
        .route(
            "/ingest/server-status",
            post(handlers::telemetry::ingest_server_status),
        )
        .route(
            "/ingest/match-results",
            post(handlers::telemetry::ingest_match_results),
        )
        // Game-server reads (service-token). Deliberately NOT the member-tier `/missions`
        // + `/event-missions/{emid}/orbat` handlers: both are scoped to the CALLING USER
        // (owner/bookmark filters, the caller's own registration state) and a service
        // token has no "me" — see the handler docs (T-181.51).
        .route(
            "/ingest/missions",
            get(handlers::missions::ingest_list_missions),
        )
        .route(
            "/ingest/events/{id}/roster",
            get(handlers::events::ingest_event_roster),
        )
        // Admin — personnel + server control.
        .route("/admin/users", get(handlers::admin::list_users))
        .route(
            "/admin/users/{discordId}",
            axum::routing::patch(handlers::admin::update_user),
        )
        .route(
            "/admin/users/{discordId}/ban",
            post(handlers::admin::ban_user).delete(handlers::admin::unban_user),
        )
        .route(
            "/admin/users/{discordId}/warnings",
            post(handlers::admin::issue_warning),
        )
        .route("/admin/roles/sync", post(handlers::admin::resync_roles))
        .route("/admin/servers/{id}/rcon", post(handlers::admin::send_rcon))
        // Approvals.
        .route("/approvals", get(handlers::approvals::list_approvals))
        .route(
            "/approvals/{id}/approve",
            post(handlers::approvals::approve_mission),
        )
        .route(
            "/approvals/{id}/reject",
            post(handlers::approvals::reject_mission),
        )
        // Field tools — mortar + inject.
        .route(
            "/fire-missions/solve",
            post(handlers::field_tools::solve_fire),
        )
        .route("/fire-missions", post(handlers::field_tools::save_fire))
        .route(
            "/events/{id}/fire-missions",
            get(handlers::field_tools::list_event_fire_missions),
        )
        .route(
            "/missions/{id}/inject",
            post(handlers::field_tools::inject_mission),
        )
        // CMS — announcements + uploads.
        .route(
            "/cms/announcements",
            get(handlers::cms::list_cms_announcements).post(handlers::cms::create_announcement),
        )
        .route(
            "/cms/announcements/{id}",
            axum::routing::patch(handlers::cms::update_announcement)
                .delete(handlers::cms::delete_announcement),
        )
        .route(
            "/cms/announcements/{id}/push-discord",
            post(handlers::cms::push_announcement_discord),
        )
        .route(
            "/cms/uploads",
            post(handlers::cms::upload_image)
                .layer(DefaultBodyLimit::max(middleware::MAX_MULTIPART_BODY)),
        );
    if dev {
        // Development-only login shortcut (also re-guards on env in-handler).
        r = r.route("/auth/dev-login", get(handlers::dev::dev_login));
    }
    r
}

/// Build the application: `/healthz`, `/metrics`, `/api/v1/*`, static `/uploads`, the optional
/// Leptos SPA + `/map-assets` (T-159.29), and the global middleware chain (outermost first:
/// request-id → logging → **metrics** → recovery → CORS → body-limit → rate-limit).
///
/// The metrics registry is created here, once per router, and shared by the `observe`
/// middleware, `/metrics` and `/healthz` — see [`metrics::Registry`] for why it is not a
/// `static`.
pub fn router(state: AppState) -> Router {
    let dev = state.cfg.is_development();
    let version_limit = state.cfg.mission_version_body_limit() as usize;
    let registry = Arc::new(metrics::Registry::new());

    // `/metrics` and `/healthz` need the registry, which is not in `AppState` (that is
    // `src/state.rs` — another slice's file), so both are closures over the `Arc`.
    let reg_metrics = registry.clone();
    let reg_health = registry.clone();
    let mut r = Router::new()
        // T-580 — public callers get `{"status": …}` and the 200/503 split, nothing else. The
        // detail is behind the same `X-Service-Token` that gates `/metrics`. See [`healthz`].
        .route(
            "/healthz",
            get(
                move |State(pool): State<PgPool>,
                      State(cfg): State<Arc<Config>>,
                      headers: axum::http::HeaderMap| {
                    let reg = reg_health.clone();
                    async move {
                        let detailed = service_token_matches(&cfg, &headers);
                        healthz(&reg, &pool, detailed).await
                    }
                },
            ),
        )
        // Scraping is gated on the SAME `X-Service-Token` the game-server ingest uses, and
        // `ServiceAuth` fails closed when `SERVICE_TOKEN` is unset — so an unconfigured
        // deployment answers 401, never a public dump of route templates and latencies.
        .route(
            "/metrics",
            get(
                move |_: middleware::ServiceAuth, State(pool): State<PgPool>| {
                    let reg = reg_metrics.clone();
                    async move { metrics_scrape(&reg, &pool).await }
                },
            ),
        )
        .nest("/api/v1", api_routes(dev, version_limit))
        .nest_service("/uploads", ServeDir::new("uploads"));

    // Always serve `/map-assets` (Trunk proxies here in dev; production SPA cutover uses the same
    // path). Gating this behind SPA_DIST_DIR left the editor with 404s for DEM/sat/world under
    // `make leptos` + `make api`.
    let map_assets = if state.cfg.map_assets_dir.is_empty() {
        "../../../packages/map-assets".to_string()
    } else {
        state.cfg.map_assets_dir.clone()
    };
    r = r.nest_service("/map-assets", ServeDir::new(map_assets));

    // T-159.29 — serve the Leptos SPA statically when SPA_DIST_DIR is set (the cutover flip; unset
    // in dev, where `trunk serve` owns the SPA). Cross-origin isolation (COOP `same-origin` + COEP
    // `credentialless`) mirrors the Vite/Trunk headers so the wasm SharedArrayBuffer path stays
    // available; a no-extension path falls back to index.html (client routing).
    if !state.cfg.spa_dist_dir.is_empty() {
        use axum::http::header::{HeaderName, HeaderValue};
        use tower_http::set_header::SetResponseHeaderLayer;

        let dist = state.cfg.spa_dist_dir.clone();
        let index = format!("{dist}/index.html");
        let coop = SetResponseHeaderLayer::overriding(
            HeaderName::from_static("cross-origin-opener-policy"),
            HeaderValue::from_static("same-origin"),
        );
        let coep = SetResponseHeaderLayer::overriding(
            HeaderName::from_static("cross-origin-embedder-policy"),
            HeaderValue::from_static("credentialless"),
        );
        r = r
            .fallback_service(ServeDir::new(dist).fallback(ServeFile::new(index)))
            .layer(coop)
            .layer(coep);
    }

    // T-578 — the rate limiter's own state: `AppState` (the in-memory L1 limiters) plus the
    // durable Postgres L2 built on the same pool. Not folded into `AppState` — see
    // `middleware::RateLimitState`.
    r.layer(from_fn_with_state(
        middleware::RateLimitState::new(state.clone()),
        middleware::rate_limit,
    ))
    .layer(DefaultBodyLimit::max(middleware::MAX_JSON_BODY))
    .layer(from_fn_with_state(state.clone(), middleware::cors))
    .layer(CatchPanicLayer::new())
    // OUTSIDE the panic-catcher and the rate limiter, INSIDE the logger. That position
    // is the whole point: inside `CatchPanicLayer` a panicking handler would never
    // reach `record` and the 500 would go uncounted, and inside `rate_limit` a throttled
    // request would never reach it either — `tbd_http_rate_limited_total` would be a
    // series that can only ever read 0. Both are checked by the tests below.
    .layer(from_fn_with_state(registry, observe))
    .layer(from_fn(middleware::logging))
    .layer(from_fn(middleware::request_id))
    .with_state(state)
}

/// Count every request: `tbd_http_requests_total`, the latency histogram, the in-flight
/// gauge, and `tbd_http_rate_limited_total` on a 429.
///
/// The `route` label is axum's [`MatchedPath`] — the registered template, so
/// `/api/v1/missions/{id}` is one series and not one per mission UUID. Requests that match
/// nothing collapse to [`metrics::UNMATCHED_ROUTE`].
async fn observe(State(reg): State<Arc<metrics::Registry>>, req: Request, next: Next) -> Response {
    let method = req.method().as_str().to_owned();
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map(|m| m.as_str().to_owned())
        .unwrap_or_else(|| metrics::UNMATCHED_ROUTE.to_owned());

    let _in_flight = reg.enter();
    let started = Instant::now();
    let resp = next.run(req).await;
    reg.record(&method, &route, resp.status().as_u16(), started.elapsed());
    resp
}

/// Budget for the health/scrape database probe. Long enough for a loaded server, short
/// enough that a wedged pool reports `down` instead of holding the probe open (the pool's
/// own acquire timeout is 30 s — a health check that inherits it is a hung health check).
const DB_PROBE_TIMEOUT: Duration = Duration::from_secs(2);

/// `SELECT 1`, bounded. Returns how long it took, or the failure text.
async fn probe_db(pool: &PgPool) -> (bool, Duration, Option<String>) {
    let started = Instant::now();
    match tokio::time::timeout(DB_PROBE_TIMEOUT, sqlx::query("SELECT 1").execute(pool)).await {
        Ok(Ok(_)) => (true, started.elapsed(), None),
        Ok(Err(e)) => (false, started.elapsed(), Some(e.to_string())),
        Err(_) => (
            false,
            started.elapsed(),
            Some(format!("timed out after {DB_PROBE_TIMEOUT:?}")),
        ),
    }
}

/// `GET /metrics` — Prometheus text exposition. Service-token gated at the route.
async fn metrics_scrape(reg: &metrics::Registry, pool: &PgPool) -> Response {
    let (db_up, db_ping, _) = probe_db(pool).await;
    let body = reg.render(&metrics::Scrape {
        db_up,
        db_ping,
        pool_connections: pool.size(),
        pool_idle: pool.num_idle(),
    });
    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, metrics::CONTENT_TYPE)],
        body,
    )
        .into_response()
}

/// Constant-time `X-Service-Token` comparison, without the 401 (T-580).
///
/// [`middleware::ServiceAuth`] is the extractor for routes that must *refuse* an unauthenticated
/// caller. `/healthz` must not: a load balancer or container orchestrator probes it with no
/// credentials and has to get a usable answer, so an absent or wrong token downgrades the payload
/// rather than rejecting the request. Same fail-closed rule as the extractor otherwise — an
/// unconfigured `SERVICE_TOKEN` matches nothing, so a deployment that never set one can never
/// serve the detail.
fn service_token_matches(cfg: &Config, headers: &axum::http::HeaderMap) -> bool {
    let got = headers
        .get("x-service-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    !cfg.service_token.is_empty() && crate::auth::constant_time_equal(got, &cfg.service_token)
}

/// Liveness/readiness probe.
///
/// Two independent checks, **either of which can take the whole probe red** — a health check
/// that cannot fail is worse than none:
///
/// * `database` — a bounded `SELECT 1`. Red when the database is down, refusing
///   connections, or slower than [`DB_PROBE_TIMEOUT`].
/// * `migrations` — red when `_sqlx_migrations` is unreadable (the schema was never
///   migrated, so the process is serving a database it does not match) or when any row
///   records a failed migration. This is the state `db::migrate` on boot is supposed to
///   guarantee and nothing was checking afterwards.
///
/// # T-580 — two payloads, one status (the reason `detailed` exists)
///
/// T-280 added those checks to a route that is **unauthenticated and published through Caddy**
/// (`scripts/deploy/Caddyfile.website:27`). Measured live against the pre-fix handler:
/// `version=0.1.0  uptime=396  pool={connections:5, idle:4}  migrations.applied=18` — the exact
/// build, how recently the process restarted, the connection-pool depth and the migration count,
/// to any caller who finds the URL. That is reconnaissance, handed over for free.
///
/// Simply adding auth was the wrong fix and is explicitly rejected: `/healthz` is probed **without
/// credentials** by `scripts/platform/preflight.sh:145`, `scripts/deploy/Caddyfile.website:27`,
/// `.github/workflows/editor-gates.yml:95` and `tools/tbd-tools/src/smokes.rs:2714`, and T-280
/// left it open for exactly that reason while gating `/metrics` behind `X-Service-Token`.
///
/// So the split is by **payload**, never by status code:
///
/// * **Public** (`detailed == false`) — `{"status": "ok" | "unavailable"}` and the 200/503 split.
///   That is everything a prober reads: `curl -fsS` only looks at the code, and `preflight.sh`
///   compares the code. Nothing about the build, the uptime, the pool or the schema is disclosed.
/// * **`X-Service-Token`** (`detailed == true`) — the full report: `version`, `uptime_seconds`,
///   per-check `status`/`latency_ms`/`error`, the applied/failed migration counts and the pool
///   gauges. Same fields, same names, same values as before T-580, so an operator's tooling is
///   unchanged; it just has to present the token `/metrics` already requires.
///
/// The top-level `status` string keeps its two legacy values in **both** shapes, and so does the
/// 200/503 split, because that pair is the actual contract every prober depends on.
async fn healthz(
    reg: &metrics::Registry,
    pool: &PgPool,
    detailed: bool,
) -> (StatusCode, Json<serde_json::Value>) {
    let (db_up, db_ping, db_err) = probe_db(pool).await;

    // Only meaningful if the database answered at all; skip the second round trip otherwise
    // and report the same cause rather than a confusing second timeout.
    let (mig_ok, mig_applied, mig_failed, mig_err) = if db_up {
        match tokio::time::timeout(
            DB_PROBE_TIMEOUT,
            sqlx::query_as::<_, (i64, i64)>(
                "SELECT count(*) FILTER (WHERE success), \
                        count(*) FILTER (WHERE NOT success) \
                 FROM _sqlx_migrations",
            )
            .fetch_one(pool),
        )
        .await
        {
            Ok(Ok((applied, failed))) => (failed == 0, applied, failed, None),
            Ok(Err(e)) => (false, 0, 0, Some(e.to_string())),
            Err(_) => (
                false,
                0,
                0,
                Some(format!("timed out after {DB_PROBE_TIMEOUT:?}")),
            ),
        }
    } else {
        (false, 0, 0, Some("database unavailable".to_owned()))
    };

    let healthy = db_up && mig_ok;
    let code = if healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    let status = if healthy { "ok" } else { "unavailable" };
    if !detailed {
        // The whole public payload. Every prober listed in this function's doc reads the code,
        // and `status` is here only because it is the one field that was already documented as
        // legacy contract. Nothing derived from the build, the process or the schema.
        return (code, Json(json!({ "status": status })));
    }
    (
        code,
        Json(json!({
            "status": status,
            "version": env!("CARGO_PKG_VERSION"),
            "uptime_seconds": reg.uptime().as_secs(),
            "checks": {
                "database": {
                    "status": if db_up { "up" } else { "down" },
                    "latency_ms": db_ping.as_millis() as u64,
                    "error": db_err,
                },
                "migrations": {
                    "status": if mig_ok { "up" } else { "down" },
                    "applied": mig_applied,
                    "failed": mig_failed,
                    "error": mig_err,
                },
            },
            "pool": {
                "connections": pool.size(),
                "idle": pool.num_idle(),
            },
        })),
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use axum::body::{Body, to_bytes};
    use axum::http::{Request as HttpRequest, StatusCode};
    use sqlx::postgres::PgPoolOptions;
    use tower::ServiceExt as _;

    use super::metrics::{Registry, Scrape, UNMATCHED_ROUTE};
    use super::*;
    use crate::config::Config;

    // ───────────────────────────── harness ─────────────────────────────

    /// A pool that never reaches a server and gives up fast.
    ///
    /// `db::connect_lazy` bakes in the production 30 s acquire timeout, which would make
    /// every "database is down" assertion below a 30 s wall — so these tests build their
    /// own. The port is in the ephemeral range and nothing listens on it.
    fn dead_pool() -> PgPool {
        PgPoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_millis(250))
            .connect_lazy("postgres://t280:t280@127.0.0.1:1/t280_no_such_db")
            .expect("lazy pool")
    }

    fn app_with(pool: PgPool) -> Router {
        let cfg = Config::for_tests("postgres://unused", "t280-test-secret");
        router(AppState::new(pool, cfg))
    }

    async fn call(
        app: &Router,
        method: &str,
        uri: &str,
        service_token: bool,
    ) -> (StatusCode, String) {
        let mut b = HttpRequest::builder().method(method).uri(uri);
        if service_token {
            b = b.header("x-service-token", "test-service-token");
        }
        let resp = app
            .clone()
            .oneshot(b.body(Body::empty()).expect("request"))
            .await
            .expect("router call");
        let status = resp.status();
        let body = to_bytes(resp.into_body(), 1 << 20).await.expect("body");
        (status, String::from_utf8_lossy(&body).into_owned())
    }

    /// One exposition line, or `None`. Matching the whole line (not `contains`) is
    /// deliberate: `contains("tbd_http_requests_total")` is true of the `# TYPE` comment,
    /// so a registry that recorded nothing at all would still pass a `contains` assertion.
    fn line<'a>(body: &'a str, prefix: &str) -> Option<&'a str> {
        body.lines().find(|l| l.starts_with(prefix))
    }

    fn value(body: &str, prefix: &str) -> Option<f64> {
        line(body, prefix)?.rsplit(' ').next()?.parse().ok()
    }

    // ───────────────────── metrics: series that move ─────────────────────

    /// The primary non-vacuity claim: `/metrics` does not merely answer 200 — every
    /// series it names **changes in response to real traffic**, and the same scrape run
    /// twice differs by exactly the traffic in between.
    #[tokio::test]
    async fn every_http_series_moves_with_real_traffic() {
        let app = app_with(dead_pool());

        // Scrape #1: the endpoint works, and the request families are empty of the route
        // we are about to drive. (`/metrics` itself is counted, so it is not zero-length.)
        let (st, first) = call(&app, "GET", "/metrics", true).await;
        assert_eq!(st, StatusCode::OK);
        assert!(
            line(
                &first,
                "tbd_http_requests_total{method=\"GET\",route=\"/healthz\""
            )
            .is_none(),
            "no /healthz traffic yet, but a /healthz series already exists:\n{first}"
        );
        assert_eq!(value(&first, "tbd_build_info").unwrap(), 1.0);

        // Real traffic: three health probes. The pool is dead, so these are 503s — a real
        // status, not a synthetic one.
        for _ in 0..3 {
            let (st, _) = call(&app, "GET", "/healthz", false).await;
            assert_eq!(st, StatusCode::SERVICE_UNAVAILABLE);
        }

        let (_, second) = call(&app, "GET", "/metrics", true).await;
        let key = "tbd_http_requests_total{method=\"GET\",route=\"/healthz\",status=\"503\"}";
        assert_eq!(
            value(&second, key),
            Some(3.0),
            "counter did not follow the three requests:\n{second}"
        );

        // Histogram: count tracks the counter, +Inf equals count, buckets are cumulative.
        let lat = "tbd_http_request_duration_seconds";
        let cnt = format!("{lat}_count{{method=\"GET\",route=\"/healthz\"}}");
        assert_eq!(value(&second, &cnt), Some(3.0), "histogram count\n{second}");
        let inf = format!("{lat}_bucket{{method=\"GET\",route=\"/healthz\",le=\"+Inf\"}}");
        assert_eq!(value(&second, &inf), Some(3.0), "+Inf bucket\n{second}");
        let mut prev = 0.0;
        for ub in super::metrics::LATENCY_BUCKETS_S {
            let p = format!("{lat}_bucket{{method=\"GET\",route=\"/healthz\",le=\"{ub}\"}}");
            let v = value(&second, &p).unwrap_or_else(|| panic!("missing bucket {ub}\n{second}"));
            assert!(v >= prev, "buckets not cumulative at le={ub}: {v} < {prev}");
            prev = v;
        }
        assert!(
            value(
                &second,
                &format!("{lat}_sum{{method=\"GET\",route=\"/healthz\"}}")
            )
            .unwrap()
                > 0.0,
            "latency sum stayed at zero — nothing was timed\n{second}"
        );

        // Gauges that reflect the world rather than a constant.
        assert_eq!(
            value(&second, "tbd_db_up"),
            Some(0.0),
            "dead pool must read db_up 0"
        );
        assert_eq!(
            value(&second, "tbd_http_requests_in_flight"),
            Some(1.0),
            "the scrape itself"
        );
        assert_eq!(
            value(&second, "tbd_metrics_series_dropped_total"),
            Some(0.0)
        );
        assert!(value(&second, "tbd_uptime_seconds").unwrap() >= 0.0);
    }

    /// Route labels come from the matched template, so a thousand mission UUIDs are one
    /// series and an unrouted scan is one more — this is the cardinality guarantee that
    /// makes `MAX_SERIES` a backstop rather than the primary defence.
    #[tokio::test]
    async fn route_label_is_the_template_not_the_uri() {
        let app = app_with(dead_pool());
        for i in 0..4 {
            let (st, _) = call(&app, "GET", &format!("/no/such/path/{i}"), false).await;
            assert_eq!(st, StatusCode::NOT_FOUND);
        }
        let (_, body) = call(&app, "GET", "/metrics", true).await;
        let key = format!(
            "tbd_http_requests_total{{method=\"GET\",route=\"{UNMATCHED_ROUTE}\",status=\"404\"}}"
        );
        assert_eq!(
            value(&body, &key),
            Some(4.0),
            "four 404s, one series:\n{body}"
        );
        assert!(
            !body.contains("/no/such/path/"),
            "a raw URI leaked into a label — unbounded cardinality:\n{body}"
        );
    }

    /// `tbd_http_rate_limited_total` is the series most at risk of being decorative: if
    /// the metrics layer sat inside the rate limiter it could only ever read 0. Drive the
    /// strict limiter past its burst and watch the series move.
    #[tokio::test]
    async fn throttled_requests_are_counted() {
        let app = app_with(dead_pool());
        let uri = "/api/v1/auth/discord/login"; // strict prefix, touches no database
        let mut refused = 0;
        for _ in 0..14 {
            let (st, _) = call(&app, "GET", uri, false).await;
            if st == StatusCode::TOO_MANY_REQUESTS {
                refused += 1;
            }
        }
        assert!(
            refused > 0,
            "strict limiter (burst 10) refused nothing in 14 requests"
        );

        let (_, body) = call(&app, "GET", "/metrics", true).await;
        let key = format!("tbd_http_rate_limited_total{{route=\"{uri}\"}}");
        assert_eq!(
            value(&body, &key),
            Some(f64::from(refused)),
            "counter disagrees with the {refused} observed 429s:\n{body}"
        );
    }

    /// The in-flight gauge must come back down — including when a handler panics, which
    /// unwinds straight through the metrics middleware.
    #[test]
    fn in_flight_guard_decrements_on_unwind() {
        let reg = Arc::new(Registry::new());
        assert_eq!(reg.in_flight(), 0);
        let r = std::panic::catch_unwind({
            let reg = reg.clone();
            move || {
                let _g = reg.enter();
                assert_eq!(reg.in_flight(), 1);
                panic!("handler exploded");
            }
        });
        assert!(r.is_err());
        assert_eq!(reg.in_flight(), 0, "gauge leaked across a panic");
    }

    /// The cardinality backstop actually drops, and says so.
    #[test]
    fn cardinality_cap_drops_and_counts() {
        let reg = Registry::new();
        for i in 0..(Registry::MAX_SERIES + 5) {
            reg.record("GET", &format!("/r{i}"), 200, Duration::from_millis(1));
        }
        assert_eq!(
            reg.requests_total("GET", "/r0", 200),
            1,
            "early series kept"
        );
        let over = format!("/r{}", Registry::MAX_SERIES + 1);
        assert_eq!(
            reg.requests_total("GET", &over, 200),
            0,
            "series past the cap must not exist"
        );
        assert!(
            reg.dropped_series() >= 5,
            "drops went uncounted: {}",
            reg.dropped_series()
        );
    }

    /// Scraping is not public. `ServiceAuth` fails closed, so an API with no
    /// `SERVICE_TOKEN` configured answers 401 rather than publishing its route table.
    #[tokio::test]
    async fn metrics_requires_the_service_token() {
        let app = app_with(dead_pool());
        let (st, _) = call(&app, "GET", "/metrics", false).await;
        assert_eq!(st, StatusCode::UNAUTHORIZED);

        let mut cfg = Config::for_tests("postgres://unused", "s");
        cfg.service_token = String::new();
        let unconfigured = router(AppState::new(dead_pool(), cfg));
        let (st, _) = call(&unconfigured, "GET", "/metrics", true).await;
        assert_eq!(
            st,
            StatusCode::UNAUTHORIZED,
            "empty SERVICE_TOKEN must fail closed"
        );
    }

    /// Exposition sanity: every family declares its TYPE exactly once, before its samples.
    #[tokio::test]
    async fn exposition_declares_each_family_once() {
        let app = app_with(dead_pool());
        let _ = call(&app, "GET", "/healthz", false).await;
        let (_, body) = call(&app, "GET", "/metrics", true).await;
        let types: Vec<&str> = body.lines().filter(|l| l.starts_with("# TYPE ")).collect();
        let mut names: Vec<&str> = types.iter().filter_map(|l| l.split(' ').nth(2)).collect();
        let before = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(
            before,
            names.len(),
            "a family declared TYPE twice: {types:?}"
        );
        assert!(before >= 9, "only {before} families declared:\n{body}");
    }

    // ───────────────────── health: it can go red ─────────────────────

    /// A health check that cannot fail is worse than none. With the database
    /// unreachable both checks report down and the probe is 503.
    ///
    /// Reads the **detailed** payload (T-580 moved `checks` behind `X-Service-Token`), because
    /// the claim under test is that each check can go red independently, and that is only
    /// visible per-check.
    #[tokio::test]
    async fn healthz_goes_red_when_the_database_is_unreachable() {
        let app = app_with(dead_pool());
        let (st, body) = call(&app, "GET", "/healthz", true).await;
        assert_eq!(st, StatusCode::SERVICE_UNAVAILABLE);
        let v: serde_json::Value = serde_json::from_str(&body).expect("healthz json");
        assert_eq!(v["status"], "unavailable", "legacy status string preserved");
        assert_eq!(v["checks"]["database"]["status"], "down");
        assert_eq!(v["checks"]["migrations"]["status"], "down");
        assert!(
            v["checks"]["database"]["error"]
                .as_str()
                .is_some_and(|e| !e.is_empty()),
            "a down check must say why: {body}"
        );
    }

    /// T-580 — the public probe discloses **only** `status`, and the 200/503 split survives.
    ///
    /// Measured against the pre-fix handler by wave 69's verifier:
    /// `version=0.1.0  uptime=396  pool={connections:5, idle:4}  migrations.applied=18`, to any
    /// unauthenticated caller. Every one of those is asserted absent here — by key AND by value,
    /// because a handler that renamed `version` to `build` would satisfy a key-only check while
    /// disclosing exactly the same thing.
    #[tokio::test]
    async fn healthz_discloses_nothing_to_an_unauthenticated_caller() {
        let app = app_with(dead_pool());
        let (st, body) = call(&app, "GET", "/healthz", false).await;
        // The contract every prober reads is unchanged: the code, and `status`.
        assert_eq!(st, StatusCode::SERVICE_UNAVAILABLE);
        let v: serde_json::Value = serde_json::from_str(&body).expect("healthz json");
        assert_eq!(v["status"], "unavailable");

        let obj = v.as_object().expect("healthz is a JSON object");
        assert_eq!(
            obj.keys().collect::<Vec<_>>(),
            vec!["status"],
            "public /healthz must carry exactly one field, got: {body}"
        );
        // Value-level: the build string, the pool depth and the migration count must not appear
        // anywhere in the bytes, under any key.
        for leak in [env!("CARGO_PKG_VERSION"), "uptime", "pool", "migration"] {
            assert!(
                !body.contains(leak),
                "public /healthz leaked {leak:?}: {body}"
            );
        }
    }

    /// …and the same route with the service token still serves the whole report, so T-580 is a
    /// relocation rather than a deletion. Without this, "discloses nothing" is satisfiable by a
    /// handler that lost the detail entirely.
    #[tokio::test]
    async fn healthz_detail_is_served_to_a_service_token() {
        let app = app_with(dead_pool());
        let (_, body) = call(&app, "GET", "/healthz", true).await;
        let v: serde_json::Value = serde_json::from_str(&body).expect("healthz json");
        assert_eq!(v["version"], env!("CARGO_PKG_VERSION"));
        assert!(v["uptime_seconds"].is_u64(), "{body}");
        assert!(v["pool"]["connections"].is_u64(), "{body}");
        assert!(v["checks"]["migrations"]["applied"].is_i64(), "{body}");
    }

    /// A **wrong** token gets the public payload, not a 401 — the probers are credential-less and
    /// a 401 would fail `curl -fsS` in `preflight.sh` / `editor-gates.yml` / `smokes.rs`.
    #[tokio::test]
    async fn healthz_with_a_wrong_token_downgrades_rather_than_rejecting() {
        let app = app_with(dead_pool());
        let resp = app
            .clone()
            .oneshot(
                HttpRequest::builder()
                    .method("GET")
                    .uri("/healthz")
                    .header("x-service-token", "not-the-token")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("router call");
        // 503 because the pool is dead — never 401/403.
        assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = to_bytes(resp.into_body(), 1 << 20).await.expect("body");
        let body = String::from_utf8_lossy(&body).into_owned();
        let v: serde_json::Value = serde_json::from_str(&body).expect("healthz json");
        assert_eq!(v.as_object().expect("object").keys().len(), 1, "{body}");
        assert_eq!(v["status"], "unavailable");
    }

    // ───────────────────── render unit checks ─────────────────────

    /// Label escaping is enforced, not assumed.
    #[test]
    fn label_values_are_escaped() {
        let reg = Registry::new();
        reg.record("GET", "/weird\"\\path", 200, Duration::from_millis(2));
        let out = reg.render(&Scrape {
            db_up: true,
            db_ping: Duration::from_millis(1),
            pool_connections: 3,
            pool_idle: 1,
        });
        assert!(
            out.contains("route=\"/weird\\\"\\\\path\""),
            "unescaped label value would produce unparseable exposition:\n{out}"
        );
        assert_eq!(
            value(&out, "tbd_db_pool_connections{state=\"in_use\"}"),
            Some(2.0)
        );
    }
}
