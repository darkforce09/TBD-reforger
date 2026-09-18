//! Per-client rate limiting, in two tiers.
//!
//! **L1 — [`IpLimiter`], in memory, on every request that reaches this middleware.** Global
//! `20/s` burst `40`; the [`STRICT_PREFIXES`] get `1/s` burst `10`. It costs nothing, it runs
//! first, and it absorbs a flood before any of it can reach the database.
//!
//! **L2 — [`PgRateLimiter`], in Postgres, strict prefixes only.** Same numbers as the strict
//! tier, keyed the same way, consulted only after L1 has already said yes. The invariants:
//!
//! * **Which routes.** Only `/api/v1/auth/` and `/api/v1/ingest/`. `/auth/*` is the only
//!   unauthenticated family in the tree, where a restart-reset bucket buys free retries against
//!   single-use refresh-token rotation and the Discord OAuth round trip; `/ingest/*` writes
//!   `matches` / `match_player_stats` on a shared service token. L2 costs **one database write
//!   per request**, and the SPA's traffic is overwhelmingly the *other* routes — the dashboard's
//!   parallel GET fan-out, `/missions`, and the Mission Creator's thousands of map-asset tiles.
//!   Widening this list puts a write on the editor's hot path to protect nothing.
//! * **Keyed on what.** `scope|client-ip`, via `bucket_key`, over the one address
//!   `client_identity::client_ip` resolves. Both tiers key identically, from a single resolution
//!   per request, so they cannot disagree about who a caller is. A token- or user-keyed bucket
//!   cannot exist here: the requests that most need limiting are the ones with no valid
//!   credential yet.
//! * **What happens on trip.** `429` + `Retry-After` + the `{"error": …}` envelope the SPA reads.
//!   `Retry-After` is derived from the tripped limiter's own refill rate, so it cannot drift away
//!   from the policy it describes.
//! * **Fail closed.** When the durable limiter's store errors this returns **`503`**, never
//!   "allowed". Every strict-prefix handler needs that same database anyway, so a request allowed
//!   past a broken limiter would only reach a handler that 500s.
//! * **The exempt mount is structural, not a path test.** [`RATE_LIMIT_EXEMPT_MOUNT`] is served
//!   **outside** this middleware entirely: `Router::layer` wraps the routes registered before it,
//!   so [`crate::core::http_router::router`] mounts that `ServeDir` on the line *after* the
//!   `rate_limit` layer. The limiter is never asked about a map asset, so it can never get the
//!   answer wrong. This module holds exactly one path predicate — [`STRICT_PREFIXES`] — and that
//!   is the only one it should ever hold: a `starts_with("/map-assets")` here would also exempt
//!   `/map-assets-admin`, would be defeated by `/api/v1/../map-assets`, and would put a second,
//!   silently-diverging notion of "which route is this" next to axum's own.
//!
//! The mount is exempt because its cost is **bytes, not requests**. One un-`Range`d
//! `GET /map-assets/everon/satellite/everon-sat.tbd-sat` is 152,713,114 B and spends one token;
//! the 49 polite Range spans that replace it fetch the same bytes and spend 49 — a request-rate
//! ceiling charges the well-behaved client 49× what it charges the greedy one. A cold Mission
//! Creator boot touches 951 distinct files under this mount and peaks at 2,115 requests inside one
//! second, so no ceiling both clears a real boot and refuses a scraper. Bytes, concurrency and
//! caching belong to the reverse proxy, which already has its own `/map-assets` block. The
//! exemption is one named mount: `/uploads` — the other `ServeDir` — serves user-uploaded content
//! and stays limited.

use std::net::IpAddr;
use std::num::NonZeroU32;
use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use governor::clock::DefaultClock;
use governor::state::keyed::DefaultKeyedStateStore;
use governor::{Quota, RateLimiter};

use crate::core::application_state::AppState;
use crate::core::configuration::proxy_network::{ProxyNet, parse_trusted_proxies};
use crate::core::middleware::client_identity::client_ip;
use crate::core::middleware::durable_ratelimit::{PgRateLimiter, bucket_key};
use crate::core::middleware::json_error;

/// Full rooted-path prefixes that get the strict limiter (a prefix match, not a substring one).
///
/// This is also the durable tier's entire surface — see the module header for why.
pub const STRICT_PREFIXES: [&str; 2] = ["/api/v1/auth/", "/api/v1/ingest/"];

