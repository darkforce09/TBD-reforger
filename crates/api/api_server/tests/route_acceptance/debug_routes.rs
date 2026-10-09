//! The equipment data viewer's debug reads exist only in a development router.
//!
//! **Role:** proves every `GET /api/v1/debug/equipment-data/*` route answers 404 from a router
//! booted with a production configuration and reaches its handler (anything but 404) from a
//! router booted with a development configuration.
//! **Position:** boots `api_server::router::router` over this binary's test database, once per
//! configuration, and drives it with `oneshot` requests.
//! **Signals & state:** none beyond the per-binary database both routers share; each request
//! carries a fresh synthetic peer so the rate limiter never answers in the handler's place.
//! **Invariants:** every path of [`DEBUG_EQUIPMENT_PATHS`] answers 404 under production
//! configuration and reaches its handler under development configuration. A missing
//! `TEST_DATABASE_URL` panics; no case passes without its database.

use crate::common;

use std::net::{IpAddr, SocketAddr};
use std::sync::atomic::{AtomicU32, Ordering};

use api_configuration::configuration::Config;

use api_server::router::router;
use axum::Router;
use axum::body::Body;
use axum::extract::ConnectInfo;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

/// Every debug read the equipment data viewer serves, as its full path.
const DEBUG_EQUIPMENT_PATHS: [&str; 12] = [
    "/api/v1/debug/equipment-data/containers",
    "/api/v1/debug/equipment-data/documents",
    "/api/v1/debug/equipment-data/download",
    "/api/v1/debug/equipment-data/fields",
    "/api/v1/debug/equipment-data/overview",
    "/api/v1/debug/equipment-data/properties",
    "/api/v1/debug/equipment-data/relationships",
    "/api/v1/debug/equipment-data/resource-cards",
    "/api/v1/debug/equipment-data/resources",
    "/api/v1/debug/equipment-data/selection",
    "/api/v1/debug/equipment-data/status",
    "/api/v1/debug/equipment-data/values",
];

static PEER: AtomicU32 = AtomicU32::new(1);

/// A synthetic client address no other request in this binary has used.
fn next_peer() -> SocketAddr {
    let [_, b, c, d] = PEER.fetch_add(1, Ordering::Relaxed).to_be_bytes();
    SocketAddr::from((IpAddr::from([10, b, c, d]), 41000))
}

/// The router a deployment with `app_env` serves, over this binary's test database.
async fn router_for(app_env: &str) -> Router {
    let url = common::require_test_database_url()
        .expect("the per-binary test database is provisioned before any case runs");
    let pool = api_database::connect(&url).await.expect("connect");
    let mut config = Config::for_tests(url, "debug-routes-secret");
    config.env = app_env.into();
    router(api_server::composition::application_state(pool, config))
}

/// The status an anonymous `GET path` answers with.
async fn status_of(app: &Router, path: &str) -> StatusCode {
    let mut request = Request::builder()
        .method("GET")
        .uri(path)
        .body(Body::empty())
        .expect("request");
    request.extensions_mut().insert(ConnectInfo(next_peer()));
    let status = app
        .clone()
        .oneshot(request)
        .await
        .expect("infallible")
        .status();
    assert_ne!(
        status,
        StatusCode::TOO_MANY_REQUESTS,
        "GET {path}: the rate limiter answered, so the status says nothing about the route"
    );
    status
}

#[tokio::test]
async fn route_acceptance_debug_equipment_routes_answer_404_under_production_configuration() {
    let app = router_for("production").await;
    let mut served = Vec::new();
    for path in DEBUG_EQUIPMENT_PATHS {
        let status = status_of(&app, path).await;
        if status != StatusCode::NOT_FOUND {
            served.push(format!("GET {path} -> {status}"));
        }
    }
    assert!(
        served.is_empty(),
        "a production router serves development-only debug routes:\n  {}",
        served.join("\n  ")
    );
}

#[tokio::test]
async fn route_acceptance_debug_equipment_routes_are_served_under_development_configuration() {
    let app = router_for("development").await;
    let mut missing = Vec::new();
    for path in DEBUG_EQUIPMENT_PATHS {
        let status = status_of(&app, path).await;
        if status == StatusCode::NOT_FOUND || status == StatusCode::METHOD_NOT_ALLOWED {
            missing.push(format!("GET {path} -> {status}"));
        }
    }
    assert!(
        missing.is_empty(),
        "a development router does not serve these debug routes:\n  {}",
        missing.join("\n  ")
    );
}
