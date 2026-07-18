//! Dashboard / leaderboards / deployments / LOA / audit reads. Skips without
//! `TEST_DATABASE_URL`. SSE endpoints are excluded (they never complete under oneshot).

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use tower::ServiceExt;
use website_api::config::Config;
use website_api::state::AppState;
use website_api::{app, db};

const DEV_ID: &str = "000000000000000001";

async fn setup() -> Option<(Router, String)> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let _ = sqlx::query("DELETE FROM leave_requests WHERE discord_id = $1")
        .bind(DEV_ID)
        .execute(&pool)
        .await;
    let app = app::router(AppState::new(pool, Config::for_tests(url, "dash-secret")));
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/dev-login?role=admin")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let loc = resp.headers()[header::LOCATION].to_str().unwrap();
    let access = loc
        .split_once('#')
        .unwrap()
        .1
        .split('&')
        .find_map(|p| p.strip_prefix("access_token="))
        .unwrap()
        .to_string();
    Some((app, access))
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    tok: &str,
    body: Option<&str>,
) -> (StatusCode, Value) {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {tok}"));
    if body.is_some() {
        b = b.header(header::CONTENT_TYPE, "application/json");
    }
    let req = b
        .body(body.map_or(Body::empty(), |s| Body::from(s.to_string())))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn dashboard_leaderboards_deployments_loa_audit() {
    let Some((app, tok)) = setup().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    // Dashboard — null-safe aggregate.
    let (st, body) = call(&app, "GET", "/api/v1/dashboard", &tok, None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(body["recent_announcements"].is_array());
    // `next_event` key is always present (null-safe); its value depends on shared-DB
    // state (other tests may seed a future event), so only its presence is asserted.
    assert!(body.as_object().unwrap().contains_key("next_event"));

    // Leaderboards — envelope + bad category.
    let (st, body) = call(&app, "GET", "/api/v1/leaderboards", &tok, None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(body["category"], "kd");
    assert!(body["data"].is_array());
    let (st, _) = call(
        &app,
        "GET",
        "/api/v1/leaderboards?category=bogus",
        &tok,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    // User stats — zeroed for a user with no telemetry.
    let (st, body) = call(
        &app,
        "GET",
        &format!("/api/v1/users/{DEV_ID}/stats"),
        &tok,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(body["stats"]["discord_id"], DEV_ID);
    assert!(body["attendance_rate"].is_number());

    // My deployments.
    let (st, body) = call(&app, "GET", "/api/v1/me/deployments", &tok, None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(body["upcoming"].is_array() && body["service_history"].is_array());

    // LOA submit → list → admin review.
    let loa = r#"{"starts_on":"2026-08-01","ends_on":"2026-08-05","reason":"holiday"}"#;
    let (st, body) = call(&app, "POST", "/api/v1/me/leave-requests", &tok, Some(loa)).await;
    assert_eq!(st, StatusCode::CREATED, "loa: {body}");
    let loa_id = body["id"].as_str().unwrap().to_string();
    assert_eq!(body["status"], "pending");
    // Dates serialize as midnight-UTC timestamps (Go time.Time on a date column).
    assert_eq!(body["starts_on"], "2026-08-01T00:00:00Z");

    let (_, body) = call(&app, "GET", "/api/v1/me/leave-requests", &tok, None).await;
    assert!(
        body["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l["id"] == loa_id.as_str())
    );

    let bad = r#"{"starts_on":"nope","ends_on":"2026-08-05"}"#;
    let (st, _) = call(&app, "POST", "/api/v1/me/leave-requests", &tok, Some(bad)).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    let (st, body) = call(&app, "GET", "/api/v1/admin/leave-requests", &tok, None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(body["total"].as_i64().unwrap() >= 1);

    let (st, body) = call(
        &app,
        "PATCH",
        &format!("/api/v1/admin/leave-requests/{loa_id}"),
        &tok,
        Some(r#"{"status":"approved"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(body["status"], "approved");

    // Audit logs list (keyset envelope).
    let (st, body) = call(&app, "GET", "/api/v1/admin/audit-logs", &tok, None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(body["data"].is_array());
    assert!(body.as_object().unwrap().contains_key("next_cursor"));
}
