//! The durable rate limiter, wired, proven through the real HTTP router.
//!
//! The `observability` module proves `PgRateLimiter` refuses at the limit at the library level, by
//! calling `check()` directly; that stays true even while no route calls `check()`. So every test
//! here goes through `api_server::router::router`, the router the `api-server` binary serves: a
//! strict route trips with a 429, a `Retry-After` and a spent durable bucket, and an unreachable
//! bucket store refuses rather than opening up.
//!
//! # ConnectInfo
//!
//! These requests carry a real `ConnectInfo` peer, because production does: the binary serves
//! with `into_make_service_with_connect_info::<SocketAddr>()`. A `oneshot` without it is a request
//! with no client, which [`api_http_layer::middleware::client_identity`]'s `client_ip` reports as
//! `None`.
//!
//! Each test owns a **distinct client IP**, so the buckets are independent and the tests run in
//! parallel with the binary's other modules.

use crate::common;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use api_configuration::configuration::Config;

use api_http_layer::middleware::durable_ratelimit::bucket_key;
use api_http_layer::middleware::{DURABLE_STRICT_BURST, DURABLE_STRICT_RPS, DURABLE_STRICT_SCOPE};
use api_server::router::router;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode, header};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt;

/// A strict-prefix route: rate-limited, and it needs no fixture rows to exercise. The handler's
/// own verdict (400 for a bodyless refresh) is irrelevant — what matters is 429 vs not-429.
const STRICT_ROUTE: &str = "/api/v1/auth/refresh";

async fn boot() -> Option<(PgPool, String)> {
    let url = common::require_test_database_url()?;
    let pool = api_database::connect(&url).await.expect("connect");
    api_database::migrate(&pool).await.expect("migrate");
    Some((pool, url))
}

fn router_for(pool: PgPool, url: &str) -> Router {
    router(api_server::composition::application_state(
        pool,
        Config::for_tests(url, "durable-secret"),
    ))
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
        "the wired limiter never refused — a limiter that cannot trip is exactly as inert as an \
         unwired one",
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

// ───────────────────────── ordinary clients are unaffected ─────────────────────────

// ───────────────────────── a request with no client ─────────────────────────

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
        .connect_lazy("postgres://unreachable:unreachable@127.0.0.1:1/no_such_db")
        .expect("lazy pool");
    let app = router_for(dead, "postgres://unused");
    let (st, retry, body) = call_from(&app, Ipv4Addr::new(10, 78, 4, 4), STRICT_ROUTE).await;
    assert_eq!(st, StatusCode::SERVICE_UNAVAILABLE, "body: {body}");
    assert_eq!(body, r#"{"error":"rate limiter unavailable"}"#);
    assert_eq!(retry.as_deref(), Some("1"));
}

// ───────────────────────── garbage collection is not an amnesty ─────────────────────────

// ───────────────────────── anti-drift pins ─────────────────────────