/// The router mount point that is served **outside** [`rate_limit`] entirely.
///
/// This is a **`Router::nest_service` argument**, not a request-path predicate. It is passed to
/// axum once, at registration, in [`crate::core::http_router::router`]; nothing in this module
/// compares it against `req.uri().path()`, and nothing should.
///
/// The literal is pinned by `tests::the_exempt_mount_is_the_path_the_editor_requests` against the
/// URL the SPA actually builds (`world_assets/mod.rs`: `format!("/map-assets/{terrain}")`) — the
/// mount and the client cannot drift apart silently.
pub const RATE_LIMIT_EXEMPT_MOUNT: &str = "/map-assets";

/// The glyph atlas mount, exempt for the same reason and registered the same way.
///
/// Glyphs are shared by every terrain, so on disk they sit beside the terrain tree rather than
/// inside it. The URL keeps them under `/map-assets/` because that is what the map client already
/// requests (`world_loader/atlas.rs` builds `/map-assets/glyphs/atlas/world-glyphs.webp`), so the
/// two directories are joined at the router rather than on disk.
///
/// This is a more specific path than [`RATE_LIMIT_EXEMPT_MOUNT`], and axum resolves the static
/// segment ahead of the catch-all regardless of registration order.
pub const RATE_LIMIT_EXEMPT_GLYPH_MOUNT: &str = "/map-assets/glyphs";

/// Bucket scope for the durable strict tier. One scope, because the two prefixes share one
/// policy; `bucket_key` keeps it independent of any future scope.
pub const DURABLE_STRICT_SCOPE: &str = "strict";

/// Durable strict tier: sustained requests per second. Same as the in-memory strict tier.
pub const DURABLE_STRICT_RPS: u32 = 1;
/// Durable strict tier: bucket capacity. Same as the in-memory strict tier.
pub const DURABLE_STRICT_BURST: u32 = 10;

/// Seconds a refused client should wait for one token, from the limiter's own refill rate.
///
/// Always at least 1: `Retry-After: 0` invites an immediate retry, which is the opposite of the
/// instruction. A rate of 0 (a bucket that never refills) also yields 1 rather than infinity —
/// there is no honest finite answer, and 1 keeps the header well-formed.
fn retry_after_secs(refill_per_second: f64) -> u64 {
    if refill_per_second <= 0.0 {
        return 1;
    }
    ((1.0 / refill_per_second).ceil() as u64).max(1)
}

/// Per-client-IP token bucket keyed by IP. In-memory, single-process (L1).
pub struct IpLimiter {
    inner: RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>,
    /// Retained for [`IpLimiter::retry_after_secs`] — `governor` does not expose its own quota.
    per_second: u32,
}

impl IpLimiter {
    /// `per_second` sustained rate with the given `burst` bucket size.
    pub fn new(per_second: u32, burst: u32) -> Self {
        let quota = Quota::per_second(NonZeroU32::new(per_second).expect("per_second > 0"))
            .allow_burst(NonZeroU32::new(burst).expect("burst > 0"));
        Self {
            inner: RateLimiter::keyed(quota),
            per_second,
        }
    }

    /// True if the request for `ip` is allowed (a token was available).
    pub fn check(&self, ip: IpAddr) -> bool {
        self.inner.check_key(&ip).is_ok()
    }

    /// `Retry-After` value for a client this limiter just refused.
    pub fn retry_after_secs(&self) -> u64 {
        retry_after_secs(f64::from(self.per_second))
    }
}

/// Middleware state for [`rate_limit`]: the app state (L1 limiters live there) plus the durable
/// tier.
///
/// The `PgRateLimiter` is built here rather than in `AppState` on purpose. It is a middleware
/// concern with exactly one consumer, `AppState` is a different module's file, and threading it
/// through the state struct would make every construction site of `AppState` — including every
/// integration suite — care about a limiter none of them configure.
#[derive(Clone)]
pub struct RateLimitState {
    pub app: AppState,
    pub durable_strict: Arc<PgRateLimiter>,
    /// The reverse proxies whose `X-Forwarded-For` this process believes, parsed once at
    /// construction. **Empty means trust none**, which is the shipped default and leaves the
    /// header ignored entirely.
    pub trusted_proxies: Arc<[ProxyNet]>,
}

