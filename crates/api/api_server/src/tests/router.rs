//! Claims that only the assembled router can make: `/metrics` and `/healthz` answer each caller
//! the way the observability token entitles it to.

use std::time::Duration;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request as HttpRequest, StatusCode};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tower::ServiceExt as _;

use super::router;
use api_configuration::configuration::Config;

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
    router(crate::composition::application_state(pool, cfg))
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    observability_token: bool,
) -> (StatusCode, String) {
    let mut b = HttpRequest::builder().method(method).uri(uri);
    if observability_token {
        b = b.header("authorization", "Bearer test-observability-token");
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

/// Scraping is not public. `ObservabilityAuth` fails closed, so an API with no
/// `OBSERVABILITY_TOKEN` configured answers 401 rather than publishing its route table.
#[tokio::test]
async fn metrics_requires_the_observability_token() {
    let app = app_with(dead_pool());
    let (st, _) = call(&app, "GET", "/metrics", false).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);

    let mut cfg = Config::for_tests("postgres://unused", "s");
    cfg.observability_token = String::new();
    let unconfigured = router(crate::composition::application_state(dead_pool(), cfg));
    let (st, _) = call(&unconfigured, "GET", "/metrics", true).await;
    assert_eq!(
        st,
        StatusCode::UNAUTHORIZED,
        "empty OBSERVABILITY_TOKEN must fail closed"
    );
}

// ───────────────────── metrics and health: who sees what ─────────────────────

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

/// …and the same route with the observability token still serves the whole report, so the gating
/// is a relocation rather than a deletion. Without this, "discloses nothing" is satisfiable by a
/// handler that lost the detail entirely.
#[tokio::test]
async fn healthz_detail_is_served_to_the_observability_token() {
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
                .header("authorization", "Bearer not-the-token")
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
