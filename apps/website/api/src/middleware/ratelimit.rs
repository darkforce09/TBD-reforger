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
//! # Behind a reverse proxy — T-625
//!
//! `scripts/deploy/Caddyfile.website` fronts this API from loopback, so the `ConnectInfo` peer for
//! **every** public client is Caddy. Through T-624 that meant every public client keyed to one
//! `strict|127.0.0.1` bucket at `1/s` burst `10`: on an op night, the 11th member to open the site
//! within ten seconds got a `429` on `/auth/refresh` and rendered logged-out. (The `/auth/*` case
//! is the one the ticket names, but the global tier shared its key the same way — `20/s` burst `40`
//! across the whole community, including the Mission Creator's tile fetches.)
//!
//! [`client_ip`] now honours `X-Forwarded-For`, and **only** behind [`Config::trusted_proxies`].
//! Three rules make that safe, because a forgeable rate-limit key is strictly worse than a shared
//! one — shared means everyone is limited together, forgeable means nobody is limited at all:
//!
//! 1. **The immediate peer must be a trusted proxy.** A client connecting directly can set any
//!    header it likes and gets its own connection's address regardless. This is the rule that
//!    makes the header unusable as a spoofing tool.
//! 2. **The hop taken is the rightmost untrusted one**, never the leftmost. Caddy *appends* the
//!    address it observed, so the rightmost entry is the proxy's own measurement and everything
//!    left of it is whatever the client sent. Taking the leftmost — the common implementation, and
//!    the one that turns this fix into a vulnerability — would let any client mint a fresh bucket
//!    per request by varying one header.
//! 3. **Empty [`Config::trusted_proxies`] ignores the header entirely**, which is the shipped
//!    default and is byte-for-byte T-624's behaviour. Nothing about this change alters an
//!    unconfigured deployment.
//!
//! Anything the chain cannot answer honestly — no header, an unparseable hop, `0.0.0.0`, a chain
//! made entirely of trusted proxies — falls back to the peer, i.e. to the shared bucket that was
//! the old behaviour. Fail-closed here means "limit them together", never "let them through".
//!
//! **What this deployment still assumes**, stated so it is not mistaken for a proof: that a host
//! listed in `TRUSTED_PROXIES` really does overwrite/append `X-Forwarded-For` with the address it
//! saw (Caddy's `reverse_proxy` does), and that nothing hostile can connect to the API from a
//! trusted address. Trusting a proxy *is* trusting it to report its clients honestly; there is no
//! version of this feature where that is not true.
//!
//! The numbers are **unchanged** at `1/s` burst `10` (strict) and `20/s` burst `40` (global).
//! Per-client, a burst of 10 is far more than the SPA's bootstrap spends — the op-night defect was
//! the shared *key*, not the quota — and `state.rs` builds `rl_strict` from the same pair, pinned
//! by `tests::durable_strict_policy_matches_the_in_memory_strict_policy`. Retuning is a separate
//! judgement from fixing the key, and this slice only fixed the key.
//!
//! [`Config::trusted_proxies`]: crate::config::Config::trusted_proxies

use std::net::{IpAddr, SocketAddr};
use std::num::NonZeroU32;
use std::sync::Arc;

use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use governor::clock::DefaultClock;
use governor::state::keyed::DefaultKeyedStateStore;
use governor::{Quota, RateLimiter};

use crate::app::durable_ratelimit::{PgRateLimiter, bucket_key};
use crate::config::{ProxyNet, parse_trusted_proxies};
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
    /// T-625 — the reverse proxies whose `X-Forwarded-For` this process believes, parsed once at
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
    /// [`Config::load`]: crate::config::Config::load
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

/// How far back through `X-Forwarded-For` this will look before giving up and using the peer.
///
/// The scan stops at the first untrusted hop, so a real chain costs one iteration. The cap only
/// binds on a chain of 64+ *trusted* hops, which no deployment has and a client cannot manufacture
/// (it would have to make every entry match `TRUSTED_PROXIES`); giving up there costs the shared
/// bucket, not a bypass. It exists so a pathological header cannot buy unbounded work per request.
const MAX_FORWARDED_HOPS: usize = 64;

/// The forwarding header this reads, lowercase because `HeaderMap` lookups are case-insensitive
/// on a lowercase key.
///
/// Named here rather than taken from `http::header` because `X-Forwarded-For` is a de-facto
/// standard, not a registered one, and the crate has no constant for it.
const X_FORWARDED_FOR: &str = "x-forwarded-for";

