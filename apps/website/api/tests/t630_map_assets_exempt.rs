//! T-630 — `/map-assets` is served **outside** the rate limiter, and nothing else is.
//!
//! # What went wrong
//!
//! T-578 put an in-memory L1 limiter on every route (global `20/s` burst `40`). T-627 replaced the
//! satellite's single whole-bundle `GET` with **49** HTTP Range requests at concurrency 4. Those
//! two shipped the same night and `/map-assets` was not exempt, so a legitimate Mission Creator
//! boot asked for 49 spans out of a 40-token bucket the DEM, the 951 world/density files and the
//! SPA's own assets had already been drawing on. Replaying the real plan with `curl` returned
//! **48× `429`, 1× `206`**; the client's fail-fast fetch discarded all 152,710,470 B on the first
//! refusal and the editor silently kept its 800×800 preview.
//!
//! T-629 taught the client to absorb `429`s with bounded backoff, which fixed the resolution and
//! left the tax: seconds of real backoff on every boot of the operator's own editor, protecting a
//! `ServeDir` that has no database, no session and no credential behind it.
//!
//! # What this file proves
//!
//! Both halves, and the second half is the one that matters:
//!
//! * a burst **far past** the global burst of 40 at `/map-assets/…` is served in full; and
//! * the same client, on the same router, is still refused on `/api/v1/auth/…` (strict tier), on
//!   an ordinary `/api/v1/…` read (global tier), and on `/uploads/…` (the *other* `ServeDir`).
//!
//! A limiter that cannot refuse anything would pass the first assertion and fail all three of the
//! others, which is exactly why they are here. `t578_ratelimit` and `t625_forwarded_for` remain the
//! proof that the limiter works at all; this file is the proof that T-630 narrowed it by one mount
//! and not by more.
//!
//! # ConnectInfo
//!
//! Requests carry a real `ConnectInfo` peer because production does (`bin/api.rs` serves with
//! `into_make_service_with_connect_info::<SocketAddr>()`). Each test owns a distinct client IP so
//! the buckets are independent and these can run in parallel like every other suite here.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use website_api::config::Config;
use website_api::middleware::{DURABLE_STRICT_BURST, RATE_LIMIT_EXEMPT_MOUNT, STRICT_PREFIXES};
use website_api::state::AppState;
use website_api::{app, db};

mod common;

/// A real, committed, non-LFS map asset: 804 bytes of JSON, present in every checkout.
///
/// Deliberately **not** the 152 MB satellite bundle. That file is git-LFS, so a checkout without
/// LFS content would turn this suite red for a reason that has nothing to do with rate limiting —
/// and the claim under test is about request *count*, not about which bytes come back. The real
/// 49-span plan against the real bundle is measured end to end with `curl` in the T-630 verify
/// notes; here the shape is 200 requests, which is 5× the global burst and 4× that plan.
const EXEMPT_ASSET: &str = "/map-assets/terrain-registry.json";

/// A strict-prefix route: both tiers, and it needs no fixture rows. The handler's own verdict
/// (400 for a bodyless refresh) is irrelevant — what matters is 429 vs not-429.
const STRICT_ROUTE: &str = "/api/v1/auth/refresh";
/// An ordinary global-tier API read.
const GLOBAL_ROUTE: &str = "/api/v1/announcements";
/// The *other* `ServeDir` in the router. The directory need not exist: a `429` short-circuits
/// before `ServeDir` is ever reached, so the interesting statuses here are 404 (allowed through)
/// and 429 (refused).
const LIMITED_STATIC: &str = "/uploads/t630-no-such-file.png";

/// Requests fired in each burst. 200 is 5× the global burst of 40, so a `/map-assets` burst that
/// survives it cannot be surviving on a bucket that merely happens to be deep.
const BURST: usize = 200;

/// The map-asset directory, resolved from the manifest rather than the process CWD.
///
/// `Config::map_assets_dir` empty makes `app::router` fall back to `../../../packages/map-assets`,
/// which is correct for the shipped binary and CWD-dependent for a test harness. Setting it
/// explicitly is the same code path a deployment with `MAP_ASSETS_DIR` set takes.
const MAP_ASSETS_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../packages/map-assets");

fn config_for(url: &str) -> Config {
    let mut cfg = Config::for_tests(url, "t630-secret");
    cfg.map_assets_dir = MAP_ASSETS_DIR.to_string();
    cfg
}

fn router_for(pool: PgPool, url: &str) -> Router {
    app::router(AppState::new(pool, config_for(url)))
}

