//! Per-client-IP rate limiting — **two tiers** since T-578.
//!
//! # The defect T-578 closes
//!
//! T-280 built and proved [`PgRateLimiter`], a durable token bucket whose state lives in Postgres,
//! and then wired it to nothing. The live limiter stayed the in-memory `governor` [`IpLimiter`],
//! so the API carried the code for durable rate limiting and had none of the protection: every
//! restart handed an abuser a fresh full bucket, and two API processes each enforced the limit
//! separately (N processes = N× the intended rate). A limiter that is present but never consulted
//! is indistinguishable from no limiter.
//!
//! # The policy, and why it is this one
//!
//! **L1 — [`IpLimiter`], in memory, every request.** Global `20/s` burst `40`; the strict prefixes
//! get `1/s` burst `10`. Unchanged from the Go original. It costs nothing, it runs first, and it
//! absorbs a flood before any of it can reach the database.
//!
//! **L2 — [`PgRateLimiter`], in Postgres, strict prefixes only.** Same numbers as the strict tier
//! (`1/s` burst `10`), keyed the same way, consulted only after L1 has already said yes.
//!
//! Three decisions are load-bearing:
//!
//! 1. **Which routes.** Only [`STRICT_PREFIXES`] — `/api/v1/auth/` and `/api/v1/ingest/`. Those are
//!    where the in-memory limiter's two documented defects actually bite: `/auth/*` is the only
//!    unauthenticated family in the tree (a restart-reset bucket is free retries against
//!    single-use refresh-token rotation and against the Discord OAuth round trip), and `/ingest/*`
//!    writes `matches` / `match_player_stats` on a shared service token. Everything else keeps L1
//!    only. That is not laziness, it is the cost trade the ticket demands be made deliberately: L2
//!    is **one database write per request**, and the SPA's traffic is overwhelmingly the *other*
//!    routes — the dashboard's parallel GET fan-out, `/missions`, and the Mission Creator pulling
//!    thousands of `/map-assets` tiles per session. Blanket-limiting those in Postgres would put a
//!    write on the editor's hot path to protect nothing. Auth and ingest are, by contrast, a
//!    handful of requests per user session and a heartbeat from a handful of game servers.
//! 2. **Keyed on what.** `scope|client-ip`, via [`bucket_key`] — the same trust-none `ConnectInfo`
//!    peer L1 keys on. Deliberately identical: two different notions of "who" between the tiers
//!    would be a second bug, and a token- or user-keyed bucket cannot exist here because the
//!    requests that most need limiting are the ones with no valid credential yet. See
//!    [`client_ip`] for what happens when there is no peer, and **§Behind a reverse proxy** below
//!    for the honest limits of IP keying on this deployment.
//! 3. **What happens on trip.** `429` + `Retry-After` + the unchanged `{"error": …}` envelope.
//!    `Retry-After` is derived from the tripped limiter's own refill rate rather than being a
//!    constant, so it cannot drift away from the policy it describes.
//!
//! # Fail closed
//!
//! When the durable limiter's store errors, this returns **`503`**, never "allowed".
//! [`PgRateLimiter::check`] hands back the `sqlx::Error` precisely so the caller has to decide
//! loudly, and a limiter that opens up when its store is unreachable is the same defect class as
//! the one this ticket closes. The cost of failing closed is nil in practice: every strict-prefix
//! handler needs that same database anyway, so a request allowed past a broken limiter would only
//! reach a handler that 500s.
//!
//! # Behind a reverse proxy
//!
//! `scripts/deploy/Caddyfile.website` fronts this API, so in production the `ConnectInfo` peer for
//! public traffic is Caddy's loopback address and **all public clients share one bucket**. That is
//! true of L1 today and T-578 does not change it — but it is the reason the durable tier keeps the
//! strict tier's existing `1/s` burst `10` rather than tightening it: the numbers are already
//! deployed and known not to lock the site out, and durability (surviving a restart, shared across
//! processes) is the property being added, not a tighter quota. `Config::trusted_proxies` exists
//! for exactly this and is read by nothing; wiring `X-Forwarded-For` behind it is the follow-up
//! that would make per-IP keying mean per-client, and it is a config + security change rather than
//! a wiring one.

