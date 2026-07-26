//! **T-239 — announcement body is a plain-text field (no ammonia on write).**
//!
//! The SPA renders `body` as a Leptos text node. Pre-T-239, `create_announcement` /
//! `update_announcement` ran `sanitize_html` (ammonia) before INSERT/UPDATE, which HTML-escaped
//! bare `<` / `&`. Leptos then escaped again → authors saw literal `a &lt; b` on screen.
//!
//! These tests pin the HTTP round-trip: create and body-only PATCH store authored text
//! **byte-identical**, and a body-only PATCH recomputes `snippet` (capped) from the new body.
//!
//! RED perturbation: re-introduce `sanitize_html(&input.body)` in `handlers/cms.rs` — the
//! `assert_eq!(body, AUTHOR)` arms fail because the row contains `&lt;` / `&amp;`.
//!
//! Skips without `TEST_DATABASE_URL` — a skip is a failure to have tested, not a pass.

mod common;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use website_api::config::Config;
use website_api::state::AppState;
use website_api::{app, db};

const SUITE: &str = "cms_announcement_body";
const AUTHOR: &str = "Damage threshold: a < b & c > d";

async fn boot() -> Option<(Router, PgPool)> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let app = app::router(AppState::new(
        pool.clone(),
        Config::for_tests(url, "t239-secret"),
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

#[tokio::test]
async fn create_stores_bare_angle_brackets_without_html_entities() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let token = common::dev_login_token(&app, SUITE, "admin").await;
    let title = format!("t239-create-{}", uuid::Uuid::new_v4());

    let (status, resp) = send(
        &app,
        "POST",
        "/api/v1/cms/announcements",
        &token,
        json!({"title": title, "body": AUTHOR, "tag": "update"}),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create: {resp}");
    assert_eq!(resp["body"], AUTHOR, "JSON response must not entity-escape");
    assert!(
        !resp["body"].as_str().unwrap_or("").contains("&lt;"),
        "response body contains HTML entities: {}",
        resp["body"]
    );

    let id: uuid::Uuid = resp["id"].as_str().unwrap().parse().unwrap();
    let (db_body, db_snip): (String, String) =
        sqlx::query_as("SELECT body, COALESCE(snippet, '') FROM announcements WHERE id = $1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(db_body, AUTHOR, "DB body must be authored plain text");
    assert_eq!(db_snip, AUTHOR, "derived snippet must preserve < / &");
    assert!(!db_body.contains("&lt;"));
    assert!(!db_snip.contains("&amp;"), "snippet must not HTML-escape &");
}

#[tokio::test]
async fn body_only_patch_stores_identity_and_refreshes_snippet() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let token = common::dev_login_token(&app, SUITE, "admin").await;
    let title = format!("t239-patch-{}", uuid::Uuid::new_v4());

    let (status, resp) = send(
        &app,
        "POST",
        "/api/v1/cms/announcements",
        &token,
        json!({
            "title": title,
            "body": "original body without brackets",
            "snippet": "old teaser",
            "tag": "update",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create: {resp}");
    let id = resp["id"].as_str().unwrap().to_string();

    let (status, resp) = send(
        &app,
        "PATCH",
        &format!("/api/v1/cms/announcements/{id}"),
        &token,
        json!({"body": AUTHOR}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "patch: {resp}");
    assert_eq!(resp["body"], AUTHOR);

    let uid: uuid::Uuid = id.parse().unwrap();
    let (db_body, db_snip): (String, String) =
        sqlx::query_as("SELECT body, COALESCE(snippet, '') FROM announcements WHERE id = $1")
            .bind(uid)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(db_body, AUTHOR);
    assert_eq!(
        db_snip, AUTHOR,
        "body-only PATCH must recompute snippet (was 'old teaser')"
    );
    assert!(!db_body.contains("&lt;"));
}

#[tokio::test]
async fn explicit_snippet_is_hard_capped_at_200_runes() {
    let Some((app, _pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let token = common::dev_login_token(&app, SUITE, "admin").await;
    let title = format!("t239-snip-{}", uuid::Uuid::new_v4());
    let long = "字".repeat(250);

    let (status, resp) = send(
        &app,
        "POST",
        "/api/v1/cms/announcements",
        &token,
        json!({
            "title": title,
            "body": "body",
            "snippet": long,
            "tag": "update",
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "create: {resp}");
    let snip = resp["snippet"].as_str().unwrap_or("");
    assert_eq!(
        snip.chars().count(),
        200,
        "explicit snippet must be capped (pre-T-239 stored all 250)"
    );
    assert!(snip.ends_with('…'));
}