/// A pool that never reaches a server and gives up fast.
///
/// Every assertion below except the `/api/v1` ones is about routes that touch no database at all,
/// so they run with this and need no `TEST_DATABASE_URL`. That is not a convenience: the exemption
/// must be guarded on a machine with no Postgres, because "the test skipped" and "the test passed"
/// look identical in a summary line.
fn dead_pool() -> PgPool {
    PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_millis(250))
        .connect_lazy("postgres://t630:t630@127.0.0.1:1/t630_no_such_db")
        .expect("lazy pool")
}

fn dead_router() -> Router {
    router_for(dead_pool(), "postgres://unused")
}

async fn boot() -> Option<(PgPool, String)> {
    let url = common::require_test_database_url()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some((pool, url))
}

/// One request from `ip`, with the `ConnectInfo` production always installs.
async fn call_from(
    app: &Router,
    ip: Ipv4Addr,
    method: &str,
    uri: &str,
    range: Option<&str>,
) -> (StatusCode, usize) {
    let mut b = Request::builder().method(method).uri(uri);
    if let Some(r) = range {
        b = b.header(header::RANGE, r);
    }
    let mut req = b
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{}"))
        .expect("request");
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::from((IpAddr::V4(ip), 51_000))));
    let resp = app.clone().oneshot(req).await.expect("router call");
    let status = resp.status();
    let body = to_bytes(resp.into_body(), 8 << 20).await.expect("body");
    (status, body.len())
}

async fn get(app: &Router, ip: Ipv4Addr, uri: &str) -> (StatusCode, usize) {
    call_from(app, ip, "GET", uri, None).await
}

/// Fire `BURST` requests at `uri` and return the status histogram, in first-seen order.
async fn burst(app: &Router, ip: Ipv4Addr, method: &str, uri: &str) -> Vec<(StatusCode, usize)> {
    let mut seen: Vec<(StatusCode, usize)> = Vec::new();
    for _ in 0..BURST {
        let (st, _) = call_from(app, ip, method, uri, None).await;
        match seen.iter_mut().find(|(s, _)| *s == st) {
            Some((_, n)) => *n += 1,
            None => seen.push((st, 1)),
        }
    }
    seen
}

fn count(hist: &[(StatusCode, usize)], want: StatusCode) -> usize {
    hist.iter().find(|(s, _)| *s == want).map_or(0, |(_, n)| *n)
}

// ───────────────────────── the headline: both halves, one client ─────────────────────────

/// **The T-630 proof.** One client, one router: 200 map-asset requests are all served, and that
/// same client is still refused on the strict tier and on the global tier.
///
/// Ordering is deliberate. The map-asset burst runs **first**, so if it were still limiter-bound it
/// would have drained the global bucket and the later `GLOBAL_ROUTE` burst would trip on request
/// one instead of at the burst — the failure would be loud in two places, not one. And running the
/// strict burst *after* 200 exempt requests is what shows the exemption did not leak into the
/// client's other buckets.
#[tokio::test]
async fn a_map_asset_burst_is_served_while_auth_and_the_global_tier_still_refuse() {
    let Some((pool, url)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset — a_map_asset_burst_is_served_while_auth…");
        return;
    };
    let ip = Ipv4Addr::new(10, 63, 1, 1);
    let app = router_for(pool, &url);

    // ── half one: 200 static map-asset requests, every one served ──
    let assets = burst(&app, ip, "GET", EXEMPT_ASSET).await;
    assert_eq!(
        count(&assets, StatusCode::OK),
        BURST,
        "{BURST} requests to {EXEMPT_ASSET} did not all return 200 — got {assets:?}. A 429 here is \
         the T-627/T-578 collision: a legitimate editor boot needs 951 distinct files from this \
         mount and cannot fit through a 40-token bucket."
    );

    // ── half two: the same client, still limited everywhere else ──
    let auth = burst(&app, ip, "POST", STRICT_ROUTE).await;
    let auth_429 = count(&auth, StatusCode::TOO_MANY_REQUESTS);
    assert!(
        auth_429 >= BURST - usize::try_from(DURABLE_STRICT_BURST).expect("burst fits") - 5,
        "the strict tier let {} of {BURST} requests through to {STRICT_ROUTE} — T-578's whole \
         point was that /auth/ needs durable limiting, and T-630 must not have touched it. Got \
         {auth:?}",
        BURST - auth_429
    );
    assert!(auth_429 > 0, "{STRICT_ROUTE} was never refused: {auth:?}");

    let global = burst(&app, ip, "GET", GLOBAL_ROUTE).await;
    assert!(
        count(&global, StatusCode::TOO_MANY_REQUESTS) > 0,
        "the global tier refused nothing in {BURST} requests to {GLOBAL_ROUTE} — the limiter has \
         been disarmed wholesale, not narrowed by one mount. Got {global:?}"
    );
}

