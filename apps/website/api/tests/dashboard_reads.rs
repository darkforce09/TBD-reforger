//! Dashboard / leaderboards / deployments / LOA / audit reads. Skips without
//! `TEST_DATABASE_URL`. SSE endpoints are excluded (they never complete under oneshot).
//!
//! **T-341 — caller identity owns the seeded rows.** Pre-fix this suite authenticated via
//! `dev-login` (`…001`) while the assertions that mattered for the dashboard 500 class depended
//! on `WHERE assigned_to = $me` / `WHERE discord_id = $me` branches that never saw a matching
//! row — so `GET /dashboard` 200 was vacuous (T-329 measured the same defect in
//! `null_tolerance`). This file mints a private session for [`DASH_UID`] and seeds every
//! caller-scoped row against that same id.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use chrono::Utc;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use website_api::config::Config;
use website_api::state::AppState;
use website_api::{app, db};

mod common;

/// Private snowflake for this suite — never the shared `dev-login` id (`…001`).
const DASH_UID: &str = "000000000000000341";

/// Tag prefix for events this suite inserts. Soft-delete is scoped to this prefix only —
/// never a blanket `DELETE FROM events` (misc_integration `boot_servers` pattern; T-410).
const EVENT_TAG: &str = "T341-Dash-";

/// Boot the router and mint a real admin session for [`DASH_UID`].
///
/// Mints through `POST /auth/refresh` rather than `dev-login` on purpose (T-329 / T-341):
/// `dev-login` always issues `…001`, a shared id on the integration DB. Seeding
/// caller-scoped rows against a different id than the bearer token is exactly how
/// `/dashboard` 200 used to pass without ever executing the assignment branch.
async fn setup() -> Option<(Router, String, PgPool)> {
    let url = common::require_test_database_url()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");

    sqlx::query(
        "INSERT INTO users (discord_id, username, role, is_banned, created_at, updated_at) \
         VALUES ($1, 'T341 Dashboard', 'admin', false, now(), now()) \
         ON CONFLICT (discord_id) DO UPDATE SET role = 'admin', is_banned = false",
    )
    .bind(DASH_UID)
    .execute(&pool)
    .await
    .expect("seed suite user");

    // Retire prior runs of this suite's fixtures.
    let like = format!("{EVENT_TAG}%");
    let _ = sqlx::query(
        "UPDATE events SET deleted_at = now(), updated_at = now() \
         WHERE deleted_at IS NULL AND name_override LIKE $1",
    )
    .bind(&like)
    .execute(&pool)
    .await;
    for sql in [
        "DELETE FROM leave_requests WHERE discord_id = $1",
        "DELETE FROM event_registrations WHERE discord_id = $1",
        "DELETE FROM orbat_slots WHERE assigned_to = $1",
        "DELETE FROM event_missions WHERE event_id IN (SELECT id FROM events WHERE created_by = $1)",
        "DELETE FROM events WHERE created_by = $1",
        "DELETE FROM missions WHERE author_id = $1",
        "DELETE FROM refresh_tokens WHERE discord_id = $1",
    ] {
        sqlx::query(sql)
            .bind(DASH_UID)
            .execute(&pool)
            .await
            .unwrap_or_else(|e| panic!("cleanup `{sql}`: {e}"));
    }

    let raw = format!("t341-dash-{}", Uuid::new_v4());
    sqlx::query(
        "INSERT INTO refresh_tokens (discord_id, token_hash, expires_at, created_at) \
         VALUES ($1, $2, now() + interval '1 hour', now())",
    )
    .bind(DASH_UID)
    .bind(website_api::auth::hash_token(&raw))
    .execute(&pool)
    .await
    .expect("seed session");

    let app = app::router(AppState::new(
        pool.clone(),
        Config::for_tests(url, "dash-secret"),
    ));
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(format!(r#"{{"refresh_token":"{raw}"}}"#)))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "mint session for {DASH_UID}");
    let body: Value =
        serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    let access = body["access_token"]
        .as_str()
        .expect("access_token")
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

/// Seed an upcoming event + mission + ORBAT assignment + registration owned by [`DASH_UID`].
///
/// Returns `(event_id, event_name, event_mission_id)` so assertions can pin identity, not mere
/// key presence.
async fn seed_owned_upcoming(pool: &PgPool) -> (String, String, Uuid) {
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
    .bind(DASH_UID)
    .fetch_one(pool)
    .await
    .expect("compute next_event fixture start_time");

    let mission_id: Uuid = sqlx::query_scalar(
        "INSERT INTO missions (title, author_id, terrain, game_mode, weather, time_of_day, \
         max_players, status, created_at, updated_at) \
         VALUES ($1, $2, 'everon', 'pve_coop', 'clear', '14:00', 16, 'live', now(), now()) \
         RETURNING id",
    )
    .bind(format!("T341 mission {stamp}"))
    .bind(DASH_UID)
    .fetch_one(pool)
    .await
    .expect("seed mission");

    let event_id: Uuid = sqlx::query_scalar(
        "INSERT INTO events (name_override, start_time, status, max_slots, created_by, \
         created_at, updated_at) \
         VALUES ($1, $2, 'scheduled', 16, $3, now(), now()) RETURNING id",
    )
    .bind(&name)
    .bind(start)
    .bind(DASH_UID)
    .fetch_one(pool)
    .await
    .expect("seed event");

    let em_id: Uuid = sqlx::query_scalar(
        "INSERT INTO event_missions (event_id, mission_id, start_time, created_at, updated_at) \
         VALUES ($1, $2, $3, now(), now()) RETURNING id",
    )
    .bind(event_id)
    .bind(mission_id)
    .bind(start)
    .fetch_one(pool)
    .await
    .expect("seed event_mission");

    // Assignment branch: `WHERE orbat_slots.assigned_to = $me` must match the bearer.
    let slot_id: Uuid = sqlx::query_scalar(
        "INSERT INTO orbat_slots (event_mission_id, faction, squad, callsign, role, loadout, \
         tag, slot_index, assigned_to, assigned_at) \
         VALUES ($1, 'USA', 'Alpha', 'HAVOC', 'SL', 'L85A3', 'CMD', 0, $2, now()) RETURNING id",
    )
    .bind(em_id)
    .bind(DASH_UID)
    .fetch_one(pool)
    .await
    .expect("seed orbat_slot");

    // Deployments upcoming branch: `WHERE event_registrations.discord_id = $me`.
    sqlx::query(
        "INSERT INTO event_registrations (event_mission_id, discord_id, slot_id, state, \
         registered_at) VALUES ($1, $2, $3, 'registered', now())",
    )
    .bind(em_id)
    .bind(DASH_UID)
    .bind(slot_id)
    .execute(pool)
    .await
    .expect("seed event_registration");

    (event_id.to_string(), name, em_id)
}

/// Class-R static pins: reinstating a bare star on event_registrations or `.ok().flatten()` on
/// `mission_title_terrain` must turn this red without needing a schema change.
#[test]
fn t341_deployments_source_pins() {
    let src = include_str!("../src/handlers/telemetry/deployments.rs");
    // Needle split so this assert's own source (and handler comments) do not contain the
    // forbidden SQL as one literal.
    let bare_star = concat!("event_registrations.", "*");
    assert!(
        !src.contains(bare_star),
        "T-341: deployments must not SELECT a bare star on event_registrations \
         (bare-* class that 500'd dashboard)"
    );
    let swallowed = concat!(".ok()", ".flatten()");
    assert!(
        !src.contains(swallowed),
        "T-341: mission_title_terrain must not swallow errors via .ok().flatten()"
    );
    assert!(
        src.contains("Result<Option<(String, TerrainType)>, sqlx::Error>"),
        "T-341: mission_title_terrain must return Result so decode/SQL failures propagate"
    );
}

#[tokio::test]
async fn dashboard_leaderboards_deployments_loa_audit() {
    let Some((app, tok, pool)) = setup().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    let (eid, name, _em_id) = seed_owned_upcoming(&pool).await;

    // Dashboard — null-safe aggregate + real next_event + real my_assignment (T-341).
    let (st, body) = call(&app, "GET", "/api/v1/dashboard", &tok, None).await;
    assert_eq!(st, StatusCode::OK, "dashboard: {body}");
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

    // Reachability pin: authenticating as DASH_UID while rows are owned by DASH_UID makes
    // `WHERE assigned_to = $me` fire. Pre-T-341 this was empty under a vacuous 200.
    let assignment = body.get("my_assignment").cloned().unwrap_or(Value::Null);
    assert!(
        assignment.is_object(),
        "my_assignment must be the seeded ORBAT seat owned by the bearer, got {assignment}"
    );
    assert_eq!(
        assignment["event_id"],
        eid.as_str(),
        "my_assignment: {assignment}"
    );
    assert_eq!(assignment["faction"], "USA", "my_assignment: {assignment}");
    assert_eq!(assignment["squad"], "Alpha", "my_assignment: {assignment}");
    assert_eq!(assignment["role"], "SL", "my_assignment: {assignment}");

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
        &format!("/api/v1/users/{DASH_UID}/stats"),
        &tok,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(body["stats"]["discord_id"], DASH_UID);
    assert!(body["attendance_rate"].is_number());

    // My deployments — upcoming must include the seeded registration (exercises the explicit
    // event_registrations column list; a bare-* decode failure would 500 here once a future
    // nullable non-Option column lands).
    let (st, body) = call(&app, "GET", "/api/v1/me/deployments", &tok, None).await;
    assert_eq!(st, StatusCode::OK, "deployments: {body}");
    let upcoming = body["upcoming"].as_array().expect("upcoming array");
    assert!(
        upcoming.iter().any(|u| u["event_id"] == eid.as_str()),
        "upcoming must contain the seeded registration for {eid}, got {upcoming:?}"
    );
    assert!(body["service_history"].is_array());

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