use std::net::{IpAddr, SocketAddr};
use std::num::NonZeroU32;
use std::sync::Arc;

use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use governor::clock::DefaultClock;
use governor::state::keyed::DefaultKeyedStateStore;
use governor::{Quota, RateLimiter};

use crate::app::durable_ratelimit::{PgRateLimiter, bucket_key};
use crate::middleware::json_error;
use crate::state::AppState;

/// Full rooted-path prefixes that get the strict limiter (HasPrefix, not substring).
///
/// This is also the durable tier's entire surface — see the module header for why.
pub const STRICT_PREFIXES: [&str; 2] = ["/api/v1/auth/", "/api/v1/ingest/"];

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
/// concern with exactly one consumer, `AppState` is a different slice's file, and threading it
/// through the state struct would make every construction site of `AppState` — including every
/// integration suite — care about a limiter none of them configure.
#[derive(Clone)]
pub struct RateLimitState {
    pub app: AppState,
    pub durable_strict: Arc<PgRateLimiter>,
}

impl RateLimitState {
    /// Production wiring: the durable strict tier on the app's own pool, at the strict policy.
    pub fn new(app: AppState) -> Self {
        let durable_strict = Arc::new(PgRateLimiter::new(
            app.pool.clone(),
            DURABLE_STRICT_RPS,
            DURABLE_STRICT_BURST,
        ));
        Self {
            app,
            durable_strict,
        }
    }
}

/// The client this request is attributable to, or `None` when it has no client.
///
/// Trust-none: the address is the direct connection peer from [`ConnectInfo`], never a header.
///
/// `None` means the request did not arrive over a socket — there is no `ConnectInfo` extension at
/// all, or the peer is the unspecified address. **`0.0.0.0` / `::` are not client addresses**; a
/// packet cannot originate from them, so a request bearing one was synthesised in-process (a
/// `tower::ServiceExt::oneshot` call in a test, a future in-process mount). Attributing such a
/// request to a durable per-client bucket would file every in-process caller under one key and
/// leave a persistent row for a client that does not exist.
///
/// The production path always has a real peer: `bin/api.rs` serves with
/// `into_make_service_with_connect_info::<SocketAddr>()`, which installs the extension for every
/// accepted connection. That is not a comment relying on good behaviour —
/// `t578_ratelimit::api_binary_still_installs_connect_info` reads `src/bin/api.rs` and fails if it
/// stops doing so, because the day it does, this function starts returning `None` in production
/// and both tiers quietly stop distinguishing clients.
fn client_ip(req: &Request) -> Option<IpAddr> {
    let ip = req.extensions().get::<ConnectInfo<SocketAddr>>()?.0.ip();
    if ip.is_unspecified() { None } else { Some(ip) }
}

/// 429 with the `Retry-After` the tripped limiter dictates. Body envelope unchanged from T-145 —
/// `{"error": "rate limit exceeded"}` is the shipped contract and the SPA reads it.
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