/// The client this request is attributable to, or `None` when it has no client.
///
/// The address is the direct connection peer from [`ConnectInfo`], **unless** that peer is a
/// configured trusted proxy, in which case the chain in `X-Forwarded-For` is consulted — see the
/// module header for the three rules and why the rightmost untrusted hop is the only safe choice.
/// With `trusted` empty (the default) this is exactly the pre-T-625 function: the peer, always.
///
/// `None` means the request did not arrive over a socket — there is no `ConnectInfo` extension at
/// all, or the peer is the unspecified address. **`0.0.0.0` / `::` are not client addresses**; a
/// packet cannot originate from them, so a request bearing one was synthesised in-process (a
/// `tower::ServiceExt::oneshot` call in a test, a future in-process mount). Attributing such a
/// request to a durable per-client bucket would file every in-process caller under one key and
/// leave a persistent row for a client that does not exist.
///
/// The peer is canonicalised (`::ffff:1.2.3.4` → `1.2.3.4`) so one client is one bucket whether the
/// listener is dual-stack or not — otherwise the same client keys two different buckets depending
/// on how the socket was opened, and `TRUSTED_PROXIES=127.0.0.1` would match nothing on a
/// dual-stack listener.
///
/// The production path always has a real peer: `bin/api.rs` serves with
/// `into_make_service_with_connect_info::<SocketAddr>()`, which installs the extension for every
/// accepted connection. That is not a comment relying on good behaviour —
/// `t578_ratelimit::api_binary_still_installs_connect_info` reads `src/bin/api.rs` and fails if it
/// stops doing so, because the day it does, this function starts returning `None` in production
/// and both tiers quietly stop distinguishing clients.
fn client_ip(req: &Request, trusted: &[ProxyNet]) -> Option<IpAddr> {
    let peer = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()?
        .0
        .ip()
        .to_canonical();
    if peer.is_unspecified() {
        return None;
    }
    // Rule 1: the header is only evidence when the hop that handed it to us is one we trust.
    // A direct client's header is not evidence of anything.
    if !trusted.iter().any(|net| net.contains(peer)) {
        return Some(peer);
    }
    Some(forwarded_client(req.headers(), trusted).unwrap_or(peer))
}

/// The rightmost `X-Forwarded-For` hop that is **not** a trusted proxy, or `None` to fall back.
///
/// Reads right-to-left because that is the direction trust flows: the last entry was written by
/// the proxy we just accepted the connection from, the one before it by the proxy before that, and
/// everything left of the first untrusted entry is client-supplied text. The first entry that is
/// not a trusted proxy is therefore the furthest hop anyone in the chain actually *observed*.
///
/// `None` — meaning "use the peer" — for every case where the chain cannot be read honestly:
///
/// * no `X-Forwarded-For` header at all (a proxy that does not set one);
/// * a hop that is not an address (`unknown`, an RFC 7239 obfuscated token, junk). The scan
///   **stops** there rather than skipping it: skipping would let a client shift which entry is
///   read by injecting one unparseable value, which is the leftmost-hop bug wearing a hat;
/// * `0.0.0.0` / `::`, which is not a source address any packet can carry;
/// * a chain made entirely of trusted proxies, or longer than [`MAX_FORWARDED_HOPS`].
///
/// Only `X-Forwarded-For` is read. RFC 7239 `Forwarded` is not consulted because the deployed
/// proxy does not send it, and a header nothing writes is a parser nothing tests.
fn forwarded_client(headers: &HeaderMap, trusted: &[ProxyNet]) -> Option<IpAddr> {
    let hops: Vec<&str> = headers
        .get_all(X_FORWARDED_FOR)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .map(str::trim)
        .collect();
    for hop in hops.iter().rev().take(MAX_FORWARDED_HOPS) {
        let ip = parse_forwarded_hop(hop)?;
        if ip.is_unspecified() {
            return None;
        }
        if !trusted.iter().any(|net| net.contains(ip)) {
            return Some(ip);
        }
    }
    None
}

