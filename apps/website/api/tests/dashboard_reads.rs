//! Dashboard / leaderboards / deployments / LOA / audit reads. Skips without
//! `TEST_DATABASE_URL`. SSE endpoints are excluded (they never complete under oneshot).

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use chrono::Utc;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use website_api::config::Config;
use website_api::state::AppState;
use website_api::{app, db};

const DEV_ID: &str = "000000000000000001";

/// Tag prefix for events this suite inserts. Soft-delete is scoped to this prefix only —
/// never a blanket `DELETE FROM events` (misc_integration `boot_servers` pattern; T-410).
const EVENT_TAG: &str = "T410-Dash-";

async fn setup() -> Option<(Router, String, PgPool)> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let _ = sqlx::query("DELETE FROM leave_requests WHERE discord_id = $1")
        .bind(DEV_ID)
        .execute(&pool)
        .await;
    // Retire prior runs of this suite's next_event fixtures so prefer-mine ASC cannot
    // pick a leftover over the row we are about to insert.
    let like = format!("{EVENT_TAG}%");
    let _ = sqlx::query(
        "UPDATE events SET deleted_at = now(), updated_at = now() \
         WHERE deleted_at IS NULL AND name_override LIKE $1",
    )
    .bind(&like)
    .execute(&pool)
    .await;
    let app = app::router(AppState::new(
        pool.clone(),
        Config::for_tests(url, "dash-secret"),
    ));
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
    Some((app, access, pool))
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
    let Some((app, tok, pool)) = setup().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    // Seed a caller-owned upcoming event so `next_event` is a real assertion, not
    // key-presence. Handler prefer-mine (T-410) keeps foreign residue from stealing
    // the slot; place start_time ahead of any other DEV_ID upcoming row so a sibling
    // suite that also creates as admin cannot win ASC among `created_by = me`.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let name = format!("{EVENT_TAG}{stamp}");
    let start: chrono::DateTime<Utc> = sqlx::query_scalar(
        "SELECT CASE \
           WHEN m IS NULL THEN now() + interval '2 hours' \
           WHEN m - interval '1 hour' > now() THEN m - interval '1 hour' \
           ELSE now() + interval '30 seconds' \
         END \
         FROM ( \
           SELECT min(start_time) AS m FROM events \
           WHERE created_by = $1 AND deleted_at IS NULL AND start_time > now() \
             AND status::text IN ('scheduled', 'open', 'live') \
         ) t",
    )
    .bind(DEV_ID)
    .fetch_one(&pool)
    .await
    .expect("compute next_event fixture start_time");
    let start = start.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let create = format!(
        r#"{{"name_override":"{name}","start_time":"{start}","status":"scheduled","max_slots":16}}"#
    );
    let (st, created) = call(&app, "POST", "/api/v1/events", &tok, Some(&create)).await;
    assert_eq!(
        st,
        StatusCode::CREATED,
        "seed next_event fixture: {created}"
    );
    let eid = created["id"].as_str().expect("event id").to_string();

    // Dashboard — null-safe aggregate + real next_event coverage.
    let (st, body) = call(&app, "GET", "/api/v1/dashboard", &tok, None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(body["recent_announcements"].is_array());
    let next = body.get("next_event").cloned().unwrap_or(Value::Null);
    assert!(
        next.is_object(),
        "next_event must be the seeded upcoming op, got {next}"
    );
    assert_eq!(next["event_id"], eid.as_str(), "next_event: {next}");
    assert_eq!(next["name"], name.as_str(), "next_event: {next}");
    assert_eq!(next["status"], "scheduled", "next_event: {next}");
    assert_eq!(next["max_slots"], 16, "next_event: {next}");
    assert!(
        next["start_time"]
            .as_str()
            .is_some_and(|s| s.ends_with('Z')),
        "next_event.start_time must be RFC3339 Z: {next}"
    );
    assert!(next["registered"].is_number(), "next_event: {next}");
    assert!(next["terrain"].is_string(), "next_event: {next}");

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
