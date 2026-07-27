//! **T-413 — `events.banner_image_url` adopts T-405/T-391's scheme guard on BOTH writers.**
//!
//! Predicate coverage lives in `services::text::is_http_url` + the shared case table. These prove
//! the **wiring** — create and PATCH both reach the guard, and a rejection leaves the DB alone.
//!
//! Skips without `TEST_DATABASE_URL` — a skip is a failure to have tested, not a pass.

mod common;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use chrono::{Duration, Utc};
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use website_api::config::Config;
use website_api::state::AppState;
use website_api::{app, db};

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
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let app = app::router(AppState::new(
        pool.clone(),
        Config::for_tests(url, "t413-events-secret"),
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
async fn create_refuses_a_non_http_banner_and_stores_nothing() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let token = common::dev_login_token(&app, SUITE, "admin").await;
    let start = future_start();

    for bad in REJECTED {
        let name = format!("t413-create-{bad:?}");
        let (status, body) = send(
            &app,
            "POST",
            "/api/v1/events",
            &token,
            json!({
                "name_override": name,
                "start_time": start,
                "banner_image_url": bad,
                "max_slots": 16,
            }),
        )
        .await;

        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "create accepted banner_image_url {bad:?} (body: {body})"
        );
        let msg = body.get("error").and_then(Value::as_str).unwrap_or("");
        assert!(
            msg.contains("banner_image_url"),
            "create rejected {bad:?} for the wrong reason: {msg:?}"
        );

        let n: i64 = sqlx::query_scalar("SELECT count(*) FROM events WHERE name_override = $1")
            .bind(&name)
            .fetch_one(&pool)
            .await
            .expect("count");
        assert_eq!(n, 0, "create stored a row despite 400-ing on {bad:?}");
    }
}

#[tokio::test]
async fn patch_refuses_a_non_http_banner_and_leaves_the_stored_value_alone() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let token = common::dev_login_token(&app, SUITE, "admin").await;

    const GOOD: &str = "https://cdn.tbd/banners/original.png";
    let name = format!("t413-patch-{}", uuid::Uuid::new_v4());
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

#[tokio::test]
async fn patch_rejection_leaves_every_other_field_untouched() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let token = common::dev_login_token(&app, SUITE, "admin").await;

    let name = format!("t413-atomic-{}", uuid::Uuid::new_v4());
    let (status, created) = send(
        &app,
        "POST",
        "/api/v1/events",
        &token,
        json!({
            "name_override": name,
            "start_time": future_start(),
            "briefing": "original briefing",
            "max_slots": 16,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "seed create failed: {created}");
    let id = uuid::Uuid::parse_str(created.get("id").and_then(Value::as_str).expect("id")).unwrap();

    let (status, _) = send(
        &app,
        "PATCH",
        &format!("/api/v1/events/{id}"),
        &token,
        json!({
            "name_override": "REWRITTEN",
            "briefing": "REWRITTEN BRIEFING",
            "banner_image_url": "javascript:alert(1)"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT name_override, COALESCE(briefing, '') FROM events WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .expect("re-read");
    assert_eq!(
        row.0, name,
        "a REFUSED PATCH still rewrote name_override — the guard runs after the update is built"
    );
    assert_eq!(
        row.1, "original briefing",
        "a REFUSED PATCH still rewrote the briefing (stored: {:?})",
        row.1
    );

    sqlx::query("DELETE FROM events WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .expect("cleanup");
}

#[tokio::test]
async fn real_banners_and_the_empty_no_banner_shape_still_work() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let token = common::dev_login_token(&app, SUITE, "admin").await;

    for good in [
        "https://cdn.tbd/banners/op-red-dawn.png",
        "http://cdn.tbd:8080/banners/a.png?v=2#x",
        "https://cdn.tbd/banners/Operation%20Red%20Dawn.png",
        "",
    ] {
        let name = format!("t413-good-{}", uuid::Uuid::new_v4());
        let (status, body) = send(
            &app,
            "POST",
            "/api/v1/events",
            &token,
            json!({
                "name_override": name,
                "start_time": future_start(),
                "banner_image_url": good,
                "max_slots": 16,
            }),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::CREATED,
            "create REJECTED the legitimate banner {good:?}: {body}"
        );
        let stored = body
            .get("banner_image_url")
            .and_then(Value::as_str)
            .unwrap_or_default();
        assert_eq!(
            stored, good,
            "create stored a different banner than it was given"
        );
        sqlx::query("DELETE FROM events WHERE name_override = $1")
            .bind(&name)
            .execute(&pool)
            .await
            .expect("cleanup");
    }
}