// ───────────────────────── the exemption, with no database at all ─────────────────────────

/// The exemption holds on a machine with no Postgres.
///
/// `/map-assets` reaches neither limiter tier and no handler, so this needs nothing but a router.
/// It runs unconditionally, which matters: the headline test above skips when `TEST_DATABASE_URL`
/// is unset, and a suite whose only guard can skip is a suite that reports success over an input it
/// never examined.
#[tokio::test]
async fn the_exemption_holds_with_no_database_at_all() {
    let app = dead_router();
    let ip = Ipv4Addr::new(10, 63, 2, 2);

    let hist = burst(&app, ip, "GET", EXEMPT_ASSET).await;
    assert_eq!(
        count(&hist, StatusCode::OK),
        BURST,
        "{BURST} requests to {EXEMPT_ASSET} did not all return 200 — got {hist:?}"
    );

    // …and the bytes are real, not an empty 200 from some fallback that happens not to 429.
    let (st, len) = get(&app, ip, EXEMPT_ASSET).await;
    assert_eq!(st, StatusCode::OK);
    assert!(
        len > 100,
        "{EXEMPT_ASSET} served {len} bytes — this suite is asserting against something that is not \
         the committed map asset"
    );
}

/// The exempt mount answers **Range** requests, which is the shape T-627 actually uses.
///
/// A 200 to a plain `GET` would not prove the satellite path works: the loader sends
/// `Range: bytes=a-b` and treats anything that is not a `206` as a failure. Thirty of them here,
/// comfortably past the global burst, all `206`.
#[tokio::test]
async fn range_requests_over_the_exempt_mount_are_all_served() {
    let app = dead_router();
    let ip = Ipv4Addr::new(10, 63, 3, 3);

    for i in 0..60u64 {
        let start = i * 8;
        let spec = format!("bytes={start}-{}", start + 7);
        let (st, len) = call_from(&app, ip, "GET", EXEMPT_ASSET, Some(&spec)).await;
        assert_eq!(
            st,
            StatusCode::PARTIAL_CONTENT,
            "Range span #{i} ({spec}) returned {st}, not 206 — this is the exact request the \
             satellite loader makes 49 times per boot"
        );
        assert_eq!(len, 8, "span #{i} returned {len} bytes, expected 8");
    }
}

// ───────────────────────── the limiter can still refuse ─────────────────────────

/// **The other `ServeDir` is still limited.** T-630 exempted one named mount, not a category.
///
/// `/uploads` serves user-uploaded content — a different risk profile from terrain data that ships
/// in the repo — and it must keep the global tier. This also doubles as the no-database proof that
/// the global tier still refuses at all, since `ServeDir` touches no Postgres: the first ~40 are
/// allowed through to a 404 and the rest are refused.
#[tokio::test]
async fn the_other_static_mount_is_still_limited() {
    let app = dead_router();
    let ip = Ipv4Addr::new(10, 63, 4, 4);

    let hist = burst(&app, ip, "GET", LIMITED_STATIC).await;
    let refused = count(&hist, StatusCode::TOO_MANY_REQUESTS);
    assert!(
        refused > 0,
        "{BURST} requests to {LIMITED_STATIC} were never refused — the global tier is not limiting \
         static serving any more, which is more than T-630 asked for. Got {hist:?}"
    );
    assert!(
        count(&hist, StatusCode::NOT_FOUND) > 0,
        "nothing reached ServeDir at all: {hist:?}"
    );
}