impl RateLimitState {
    /// Production wiring: the durable strict tier on the app's own pool, at the strict policy,
    /// plus the parsed trusted-proxy list from config.
    ///
    /// A `TRUSTED_PROXIES` entry that does not parse cannot reach here — [`Config::load`] refuses
    /// it at boot. The `Err` arm is for a hand-built `Config` (a test, a future embedder), and it
    /// **drops the whole list**: a partially-understood trust list is the one input where "use
    /// what parsed" is the wrong answer, because the entries that failed are exactly the ones
    /// nobody has checked. Trusting nobody restores the shared-bucket behaviour, which is safe.
    ///
    /// [`Config::load`]: crate::core::configuration::Config::load
    pub fn new(app: AppState) -> Self {
        let durable_strict = Arc::new(PgRateLimiter::new(
            app.pool.clone(),
            DURABLE_STRICT_RPS,
            DURABLE_STRICT_BURST,
        ));
        let trusted_proxies: Arc<[ProxyNet]> = match parse_trusted_proxies(&app.cfg.trusted_proxies)
        {
            Ok(nets) => nets.into(),
            Err((entry, why)) => {
                tracing::error!(
                    entry = %entry,
                    reason = %why,
                    "TRUSTED_PROXIES entry is malformed — trusting NO proxy and ignoring \
                     X-Forwarded-For entirely (rate limiting falls back to the connection peer)"
                );
                Arc::from(Vec::new())
            }
        };
        Self {
            app,
            durable_strict,
            trusted_proxies,
        }
    }
}

/// 429 with the `Retry-After` the tripped limiter dictates. `{"error": "rate limit exceeded"}` is
/// the shipped body contract and the SPA reads it.
fn too_many(retry_after: u64) -> Response {
    with_retry_after(
        json_error(StatusCode::TOO_MANY_REQUESTS, "rate limit exceeded").into_response(),
        retry_after,
    )
}

/// 503 for a durable tier that could not reach its store. Deliberately **not** a pass: see the
/// module header. Carries `Retry-After` for the same reason a 429 does.
fn limiter_unavailable(retry_after: u64) -> Response {
    with_retry_after(
        json_error(StatusCode::SERVICE_UNAVAILABLE, "rate limiter unavailable").into_response(),
        retry_after,
    )
}

fn with_retry_after(mut resp: Response, secs: u64) -> Response {
    // `secs` is a `u64` rendered as decimal digits, so this header value is always valid; the
    // fallback exists only so a malformed value could never panic the request path.
    if let Ok(v) = HeaderValue::from_str(&secs.to_string()) {
        resp.headers_mut().insert(header::RETRY_AFTER, v);
    }
    resp
}

/// L1 for every request that reaches this middleware; L2 (durable) additionally for the strict
/// prefixes.
///
/// "Every request that reaches this middleware" is the whole of the static-mount exemption:
/// nothing below tests for [`RATE_LIMIT_EXEMPT_MOUNT`], because a map-asset request never arrives
/// here — the router hands it to a `ServeDir` mounted outside this layer. One path predicate lives
/// in this function ([`STRICT_PREFIXES`]) and that is the only one there should ever be.
pub async fn rate_limit(State(rl): State<RateLimitState>, req: Request, next: Next) -> Response {
    let path = req.uri().path();
    let strict = STRICT_PREFIXES.iter().any(|p| path.starts_with(p));
    let limiter = if strict {
        &rl.app.rl_strict
    } else {
        &rl.app.rl_global
    };

    // One notion of "who" for both tiers. Resolving once here is what keeps them identical: two
    // calls could not disagree even if the rules changed.
    let ip = client_ip(&req, &rl.trusted_proxies);

    // ── L1: in-memory. Peerless requests fall back to the unspecified address so they still
    // share a bucket rather than being unlimited.
    if !limiter.check(ip.unwrap_or(IpAddr::from([0, 0, 0, 0]))) {
        return too_many(limiter.retry_after_secs());
    }

    // ── L2: durable, strict prefixes only, and only for a request with a real client.
    if strict && let Some(ip) = ip {
        let key = bucket_key(DURABLE_STRICT_SCOPE, ip);
        match rl.durable_strict.check(&key).await {
            Ok(true) => {}
            Ok(false) => return too_many(rl.durable_strict.retry_after_secs()),
            Err(e) => {
                tracing::error!(
                    error = %e,
                    bucket = %key,
                    "durable rate limiter unreachable — refusing (fail closed)"
                );
                return limiter_unavailable(rl.durable_strict.retry_after_secs());
            }
        }
    }

    next.run(req).await
}

#[cfg(test)]
#[path = "tests/rate_limiting.rs"]
mod tests;
