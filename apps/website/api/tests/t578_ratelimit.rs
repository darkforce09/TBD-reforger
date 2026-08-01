//! T-578 — the durable rate limiter, **wired**, proven through the real HTTP router.
//!
//! # What T-280 proved, and why it was not enough
//!
//! `tests/t280_observability.rs` proves `PgRateLimiter` refuses at the limit and still refuses
//! after a restart — at the **library** level, by calling `check()` directly. Every one of those
//! assertions stayed true while the API had no working rate limiting at all, because nothing
//! called `check()`. That is the signature defect in its production form: the code exists, the
//! protection does not.
//!
//! So every test here goes through `app::router` — the same router `bin/api.rs` serves — and the
//! central one is [`refusal_survives_a_restart`]: spend the bucket on one router, build a second
//! router with a **fresh in-memory limiter** over the same database, and require the very next
//! request to be refused. A fresh `IpLimiter` cannot refuse a first request, so nothing but the
//! durable tier can produce that 429. Under the pre-T-578 tree that request is a 400 from the
//! handler, which is exactly the RED this file is for.
//!
//! # ConnectInfo
//!
//! These requests carry a real `ConnectInfo` peer, because production does: `bin/api.rs` serves
//! with `into_make_service_with_connect_info::<SocketAddr>()`. A `oneshot` without it is a request
//! with no client, which [`middleware::ratelimit`]'s `client_ip` reports as `None` — see
//! [`api_binary_still_installs_connect_info`], which pins the binary so that path cannot become
//! production's.
//!
//! Each test owns a **distinct client IP**, so the buckets are independent and the tests can run
//! in parallel inside this binary the way every other suite does.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;
use website_api::app::durable_ratelimit::{RATE_LIMIT_BUCKETS_DDL, bucket_key};
use website_api::config::Config;
use website_api::middleware::{
    DURABLE_STRICT_BURST, DURABLE_STRICT_RPS, DURABLE_STRICT_SCOPE, STRICT_PREFIXES,
};
use website_api::services::{RATE_LIMIT_BUCKET_TTL, start_rate_limit_prune};
use website_api::state::AppState;
use website_api::{app, db};

mod common;

/// A strict-prefix route: rate-limited, and it needs no fixture rows to exercise. The handler's
/// own verdict (400 for a bodyless refresh) is irrelevant — what matters is 429 vs not-429.
const STRICT_ROUTE: &str = "/api/v1/auth/refresh";
/// A global-scope route, outside the durable tier's surface.
const GLOBAL_ROUTE: &str = "/api/v1/announcements";

async fn boot() -> Option<(PgPool, String)> {
    let url = common::require_test_database_url()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some((pool, url))
}

fn router_for(pool: PgPool, url: &str) -> Router {
    app::router(AppState::new(pool, Config::for_tests(url, "t578-secret")))
}

/// One request from `ip`, with the `ConnectInfo` production always installs.
async fn call_from(app: &Router, ip: Ipv4Addr, uri: &str) -> (StatusCode, Option<String>, String) {
    let mut req = Request::builder()
        .method(if uri == STRICT_ROUTE { "POST" } else { "GET" })
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{}"))
        .expect("request");
    req.extensions_mut()
        .insert(ConnectInfo(SocketAddr::from((IpAddr::V4(ip), 51_000))));
    let resp = app.clone().oneshot(req).await.expect("router call");
    let status = resp.status();
    let retry_after = resp
        .headers()
        .get(header::RETRY_AFTER)
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let body = to_bytes(resp.into_body(), 1 << 20).await.expect("body");
    (
        status,
        retry_after,
        String::from_utf8_lossy(&body).into_owned(),
    )
}

/// `tokens` remaining in the durable bucket for `ip`, or `None` when no row exists.
async fn bucket_tokens(pool: &PgPool, ip: Ipv4Addr) -> Option<f64> {
    sqlx::query_scalar::<_, f64>("SELECT tokens FROM rate_limit_buckets WHERE bucket_key = $1")
        .bind(bucket_key(DURABLE_STRICT_SCOPE, IpAddr::V4(ip)))
        .fetch_optional(pool)
        .await
        .expect("read bucket")
}

// ───────────────────────── the limiter actually refuses ─────────────────────────