/// **The SPA-serving deployment, which dev never exercises.**
///
/// `SPA_DIST_DIR` is unset under `cargo xtask mk leptos` + `cargo xtask mk rust-api`, so the branch that mounts the SPA
/// fallback and the COOP/COEP layers is dead on the operator's machine and would go untested by
/// everything else in this file. T-630 moved the map-asset mount across the rate-limit seam and had
/// to move those two header layers with it, so that branch is exactly the one place this slice
/// could have changed behaviour by accident. Two claims:
///
/// 1. the SPA fallback is still **rate-limited** — it is a document route, not a map asset, and it
///    was registered above the seam on purpose; and
/// 2. a map-asset response still carries both isolation headers, byte-for-byte what it carried
///    before the split.
///
/// The dist directory does not exist, so the fallback answers 404 — which is the point: a 404 means
/// the request reached `ServeDir`, and a 429 means the limiter stopped it first.
#[tokio::test]
async fn the_spa_deployment_keeps_its_fallback_limited_and_its_isolation_headers() {
    let mut cfg = config_for("postgres://unused");
    cfg.spa_dist_dir = "/nonexistent-t630-dist".to_string();
    let app = app::router(AppState::new(dead_pool(), cfg));
    let ip = Ipv4Addr::new(10, 63, 6, 6);

    // 1. the SPA fallback is a limited route.
    let hist = burst(&app, ip, "GET", "/t630-spa-route").await;
    assert!(
        count(&hist, StatusCode::TOO_MANY_REQUESTS) > 0,
        "the SPA fallback was never refused in {BURST} requests — moving the map-asset mount below \
         the rate-limit layer took the fallback with it. Got {hist:?}"
    );
    assert!(
        count(&hist, StatusCode::NOT_FOUND) > 0,
        "nothing reached the SPA ServeDir at all: {hist:?}"
    );

    // 2. …and the exempt mount still carries the isolation headers it carried pre-T-630.
    let fresh = Ipv4Addr::new(10, 63, 6, 7);
    let mut req = Request::builder()
        .method("GET")
        .uri(EXEMPT_ASSET)
        .body(Body::empty())
        .expect("request");
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::from((IpAddr::V4(fresh), 51_000))));
    let resp = app.oneshot(req).await.expect("router call");
    assert_eq!(resp.status(), StatusCode::OK);
    for (name, want) in [
        ("cross-origin-opener-policy", "same-origin"),
        ("cross-origin-embedder-policy", "credentialless"),
    ] {
        assert_eq!(
            resp.headers().get(name).and_then(|v| v.to_str().ok()),
            Some(want),
            "{EXEMPT_ASSET} lost {name} when the mount moved below the rate-limit layer — the \
             wasm SharedArrayBuffer path depends on these matching Trunk's"
        );
    }
}

/// A `429` from the limiter still carries the shipped envelope and `Retry-After`.
///
/// Narrowing which routes the limiter sees must not change what it says when it does refuse — the
/// SPA reads both, and T-629's backoff ladder is driven by the `Retry-After` value.
#[tokio::test]
async fn a_refusal_still_looks_exactly_as_it_did() {
    let app = dead_router();
    let ip = Ipv4Addr::new(10, 63, 5, 5);

    let mut refusal = None;
    for _ in 0..BURST {
        let mut req = Request::builder()
            .method("GET")
            .uri(LIMITED_STATIC)
            .body(Body::empty())
            .expect("request");
        req.extensions_mut()
            .insert(ConnectInfo(SocketAddr::from((IpAddr::V4(ip), 51_000))));
        let resp = app.clone().oneshot(req).await.expect("router call");
        if resp.status() == StatusCode::TOO_MANY_REQUESTS {
            let retry = resp
                .headers()
                .get(header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned);
            let body = to_bytes(resp.into_body(), 1 << 20).await.expect("body");
            refusal = Some((retry, String::from_utf8_lossy(&body).into_owned()));
            break;
        }
    }
    let (retry, body) = refusal.expect("the global tier refused nothing in a burst of 200");
    assert_eq!(retry.as_deref(), Some("1"), "429 must carry Retry-After");
    assert_eq!(body, r#"{"error":"rate limit exceeded"}"#);
}

// ───────────────────────── anti-drift pins ─────────────────────────

/// Class-R: T-630 changed **which routes** the limiter sees and nothing about the strict tier's
/// surface. If this ever needs updating, `/auth/` or `/ingest/` protection is being edited and that
/// is a different ticket.
#[test]
fn the_strict_surface_is_untouched() {
    assert_eq!(STRICT_PREFIXES, ["/api/v1/auth/", "/api/v1/ingest/"]);
    assert!(STRICT_PREFIXES.iter().any(|p| STRICT_ROUTE.starts_with(p)));
    assert!(!STRICT_PREFIXES.iter().any(|p| GLOBAL_ROUTE.starts_with(p)));
    assert!(
        !STRICT_PREFIXES
            .iter()
            .any(|p| RATE_LIMIT_EXEMPT_MOUNT.starts_with(p)),
        "the exempt mount overlaps a strict prefix — the exemption would be hiding an auth route"
    );
}

/// Class-R: this suite asserts against the mount the router actually registers.
#[test]
fn the_asset_under_test_is_under_the_exempt_mount() {
    assert_eq!(RATE_LIMIT_EXEMPT_MOUNT, "/map-assets");
    assert!(
        EXEMPT_ASSET.starts_with(&format!("{RATE_LIMIT_EXEMPT_MOUNT}/")),
        "{EXEMPT_ASSET} is not under {RATE_LIMIT_EXEMPT_MOUNT} — this suite would be measuring \
         some other route's limiter behaviour"
    );
    assert!(
        std::path::Path::new(MAP_ASSETS_DIR)
            .join("terrain-registry.json")
            .is_file(),
        "the committed map asset this suite serves is missing from {MAP_ASSETS_DIR} — every burst \
         assertion below would be measuring 404s"
    );
}
