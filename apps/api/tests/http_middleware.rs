//! The request-id and CORS middleware, asserted on the production router.
//!
//! Skips without `TEST_DATABASE_URL`.

use axum::body::Body;
use axum::http::{Method, Request, header};
use tower::ServiceExt;

mod common;
mod router_boot_support;

use router_boot_support::*;

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