/// One `X-Forwarded-For` entry as an address.
///
/// Accepts the bare address every proxy in this deployment writes, plus the `addr:port` and
/// `[v6]:port` spellings some proxies emit, plus a bracketed IPv6 with no port. Anything else is
/// `None`, which stops the scan.
fn parse_forwarded_hop(hop: &str) -> Option<IpAddr> {
    if let Ok(ip) = hop.parse::<IpAddr>() {
        return Some(ip.to_canonical());
    }
    if let Ok(sock) = hop.parse::<SocketAddr>() {
        return Some(sock.ip().to_canonical());
    }
    hop.strip_prefix('[')?
        .strip_suffix(']')?
        .parse::<IpAddr>()
        .ok()
        .map(|ip| ip.to_canonical())
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

    // One notion of "who" for both tiers — see the module header's decision 2. Resolving once
    // here is what keeps them identical: two calls could not disagree even if the rules changed.
    let ip = client_ip(&req, &rl.trusted_proxies);

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

    // ───────────────────── T-625 — X-Forwarded-For, behind trusted proxies ─────────────────────

    fn nets(entries: &[&str]) -> Vec<ProxyNet> {
        entries
            .iter()
            .map(|e| ProxyNet::parse(e).unwrap_or_else(|why| panic!("{e:?}: {why}")))
            .collect()
    }

    /// A request as the router sees one: an optional `ConnectInfo` peer and zero or more
    /// `X-Forwarded-For` **header lines** (a chain may legally arrive split across several).
    fn req_from(peer: Option<&str>, forwarded: &[&str]) -> Request {
        let mut b = axum::http::Request::builder().uri("/api/v1/auth/refresh");
        for v in forwarded {
            b = b.header(X_FORWARDED_FOR, *v);
        }
        let mut req = b.body(axum::body::Body::empty()).expect("request");
        if let Some(peer) = peer {
            let ip: IpAddr = peer.parse().expect("peer address");
            req.extensions_mut()
                .insert(ConnectInfo(SocketAddr::new(ip, 51_000)));
        }
        req
    }

    fn resolved(peer: Option<&str>, forwarded: &[&str], trusted: &[&str]) -> Option<IpAddr> {
        client_ip(&req_from(peer, forwarded), &nets(trusted))
    }

    fn addr(s: &str) -> Option<IpAddr> {
        Some(s.parse().expect("address"))
    }

    /// **The default deployment.** With no trusted proxy configured the header is not read at
    /// all — behaviour identical to every wave before T-625.
    #[test]
    fn an_empty_trust_list_ignores_the_header_entirely() {
        assert_eq!(resolved(Some("203.0.113.9"), &[], &[]), addr("203.0.113.9"));
        assert_eq!(
            resolved(Some("203.0.113.9"), &["198.51.100.10"], &[]),
            addr("203.0.113.9"),
            "an unconfigured deployment must not read X-Forwarded-For"
        );
        // Including a header that claims to be the proxy itself.
        assert_eq!(
            resolved(Some("127.0.0.1"), &["198.51.100.10"], &[]),
            addr("127.0.0.1")
        );
    }

    /// **The spoof.** A client that connects directly cannot hand itself another identity, no
    /// matter what it sends — the peer is not a trusted proxy, so the header is not evidence.
    #[test]
    fn a_forged_header_from_an_untrusted_peer_is_ignored() {
        for forged in [
            "198.51.100.10",
            "127.0.0.1",
            "203.0.113.9, 198.51.100.10",
            "10.0.0.1",
        ] {
            assert_eq!(
                resolved(Some("203.0.113.9"), &[forged], &["127.0.0.1"]),
                addr("203.0.113.9"),
                "forged header {forged:?} from an untrusted peer changed the key"
            );
        }
    }

    /// **The rule that keeps this from being a vulnerability.** Caddy appends the address it
    /// observed, so the rightmost hop is the proxy's measurement and everything left of it is the
    /// client's text. A leftmost implementation returns `9.9.9.9` here — a fresh bucket per
    /// request, for free.
    #[test]
    fn the_rightmost_untrusted_hop_is_taken_not_the_leftmost() {
        assert_eq!(
            resolved(
                Some("127.0.0.1"),
                &["9.9.9.9, 198.51.100.10"],
                &["127.0.0.1"]
            ),
            addr("198.51.100.10")
        );
        // Varying the forged prefix cannot vary the answer.
        assert_eq!(
            resolved(
                Some("127.0.0.1"),
                &["203.0.113.7, 198.51.100.10"],
                &["127.0.0.1"]
            ),
            addr("198.51.100.10")
        );
    }

    /// Two proxies in front of the API: both are skipped, the client behind them is taken.
    #[test]
    fn trusted_hops_are_skipped_from_the_right() {
        assert_eq!(
            resolved(
                Some("127.0.0.1"),
                &["198.51.100.10, 10.1.1.1, 10.2.2.2"],
                &["127.0.0.1", "10.0.0.0/8"]
            ),
            addr("198.51.100.10")
        );
    }

    /// A chain split across several header lines is one chain, in order.
    #[test]
    fn multiple_header_lines_are_one_chain() {
        assert_eq!(
            resolved(
                Some("127.0.0.1"),
                &["9.9.9.9", "198.51.100.10"],
                &["127.0.0.1"]
            ),
            addr("198.51.100.10")
        );
    }

    /// Every chain that cannot be read honestly falls back to the peer — the shared bucket, i.e.
    /// the old behaviour. Fail-closed here is "limited together", never "not limited".
    #[test]
    fn an_unusable_chain_falls_back_to_the_peer() {
        let peer = addr("127.0.0.1");
        // No header at all.
        assert_eq!(resolved(Some("127.0.0.1"), &[], &["127.0.0.1"]), peer);
        for chain in [
            "",                         // empty value
            "unknown",                  // RFC 7239 token
            "_hidden",                  // obfuscated identifier
            "198.51.100.10, not-an-ip", // junk to the RIGHT of a real address: stop, do not
            "198.51.100.10, unknown",   // skip past it — skipping is the leftmost bug again
            "0.0.0.0",                  // not a source address
            "198.51.100.10, 0.0.0.0",   // …including at the end of a chain
            "127.0.0.1",                // nothing but trusted proxies
            "127.0.0.1, 127.0.0.1",     //
            "example.com",              // a name, not an address
        ] {
            assert_eq!(
                resolved(Some("127.0.0.1"), &[chain], &["127.0.0.1"]),
                peer,
                "chain {chain:?} should have fallen back to the peer"
            );
        }
    }

    /// A request with no socket has no client, header or not. Acquiring one from a header would
    /// hand every in-process caller a forgeable identity and file a durable bucket for it.
    #[test]
    fn a_peerless_request_has_no_client_even_with_a_header() {
        assert_eq!(resolved(None, &["198.51.100.10"], &["127.0.0.1"]), None);
        assert_eq!(resolved(None, &[], &[]), None);
        // The unspecified peer is the same case: synthesised in-process.
        assert_eq!(
            resolved(Some("0.0.0.0"), &["198.51.100.10"], &["127.0.0.1"]),
            None
        );
    }

    /// The spellings a proxy might actually write, all landing on the same client.
    #[test]
    fn hop_spellings_parse_to_canonical_addresses() {
        assert_eq!(parse_forwarded_hop("198.51.100.10"), addr("198.51.100.10"));
        assert_eq!(
            parse_forwarded_hop("198.51.100.10:41234"),
            addr("198.51.100.10")
        );
        assert_eq!(
            parse_forwarded_hop("::ffff:198.51.100.10"),
            addr("198.51.100.10"),
            "an IPv4-mapped hop is that IPv4 client, not a separate bucket"
        );
        assert_eq!(parse_forwarded_hop("2001:db8::1"), addr("2001:db8::1"));
        assert_eq!(parse_forwarded_hop("[2001:db8::1]"), addr("2001:db8::1"));
        assert_eq!(
            parse_forwarded_hop("[2001:db8::1]:443"),
            addr("2001:db8::1")
        );
        for junk in [
            "",
            "unknown",
            "_secret",
            "example.com",
            "1.2.3",
            "1.2.3.4.5",
        ] {
            assert_eq!(
                parse_forwarded_hop(junk),
                None,
                "{junk:?} is not an address"
            );
        }
    }

    /// A dual-stack listener's mapped peer is the same client as the plain one — one bucket, and
    /// `TRUSTED_PROXIES=127.0.0.1` still recognises the proxy.
    #[test]
    fn a_mapped_peer_is_canonicalised_before_anything_else() {
        assert_eq!(
            resolved(Some("::ffff:203.0.113.9"), &[], &[]),
            addr("203.0.113.9")
        );
        assert_eq!(
            resolved(Some("::ffff:127.0.0.1"), &["198.51.100.10"], &["127.0.0.1"]),
            addr("198.51.100.10"),
            "a mapped loopback peer is still the trusted proxy"
        );
    }

    /// The hop cap binds only on a chain of trusted proxies, and giving up costs the shared
    /// bucket rather than a bypass.
    #[test]
    fn an_absurdly_long_trusted_chain_falls_back_to_the_peer() {
        let mut chain = vec!["198.51.100.10".to_string()];
        chain.extend(std::iter::repeat_n(
            "10.0.0.2".to_string(),
            MAX_FORWARDED_HOPS + 1,
        ));
        let joined = chain.join(", ");
        assert_eq!(
            resolved(Some("10.0.0.1"), &[&joined], &["10.0.0.0/8"]),
            addr("10.0.0.1"),
            "a chain longer than the cap must fall back to the peer, not read past it"
        );
        // One hop under the cap, the client is still found.
        let short = std::iter::repeat_n("10.0.0.2".to_string(), MAX_FORWARDED_HOPS - 1)
            .collect::<Vec<_>>()
            .join(", ");
        let joined = format!("198.51.100.10, {short}");
        assert_eq!(
            resolved(Some("10.0.0.1"), &[&joined], &["10.0.0.0/8"]),
            addr("198.51.100.10")
        );
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
