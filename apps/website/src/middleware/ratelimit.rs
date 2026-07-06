//! Per-IP rate limiting — Rust port of `ratelimit.go`. Global limiter, switching to
//! the strict limiter for `/api/v1/auth/` + `/api/v1/ingest/` prefixes. In-memory,
//! single-instance (same caveat as the Go original). Behavioral parity only — exact
//! refill timing is a documented non-bit-exact surface.

use std::net::{IpAddr, SocketAddr};
use std::num::NonZeroU32;

use axum::extract::{ConnectInfo, Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use governor::clock::DefaultClock;
use governor::state::keyed::DefaultKeyedStateStore;
use governor::{Quota, RateLimiter};

use crate::middleware::json_error;
use crate::state::AppState;

/// Full rooted-path prefixes that get the strict limiter (HasPrefix, not substring).
const STRICT_PREFIXES: [&str; 2] = ["/api/v1/auth/", "/api/v1/ingest/"];

/// Per-client-IP token bucket keyed by IP.
pub struct IpLimiter {
    inner: RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>,
}

impl IpLimiter {
    /// `per_second` sustained rate with the given `burst` bucket size.
    pub fn new(per_second: u32, burst: u32) -> Self {
        let quota = Quota::per_second(NonZeroU32::new(per_second).expect("per_second > 0"))
            .allow_burst(NonZeroU32::new(burst).expect("burst > 0"));
        Self {
            inner: RateLimiter::keyed(quota),
        }
    }

    /// True if the request for `ip` is allowed (a token was available).
    pub fn check(&self, ip: IpAddr) -> bool {
        self.inner.check_key(&ip).is_ok()
    }
}

pub async fn rate_limit(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let strict = STRICT_PREFIXES
        .iter()
        .any(|p| req.uri().path().starts_with(p));
    let limiter = if strict {
        &state.rl_strict
    } else {
        &state.rl_global
    };

    // Trust-none: the client IP is the direct connection peer (ConnectInfo).
    let ip = req
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ci| ci.0.ip())
        .unwrap_or(IpAddr::from([0, 0, 0, 0]));

    if !limiter.check(ip) {
        return json_error(StatusCode::TOO_MANY_REQUESTS, "rate limit exceeded").into_response();
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
}