/// Drive the wired router past the durable burst and require a `429` **with `Retry-After`**, an
/// unchanged error envelope, and a bucket row that records the spend.
///
/// The database assertion is the one that says *which* tier refused: a 429 alone is also what the
/// in-memory limiter produces, but only the durable tier leaves `rate_limit_buckets` at zero.
#[tokio::test]
async fn strict_route_trips_with_429_retry_after_and_a_spent_bucket() {
    let Some((pool, url)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset — strict_route_trips_…");
        return;
    };
    let ip = Ipv4Addr::new(10, 78, 1, 1);
    let app = router_for(pool.clone(), &url);

    let mut refused = None;
    for i in 1..=(DURABLE_STRICT_BURST + 4) {
        let (st, retry, body) = call_from(&app, ip, STRICT_ROUTE).await;
        if st == StatusCode::TOO_MANY_REQUESTS {
            refused = Some((i, retry, body));
            break;
        }
    }
    let (at, retry, body) = refused.expect(
        "the wired limiter never refused — a limiter that cannot trip is exactly as inert as the \
         unwired one T-578 was filed against",
    );
    assert!(
        at > DURABLE_STRICT_BURST,
        "refused at request {at}, before the burst of {DURABLE_STRICT_BURST} was spent"
    );
    assert_eq!(
        retry.as_deref(),
        Some("1"),
        "429 must carry Retry-After; the strict tier refills {DURABLE_STRICT_RPS}/s"
    );
    assert_eq!(
        body, r#"{"error":"rate limit exceeded"}"#,
        "the shipped error envelope must not change under a 429"
    );

    // Database state: the durable bucket exists for this client and is spent.
    let tokens = bucket_tokens(&pool, ip)
        .await
        .expect("the durable limiter must have written a bucket row for this client");
    assert!(
        tokens < 1.0,
        "bucket for {ip} holds {tokens} tokens — a refusal means it was under one"
    );
}

/// **The wiring proof.** Spend the bucket on one router; hand the *next* request to a second
/// router whose in-memory limiter has never seen this client.
///
/// A fresh `IpLimiter` (burst 10) cannot refuse a first request, so a 429 here can only have come
/// from Postgres. This is the perturbation that separates "the durable limiter exists" from "the
/// durable limiter is consulted" — pre-T-578 it is a 400 from the refresh handler.
#[tokio::test]
async fn refusal_survives_a_restart() {
    let Some((pool, url)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset — refusal_survives_a_restart");
        return;
    };
    let ip = Ipv4Addr::new(10, 78, 2, 2);

    // Process 1: spend the whole durable burst.
    let before = router_for(pool.clone(), &url);
    for _ in 0..DURABLE_STRICT_BURST {
        call_from(&before, ip, STRICT_ROUTE).await;
    }
    let spent = bucket_tokens(&pool, ip).await.expect("bucket row");
    assert!(spent < 1.0, "burst not actually spent: {spent} tokens left");
    drop(before);

    // Process 2: brand-new AppState, brand-new in-memory limiter, brand-new pool. Only the
    // database is shared — which is the entire claim behind the word "durable".
    let restarted_pool = PgPoolOptions::new()
        .max_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&url)
        .await
        .expect("second pool");
    let after = router_for(restarted_pool, &url);
    let (st, retry, body) = call_from(&after, ip, STRICT_ROUTE).await;
    assert_eq!(
        st,
        StatusCode::TOO_MANY_REQUESTS,
        "a restart handed the client a fresh bucket — the durable limiter is not wired \
         (got {st}, body {body})"
    );
    assert_eq!(retry.as_deref(), Some("1"));
}

// ───────────────────────── ordinary clients are unaffected ─────────────────────────

