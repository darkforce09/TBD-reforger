//! **`events.banner_image_url` carries the HTTP scheme guard on BOTH writers.**
//!
//! Predicate coverage lives in `http_url_guard::is_http_url` + the shared case table. These prove
//! the **wiring** — create and PATCH both reach the guard, and a rejection leaves the DB alone.
//!
//! Skips without `TEST_DATABASE_URL` — a skip is a failure to have tested, not a pass.

use crate::common;

use api_configuration::configuration::Config;

use api_server::router::router;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use chrono::{Duration, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

const SUITE: &str = "events_url_guard";

const REJECTED: &[&str] = &[
    "javascript:alert(1)",
    "JaVaScRiPt:alert(1)",
    "javascript://evil.com/%0aalert(1)",
    "data:text/html,<script>alert(1)</script>",
    "vbscript:msgbox(1)",
    "file:///etc/passwd",
    "java\tscript:alert(1)",
    "\tjavascript:alert(1)",
    "javascript:alert(1) ",
    "//evil.com/x.png",
    "http://",
    "\u{200b}javascript:alert(1)",
];

async fn boot() -> Option<(Router, PgPool)> {
    let url = common::require_test_database_url()?;
    let pool = api_database::connect(&url).await.expect("connect");
    api_database::migrate(&pool).await.expect("migrate");
    let app = router(api_server::composition::application_state(
        pool.clone(),
        Config::for_tests(url, "url-guard-events-secret"),
    ));
    Some((app, pool))
}

async fn send(
    app: &Router,
    method: &str,
    uri: &str,
    token: &str,
    body: Value,
) -> (StatusCode, Value) {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .expect("build request"),
        )
        .await
        .expect("send");
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), 1 << 20).await.expect("body");
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

fn future_start() -> String {
    (Utc::now() + Duration::hours(24)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[tokio::test]
async fn patch_refuses_a_non_http_banner_and_leaves_the_stored_value_alone() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let token = common::dev_login_token(&app, SUITE, "admin").await;

    const GOOD: &str = "https://cdn.tbd/banners/original.png";
    let name = format!("url-guard-patch-{}", uuid::Uuid::new_v4());
    let (status, created) = send(
        &app,
        "POST",
        "/api/v1/events",
        &token,
        json!({
            "name_override": name,
            "start_time": future_start(),
            "banner_image_url": GOOD,
            "max_slots": 16,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "seed create failed: {created}");
    let id = created
        .get("id")
        .and_then(Value::as_str)
        .expect("id")
        .to_string();

    for bad in REJECTED {
        let (status, body) = send(
            &app,
            "PATCH",
            &format!("/api/v1/events/{id}"),
            &token,
            json!({"banner_image_url": bad}),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "PATCH accepted banner_image_url {bad:?} (body: {body})"
        );
        let msg = body.get("error").and_then(Value::as_str).unwrap_or("");
        assert!(
            msg.contains("banner_image_url"),
            "PATCH rejected {bad:?} for the wrong reason: {msg:?}"
        );

        let stored: String =
            sqlx::query_scalar("SELECT COALESCE(banner_image_url, '') FROM events WHERE id = $1")
                .bind(uuid::Uuid::parse_str(&id).unwrap())
                .fetch_one(&pool)
                .await
                .expect("re-read");
        assert_eq!(
            stored, GOOD,
            "PATCH overwrote the stored banner with {bad:?} despite 400-ing"
        );
    }

    sqlx::query("DELETE FROM events WHERE id = $1")
        .bind(uuid::Uuid::parse_str(&id).unwrap())
        .execute(&pool)
        .await
        .expect("cleanup");
}
