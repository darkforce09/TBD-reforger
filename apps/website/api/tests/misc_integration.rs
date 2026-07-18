//! Dev-login redirect/default-role + request-id + CORS middleware. Ports the Go
//! `dev_login_test.go` and the request-id / CORS cases of `middleware_test.go`.
//! Skips without `TEST_DATABASE_URL`.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode, header};
use serde_json::Value;
use tower::ServiceExt;
use website_api::config::Config;
use website_api::state::AppState;
use website_api::{app, db};

const ORIGIN: &str = "http://localhost:5173";

async fn boot() -> Option<Router> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    Some(app::router(AppState::new(
        pool,
        Config::for_tests(url, "misc-secret"),
    )))
}

#[tokio::test]
async fn dev_login_redirects_to_spa() {
    let Some(app) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/dev-login?role=admin")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FOUND);
    let loc = resp.headers()[header::LOCATION].to_str().unwrap();
    assert!(loc.starts_with(ORIGIN), "redirects to SPA: {loc}");
    assert!(loc.contains("access_token="), "carries the token fragment");
}

#[tokio::test]
async fn dev_login_unknown_role_defaults_to_admin() {
    let Some(app) = boot().await else { return };
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/dev-login?role=wizard")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let loc = resp.headers()[header::LOCATION]
        .to_str()
        .unwrap()
        .to_string();
    let tok = loc
        .split_once('#')
        .unwrap()
        .1
        .split('&')
        .find_map(|p| p.strip_prefix("access_token="))
        .unwrap();
    // The minted identity is an admin → /me reports role admin.
    let me = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/me")
                .header(header::AUTHORIZATION, format!("Bearer {tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body = to_bytes(me.into_body(), usize::MAX).await.unwrap();
    let v: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(v["user"]["role"], "admin");
}

#[tokio::test]
async fn request_id_echoed_and_honored() {
    let Some(app) = boot().await else { return };
    // No inbound id → the server generates one.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/announcements")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let rid = resp
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(!rid.is_empty(), "server assigns a request id");

    // Inbound id is honored.
    let resp = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/announcements")
                .header("x-request-id", "trace-123")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.headers()["x-request-id"], "trace-123");
}

#[tokio::test]
async fn cors_reflects_allowed_origin_only() {
    let Some(app) = boot().await else { return };
    // Allowed origin → reflected.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/api/v1/announcements")
                .header(header::ORIGIN, ORIGIN)
                .header("access-control-request-method", "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.headers()[header::ACCESS_CONTROL_ALLOW_ORIGIN], ORIGIN);

    // Disallowed origin → never reflected.
    let resp = app
        .oneshot(
            Request::builder()
                .method(Method::OPTIONS)
                .uri("/api/v1/announcements")
                .header(header::ORIGIN, "http://evil.example")
                .header("access-control-request-method", "GET")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let acao = resp
        .headers()
        .get(header::ACCESS_CONTROL_ALLOW_ORIGIN)
        .and_then(|v| v.to_str().ok());
    assert_ne!(acao, Some("http://evil.example"));
}