/// An ordinary client is never refused and never even reaches Postgres on the routes it spends
/// its session in.
///
/// Both halves matter. "Not 429" alone would also hold if the limiter throttled nothing; the
/// `rate_limit_buckets` assertion is what proves the *cost* decision — the SPA's read traffic and
/// the Mission Creator's `/map-assets` fetches do not carry a database write.
#[tokio::test]
async fn an_ordinary_client_is_not_throttled_and_writes_no_bucket() {
    let Some((pool, url)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset — an_ordinary_client_is_not_throttled…");
        return;
    };
    let ip = Ipv4Addr::new(10, 78, 3, 3);
    let app = router_for(pool.clone(), &url);

    // A page load's worth of global-scope reads — comfortably more than the strict burst, to
    // show the strict policy is not being applied to them.
    for i in 0..(DURABLE_STRICT_BURST * 2) {
        let (st, _, body) = call_from(&app, ip, GLOBAL_ROUTE).await;
        assert_ne!(
            st,
            StatusCode::TOO_MANY_REQUESTS,
            "ordinary read #{i} was throttled: {body}"
        );
        assert_ne!(st, StatusCode::SERVICE_UNAVAILABLE, "read #{i}: {body}");
    }
    assert_eq!(
        bucket_tokens(&pool, ip).await,
        None,
        "a global-scope route must not reach the durable limiter — that is one database write \
         per request on the SPA's hot path"
    );

    // …and a *normal* amount of auth traffic (a login plus a couple of refreshes) passes too,
    // while leaving the bucket nearly full.
    let auth_ip = Ipv4Addr::new(10, 78, 3, 4);
    for i in 0..3 {
        let (st, _, body) = call_from(&app, auth_ip, STRICT_ROUTE).await;
        assert_ne!(
            st,
            StatusCode::TOO_MANY_REQUESTS,
            "ordinary auth request #{i} was throttled: {body}"
        );
    }
    let tokens = bucket_tokens(&pool, auth_ip)
        .await
        .expect("auth traffic does reach the durable limiter");
    assert!(
        tokens > f64::from(DURABLE_STRICT_BURST) - 4.0,
        "three requests should leave ~{} tokens, found {tokens}",
        DURABLE_STRICT_BURST - 3
    );
}

// ───────────────────────── a request with no client ─────────────────────────

/// A request that arrived over no socket is skipped by the **durable** tier — and is still
/// limited by the in-memory one.
///
/// This is the other half of `client_ip`'s contract, and it exists so the skip cannot be read as
/// "unlimited". `0.0.0.0` is not a source address a packet can carry, so such a request has no
/// client to attribute a durable bucket to; what it does *not* get is a free pass. Production
/// never takes this path — `bin/api.rs` installs `ConnectInfo` on every accepted connection, which
/// [`api_binary_still_installs_connect_info`] pins.
#[tokio::test]
async fn a_request_with_no_peer_is_still_limited_in_memory_and_writes_no_bucket() {
    let Some((pool, url)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset — a_request_with_no_peer…");
        return;
    };
    let app = router_for(pool.clone(), &url);

    let mut refused = false;
    for _ in 0..(DURABLE_STRICT_BURST + 5) {
        // No `ConnectInfo` — a `oneshot` exactly as the other suites in this crate build one.
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(STRICT_ROUTE)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from("{}"))
                    .expect("request"),
            )
            .await
            .expect("router call");
        if resp.status() == StatusCode::TOO_MANY_REQUESTS {
            refused = true;
            break;
        }
    }
    assert!(
        refused,
        "a peerless request is exempt from the durable tier, NOT from rate limiting — the \
         in-memory limiter must still refuse it"
    );
    let unspecified: Option<f64> =
        sqlx::query_scalar("SELECT tokens FROM rate_limit_buckets WHERE bucket_key = $1")
            .bind(bucket_key(
                DURABLE_STRICT_SCOPE,
                IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            ))
            .fetch_optional(&pool)
            .await
            .expect("read bucket");
    assert_eq!(
        unspecified, None,
        "a durable bucket was filed under the unspecified address — that is one persistent row \
         for a client that does not exist, shared by every in-process caller"
    );
}

// ───────────────────────── fail closed ─────────────────────────

/// With the store unreachable the durable tier returns **503**, never "allowed".
///
/// Needs no database: the whole point is that there isn't one. A limiter that opens up when its
/// backing store is gone is the same defect class as one that was never wired.
#[tokio::test]
async fn an_unreachable_store_refuses_rather_than_opening_up() {
    // Ephemeral port, nothing listening, short acquire budget so this is fast rather than 30 s.
    let dead = PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_millis(250))
        .connect_lazy("postgres://t578:t578@127.0.0.1:1/t578_no_such_db")
        .expect("lazy pool");
    let app = router_for(dead, "postgres://unused");
    let (st, retry, body) = call_from(&app, Ipv4Addr::new(10, 78, 4, 4), STRICT_ROUTE).await;
    assert_eq!(st, StatusCode::SERVICE_UNAVAILABLE, "body: {body}");
    assert_eq!(body, r#"{"error":"rate limiter unavailable"}"#);
    assert_eq!(retry.as_deref(), Some("1"));
}

// ───────────────────────── garbage collection is not an amnesty ─────────────────────────

