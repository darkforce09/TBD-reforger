//! Unit coverage for the two limiter tiers, the strict-prefix predicate and the derived
//! `Retry-After`.

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
    // Machine-credential game-runtime and ingest traffic stays on the global tier.
    assert!(!strict("/api/v1/game-runtime/sessions"));
    assert!(!strict("/api/v1/ingest/match-results"));
    // Global paths use the global bucket.
    assert!(!strict("/api/v1/announcements"));
    assert!(!strict("/api/v1/missions"));
    // "auth" as a substring (e.g. /oauth/) is NOT the rooted /auth/ prefix.
    assert!(!strict("/api/v1/oauth/authorize"));
}

/// The durable tier's surface is exactly the strict tier's, and the routes the SPA leans on are
/// outside it. A change that quietly widened `STRICT_PREFIXES` to `/api/v1/` would put a database
/// write on every editor tile fetch.
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
    assert!(!durable("/api/v1/ingest/match-events"));
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

// ───────────────────── the exempt static mount ─────────────────────