/// L1 for every request; L2 (durable) additionally for the strict prefixes.
pub async fn rate_limit(State(rl): State<RateLimitState>, req: Request, next: Next) -> Response {
    let path = req.uri().path();
    let strict = STRICT_PREFIXES.iter().any(|p| path.starts_with(p));
    let limiter = if strict {
        &rl.app.rl_strict
    } else {
        &rl.app.rl_global
    };

    let ip = client_ip(&req);

    // ── L1: in-memory. Peerless requests fall back to the unspecified address so they still
    // share a bucket rather than being unlimited — this is exactly the pre-T-578 behaviour.
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
mod tests {
    use super::*;

    #[test]
    fn allows_burst_then_throttles() {
        let l = IpLimiter::new(1, 5); // 1 req/s, burst 5
        let ip = IpAddr::from([1, 2, 3, 4]);
        let allowed = (0..40).filter(|_| l.check(ip)).count();
        // GCRA lets the burst through, then throttles (a token may replenish mid-loop).
        assert!((5..=6).contains(&allowed), "burst ~5, got {allowed}");
    }

    #[test]
    fn limiters_are_keyed_per_ip() {
        let l = IpLimiter::new(1, 2);
        let a = IpAddr::from([10, 0, 0, 1]);
        let b = IpAddr::from([10, 0, 0, 2]);
        assert!(l.check(a) && l.check(a)); // a's burst
        assert!(!l.check(a)); // a throttled
        assert!(l.check(b)); // b is independent
    }

    #[test]
    fn strict_prefix_is_rooted_not_substring() {
        let strict = |path: &str| STRICT_PREFIXES.iter().any(|p| path.starts_with(p));
        assert!(strict("/api/v1/auth/refresh"));
        assert!(strict("/api/v1/ingest/server-status"));
        // Global paths use the global bucket.
        assert!(!strict("/api/v1/announcements"));
        assert!(!strict("/api/v1/missions"));
        // "auth" as a substring (e.g. /oauth/) is NOT the rooted /auth/ prefix.
        assert!(!strict("/api/v1/oauth/authorize"));
    }

    /// T-578: the durable tier's surface is exactly the strict tier's, and the routes the SPA
    /// leans on are outside it. A change that quietly widened `STRICT_PREFIXES` to `/api/v1/`
    /// would put a database write on every editor tile fetch.
    #[test]
    fn durable_tier_excludes_the_spa_hot_paths() {
        let durable = |path: &str| STRICT_PREFIXES.iter().any(|p| path.starts_with(p));
        for hot in [
            "/api/v1/missions/00000000-0000-0000-0000-000000000000",
            "/api/v1/dashboard",
            "/api/v1/announcements",
            "/map-assets/everon/objects/12_9.bin",
            "/uploads/x.png",
            "/healthz",
            "/metrics",
        ] {
            assert!(!durable(hot), "{hot} must not reach the durable limiter");
        }
        assert!(durable("/api/v1/auth/refresh"));
        assert!(durable("/api/v1/ingest/match-results"));
    }

    /// `Retry-After` tracks the policy instead of being a constant, and is never 0.
    #[test]
    fn retry_after_is_derived_and_never_zero() {
        assert_eq!(retry_after_secs(1.0), 1); // strict: one token per second
        assert_eq!(retry_after_secs(20.0), 1); // global: sub-second, floored to 1
        assert_eq!(retry_after_secs(0.25), 4); // a token every four seconds
        assert_eq!(retry_after_secs(0.0), 1); // never-refills → still well-formed
        assert_eq!(IpLimiter::new(1, 10).retry_after_secs(), 1);
        assert_eq!(IpLimiter::new(20, 40).retry_after_secs(), 1);
    }

    /// The durable tier's numbers are the strict tier's numbers. `AppState::new` builds
    /// `rl_strict` as `IpLimiter::new(1, 10)`; if one side is retuned and the other is not, L1
    /// silently becomes the only limiter that can ever refuse (or L2 starts refusing traffic L1
    /// was sized to allow).
    #[test]
    fn durable_strict_policy_matches_the_in_memory_strict_policy() {
        assert_eq!(DURABLE_STRICT_RPS, 1);
        assert_eq!(DURABLE_STRICT_BURST, 10);
        let src = include_str!("../state.rs");
        assert!(
            src.contains(&format!(
                "IpLimiter::new({DURABLE_STRICT_RPS}, {DURABLE_STRICT_BURST})"
            )),
            "state.rs no longer builds rl_strict as IpLimiter::new({DURABLE_STRICT_RPS}, \
             {DURABLE_STRICT_BURST}) — retune the durable tier with it"
        );
    }
}
