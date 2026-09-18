//! Claims that only the assembled router can make: that every metrics series moves with real
//! traffic through the real layer chain, and that `/metrics` and `/healthz` answer the way the
//! chain positions them to.

use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request as HttpRequest, StatusCode};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt as _;

use super::router;
use crate::core::application_state::AppState;
use crate::core::configuration::Config;
use crate::core::observability::metrics_registry::{LATENCY_BUCKETS_S, UNMATCHED_ROUTE};

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
        .connect_lazy("postgres://probe:probe@127.0.0.1:1/no_such_db")
        .expect("lazy pool")
}

fn app_with(pool: PgPool) -> Router {
    let cfg = Config::for_tests("postgres://unused", "router-test-secret");
    router(AppState::new(pool, cfg))
}

async fn call(app: &Router, method: &str, uri: &str, service_token: bool) -> (StatusCode, String) {
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
    for ub in LATENCY_BUCKETS_S {
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
/// Reads the **detailed** payload (`checks` sits behind `X-Service-Token`), because the claim
/// under test is that each check can go red independently, and that is only visible per-check.
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

/// The public probe discloses **only** `status`, and the 200/503 split survives.
///
/// The detail an unauthenticated caller must not see is
/// `version=0.1.0  uptime=396  pool={connections:5, idle:4}  migrations.applied=18`. Every one of
/// those is asserted absent here — by key AND by value, because a handler that renamed `version`
/// to `build` would satisfy a key-only check while disclosing exactly the same thing.
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

/// …and the same route with the service token still serves the whole report, so the gating is a
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

// ───────────────────── domain route tables: one router, no collisions ─────────────────────

/// Merging the eight domain route tables must produce a router, not a panic.
///
/// `Router::merge` panics at BUILD time when two tables register the same method on the same
/// path, so this claim cannot be made by reading the tables: it needs the merge to run. Both
/// `dev` arms are built because the development-only `/auth/dev-login` registration exists in
/// only one of them, and a collision it introduced would otherwise never be exercised.
///
/// Assembling the full application on top covers the same merge plus the per-route
/// `DefaultBodyLimit` layers and the global middleware chain. The pool never reaches a server,
/// but `connect_lazy` still needs a runtime, which is why this is a `tokio::test`.
#[tokio::test]
async fn the_eight_domain_route_tables_merge_into_one_router() {
    for dev in [false, true] {
        let _: Router<AppState> = super::api_v1_routes(dev, 1 << 20);
    }
    let _: Router = app_with(dead_pool());
}
