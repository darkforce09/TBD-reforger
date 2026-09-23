//! Unit coverage for the two limiter tiers, the strict-prefix predicate, the derived
//! `Retry-After`, and the router-ordering facts the exempt mount depends on.

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
    assert!(strict("/api/v1/ingest/match-results"));
    // Machine-credential game-runtime traffic stays on the global tier.
    assert!(!strict("/api/v1/game-runtime/sessions"));
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

// ───────────────────── the exempt static mount ─────────────────────

/// The mount is the path the editor actually asks for.
///
/// `apps/website/frontend/src/world_assets/mod.rs` builds every map-asset URL as
/// `format!("/map-assets/{terrain}")` and `world_host.rs` hard-codes
/// `/map-assets/glyphs/atlas/…`. If this constant drifts from that literal the exemption stops
/// covering the traffic it was written for and a cold editor boot starts paying backoff again —
/// silently, because everything still *works*, just slowly. That is the failure mode this pin
/// exists for.
#[test]
fn the_exempt_mount_is_the_path_the_editor_requests() {
    assert_eq!(RATE_LIMIT_EXEMPT_MOUNT, "/map-assets");
}

/// **The seam.** `Router::layer` wraps the routes registered before it and nothing after, so
/// the entire exemption is one ordering fact in `core::http_router::router`: the `/map-assets`
/// `nest_service` comes *below* the `rate_limit` layer.
///
/// `tests/map_assets_rate_limit_exemption.rs` proves the behaviour through real HTTP, which is the
/// primary guard. This is the diagnostic one: a tidy-up that moves the mount back alongside the
/// other static mounts re-arms the defect with no compile error and no obvious symptom beyond a
/// slower editor boot, and this says so by name instead of leaving a burst test to fail
/// cryptically.
///
/// Both markers are required to exist exactly once — a pin that passes because it could not
/// find what it was checking is not checking anything.
#[test]
fn the_exempt_mount_is_registered_below_the_rate_limit_layer() {
    const LAYER: &str = "RateLimitState::new(state.clone())";
    const MOUNT: &str = "RATE_LIMIT_EXEMPT_MOUNT";
    let src = include_str!("../../http_router.rs");

    assert_eq!(
        src.matches(LAYER).count(),
        1,
        "src/core/http_router.rs no longer contains exactly one `{LAYER}` — this pin cannot locate the \
         rate-limit layer, so it is not checking anything"
    );
    assert_eq!(
        src.matches(MOUNT).count(),
        1,
        "src/core/http_router.rs no longer contains exactly one `{MOUNT}` — this pin cannot locate the \
         exempt mount, so it is not checking anything"
    );

    let layer_at = src.find(LAYER).expect("counted above");
    let mount_at = src.find(MOUNT).expect("counted above");
    assert!(
        mount_at > layer_at,
        "src/core/http_router.rs mounts {RATE_LIMIT_EXEMPT_MOUNT} at byte {mount_at}, ABOVE the rate-limit \
         layer at byte {layer_at}. `Router::layer` wraps everything registered before it, so \
         the map-asset ServeDir is back inside the limiter and a Mission Creator boot is \
         paying backoff again. Move the `nest_service` back below the layer."
    );
}

/// The glyph mount carries the same exemption and needs the same pin.
///
/// A map client asks for the atlas on the same cold boot that asks for 951 terrain files, so a
/// glyph mount left above the layer re-opens the defect for a smaller set of requests — small
/// enough to look like an unrelated intermittent failure rather than a throttle.
#[test]
fn the_glyph_mount_is_also_registered_below_the_rate_limit_layer() {
    const LAYER: &str = "RateLimitState::new(state.clone())";
    const MOUNT: &str = "RATE_LIMIT_EXEMPT_GLYPH_MOUNT";
    let src = include_str!("../../http_router.rs");

    assert_eq!(
        src.matches(MOUNT).count(),
        1,
        "src/core/http_router.rs no longer contains exactly one `{MOUNT}` — this pin cannot locate the \
         glyph mount, so it is not checking anything"
    );

    let layer_at = src.find(LAYER).expect("the rate-limit layer");
    let mount_at = src.find(MOUNT).expect("counted above");
    assert!(
        mount_at > layer_at,
        "src/core/http_router.rs mounts {RATE_LIMIT_EXEMPT_GLYPH_MOUNT} ABOVE the rate-limit layer"
    );
}

/// The glyph mount must stay a strict sub-path of the terrain mount.
///
/// The two directories are joined at the router, not on disk, and that join only reaches the
/// client if the glyph prefix still sits under the prefix the map client builds its URLs from.
#[test]
fn the_glyph_mount_is_nested_under_the_map_asset_mount() {
    assert!(
        RATE_LIMIT_EXEMPT_GLYPH_MOUNT.starts_with(&format!("{RATE_LIMIT_EXEMPT_MOUNT}/")),
        "{RATE_LIMIT_EXEMPT_GLYPH_MOUNT} is not under {RATE_LIMIT_EXEMPT_MOUNT}"
    );
}

/// The exemption is one named mount, not a category.
///
/// `/uploads` is the other `ServeDir` in the router and it is **not** exempt: it serves
/// user-uploaded content, which is a different risk profile from terrain data shipped in the
/// repo. Nothing here generalises to "static files are unlimited", and this states that so the
/// next reader does not generalise it for us. The behavioural half is
/// `map_assets_rate_limit_exemption::the_other_static_mount_is_still_limited`.
#[test]
fn the_exemption_does_not_cover_the_other_static_mount() {
    assert_ne!(RATE_LIMIT_EXEMPT_MOUNT, "/uploads");
    let src = include_str!("../../http_router.rs");
    assert!(
        src.contains(r#"nest_service("/uploads", ServeDir::new(uploads_dir))"#),
        "src/core/http_router.rs no longer mounts /uploads the way this test assumes — re-check that it is \
         still registered ABOVE the rate-limit layer"
    );
    let uploads_at = src
        .find(r#"nest_service("/uploads""#)
        .expect("asserted above");
    let layer_at = src
        .find("RateLimitState::new(state.clone())")
        .expect("the rate-limit layer");
    assert!(
        uploads_at < layer_at,
        "/uploads drifted below the rate-limit layer and is now unlimited too — the exemption covers \
         one mount, not every ServeDir"
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
    let src = include_str!("../../application_state.rs");
    assert!(
        src.contains(&format!(
            "IpLimiter::new({DURABLE_STRICT_RPS}, {DURABLE_STRICT_BURST})"
        )),
        "application_state.rs no longer builds rl_strict as IpLimiter::new({DURABLE_STRICT_RPS}, \
         {DURABLE_STRICT_BURST}) — retune the durable tier with it"
    );
}
