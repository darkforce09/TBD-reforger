//! **T-413 — `missions.thumbnail_url` adopts T-405/T-391's scheme guard on the PATCH writer.**
//!
//! Create hardcodes `thumbnail_url` to `''` and does not accept a body field — PATCH is the only
//! HTTP writer. Authz is `MissionMakerUser` (T-408); these prove the URL guard wiring.
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

const SUITE: &str = "missions_url_guard";

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
        Config::for_tests(url, "t413-missions-secret"),
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

async fn seed_mission(app: &Router, token: &str) -> String {
    let title = format!("t413-m-{}", uuid::Uuid::new_v4());
    let (status, body) = send(
        app,
        "POST",
        "/api/v1/missions",
        token,
        json!({
            "title": title,
            "terrain": "everon",
            "game_mode": "pve_coop",
            "weather": "clear",
            "max_players": 16,
        }),
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "seed create failed: {body}");
    body.get("id")
        .and_then(Value::as_str)
        .expect("id")
        .to_string()
}

#[tokio::test]
async fn patch_refuses_a_non_http_thumbnail_and_leaves_the_stored_value_alone() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let token = common::dev_login_token(&app, SUITE, "mission_maker").await;
    let id = seed_mission(&app, &token).await;

    const GOOD: &str = "https://cdn.tbd/thumbs/original.png";
    let (status, _) = send(
        &app,
        "PATCH",
        &format!("/api/v1/missions/{id}"),
        &token,
        json!({"thumbnail_url": GOOD}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "seed PATCH of good thumbnail failed"
    );

    for bad in REJECTED {
        let (status, body) = send(
            &app,
            "PATCH",
            &format!("/api/v1/missions/{id}"),
            &token,
            json!({"thumbnail_url": bad}),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "PATCH accepted thumbnail_url {bad:?} (body: {body})"
        );
        let msg = body.get("error").and_then(Value::as_str).unwrap_or("");
        assert!(
            msg.contains("thumbnail_url"),
            "PATCH rejected {bad:?} for the wrong reason: {msg:?}"
        );

        let stored: String =
            sqlx::query_scalar("SELECT COALESCE(thumbnail_url, '') FROM missions WHERE id = $1")
                .bind(uuid::Uuid::parse_str(&id).unwrap())
                .fetch_one(&pool)
                .await
                .expect("re-read");
        assert_eq!(
            stored, GOOD,
            "PATCH overwrote the stored thumbnail with {bad:?} despite 400-ing"
        );
    }

    sqlx::query("DELETE FROM mission_versions WHERE mission_id = $1")
        .bind(uuid::Uuid::parse_str(&id).unwrap())
        .execute(&pool)
        .await
        .expect("cleanup versions");
    sqlx::query("DELETE FROM missions WHERE id = $1")
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
    let token = common::dev_login_token(&app, SUITE, "mission_maker").await;
    let id = seed_mission(&app, &token).await;
    let mid = uuid::Uuid::parse_str(&id).unwrap();

    let original_title: String = sqlx::query_scalar("SELECT title FROM missions WHERE id = $1")
        .bind(mid)
        .fetch_one(&pool)
        .await
        .expect("title");

    let (status, _) = send(
        &app,
        "PATCH",
        &format!("/api/v1/missions/{id}"),
        &token,
        json!({
            "title": "REWRITTEN",
            "briefing": "REWRITTEN BRIEFING",
            "thumbnail_url": "javascript:alert(1)"
        }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let row = sqlx::query_as::<_, (String, String)>(
        "SELECT title, COALESCE(briefing, '') FROM missions WHERE id = $1",
    )
    .bind(mid)
    .fetch_one(&pool)
    .await
    .expect("re-read");
    assert_eq!(
        row.0, original_title,
        "a REFUSED PATCH still rewrote the title — the guard runs after the update is built"
    );
    assert_eq!(
        row.1, "",
        "a REFUSED PATCH still rewrote the briefing (stored: {:?})",
        row.1
    );

    sqlx::query("DELETE FROM mission_versions WHERE mission_id = $1")
        .bind(mid)
        .execute(&pool)
        .await
        .expect("cleanup versions");
    sqlx::query("DELETE FROM missions WHERE id = $1")
        .bind(mid)
        .execute(&pool)
        .await
        .expect("cleanup");
}

#[tokio::test]
async fn real_thumbnails_and_the_empty_no_thumbnail_shape_still_work() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let token = common::dev_login_token(&app, SUITE, "mission_maker").await;
    let id = seed_mission(&app, &token).await;

    for good in [
        "https://cdn.tbd/thumbs/op-red-dawn.png",
        "http://cdn.tbd:8080/thumbs/a.png?v=2#x",
        "https://cdn.tbd/thumbs/Operation%20Red%20Dawn.png",
        "",
    ] {
        let (status, body) = send(
            &app,
            "PATCH",
            &format!("/api/v1/missions/{id}"),
            &token,
            json!({"thumbnail_url": good}),
        )
        .await;
        assert_eq!(
            status,
            StatusCode::OK,
            "PATCH REJECTED the legitimate thumbnail {good:?}: {body}"
        );
        let stored = body
            .get("thumbnail_url")
            .and_then(Value::as_str)
            .unwrap_or_default();
        assert_eq!(
            stored, good,
            "PATCH stored a different thumbnail than it was given"
        );
    }

    let mid = uuid::Uuid::parse_str(&id).unwrap();
    sqlx::query("DELETE FROM mission_versions WHERE mission_id = $1")
        .bind(mid)
        .execute(&pool)
        .await
        .expect("cleanup versions");
    sqlx::query("DELETE FROM missions WHERE id = $1")
        .bind(mid)
        .execute(&pool)
        .await
        .expect("cleanup");
}