/// A sweep at the production TTL does not release a bucket that was just spent.
///
/// The general claim — that deleting an idle bucket can never grant quota — is arithmetic and is
/// asserted as such below: the TTL is long enough that any row old enough to be swept has already
/// refilled to capacity, so an absent bucket and a full bucket are the same thing.
#[tokio::test]
async fn pruning_at_the_production_ttl_does_not_release_a_spent_bucket() {
    let Some((pool, url)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset — pruning_at_the_production_ttl…");
        return;
    };
    let ip = Ipv4Addr::new(10, 78, 5, 5);
    let app = router_for(pool.clone(), &url);
    for _ in 0..(DURABLE_STRICT_BURST + 2) {
        call_from(&app, ip, STRICT_ROUTE).await;
    }
    assert!(bucket_tokens(&pool, ip).await.expect("bucket row") < 1.0);

    // One sweep at the production TTL/interval, then stop the task.
    let handle = start_rate_limit_prune(
        pool.clone(),
        RATE_LIMIT_BUCKET_TTL,
        Duration::from_secs(3600),
    );
    tokio::time::sleep(Duration::from_millis(300)).await;
    handle.abort();

    assert!(
        bucket_tokens(&pool, ip).await.is_some_and(|t| t < 1.0),
        "the sweeper removed a live bucket — that is an amnesty, not garbage collection"
    );
    let (st, _, body) = call_from(&app, ip, STRICT_ROUTE).await;
    assert_eq!(st, StatusCode::TOO_MANY_REQUESTS, "body: {body}");
}

/// Class-R: the TTL is long enough that a swept bucket had already refilled to capacity, which is
/// what makes [`start_rate_limit_prune`] reclamation rather than a grant of quota.
#[test]
fn prune_ttl_is_longer_than_a_full_refill() {
    let refill_secs = f64::from(DURABLE_STRICT_BURST) / f64::from(DURABLE_STRICT_RPS);
    assert!(
        RATE_LIMIT_BUCKET_TTL.as_secs_f64() > refill_secs * 10.0,
        "TTL {}s is not comfortably longer than the {refill_secs}s a bucket takes to refill — \
         sweeping would start returning quota early",
        RATE_LIMIT_BUCKET_TTL.as_secs_f64()
    );
}

// ───────────────────────── anti-drift pins ─────────────────────────

/// The migration is `RATE_LIMIT_BUCKETS_DDL`, verbatim.
///
/// T-280 made the DDL a `const` precisely so "the bytes the tests prove and the bytes the
/// migration lands" could not diverge. That guarantee is worth nothing unless something reads both
/// and compares them, which is what this does.
#[test]
fn migration_0020_is_the_ddl_constant_verbatim() {
    let sql = include_str!("../migrations/0020_rate_limit_buckets.sql");
    assert!(
        sql.contains(RATE_LIMIT_BUCKETS_DDL),
        "migrations/0020_rate_limit_buckets.sql no longer contains RATE_LIMIT_BUCKETS_DDL \
         verbatim — the limiter would bind a shape the migration did not land.\n\
         --- const ---\n{RATE_LIMIT_BUCKETS_DDL}\n--- file ---\n{sql}"
    );
}

/// Class-R: the served binary still installs `ConnectInfo`.
///
/// `client_ip` reports "no client" when the extension is absent, and a request with no client is
/// not attributed to a durable bucket. That is correct for an in-process caller and catastrophic
/// if it ever describes production traffic — the durable tier would silently stop being consulted
/// while every test in this file still passed. This is the tripwire for that.
#[test]
fn api_binary_still_installs_connect_info() {
    let src = include_str!("../src/bin/api.rs");
    assert!(
        src.contains("into_make_service_with_connect_info::<SocketAddr>()"),
        "src/bin/api.rs no longer serves with into_make_service_with_connect_info::<SocketAddr>() \
         — without it every production request has no peer address, and both rate-limit tiers \
         stop distinguishing clients"
    );
    // …and the prune tick the bucket table depends on is still armed there.
    assert!(
        src.contains("start_rate_limit_prune"),
        "src/bin/api.rs no longer arms the rate-limit bucket sweeper — rate_limit_buckets grows \
         without bound"
    );
}

/// Class-R: the durable tier's surface is the strict tier's, stated once.
#[test]
fn durable_surface_is_the_strict_prefixes() {
    assert_eq!(STRICT_PREFIXES, ["/api/v1/auth/", "/api/v1/ingest/"]);
    assert!(STRICT_PREFIXES.iter().any(|p| STRICT_ROUTE.starts_with(p)));
    assert!(!STRICT_PREFIXES.iter().any(|p| GLOBAL_ROUTE.starts_with(p)));
}
