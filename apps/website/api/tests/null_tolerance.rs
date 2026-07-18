//! Null-tolerance regression: rows with every nullable column left NULL (as seeds /
//! external imports / partial inserts produce, and as Go's GORM tolerated) must read
//! back `200` with zero-values, not 500. Guards the Go→Rust `null`→zero-value hazard.
//! Skips without `TEST_DATABASE_URL`.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use website_api::config::Config;
use website_api::state::AppState;
use website_api::{app, db};

async fn boot() -> Option<(Router, PgPool, String)> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let app = app::router(AppState::new(
        pool.clone(),
        Config::for_tests(url, "null-secret"),
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
    let tok = loc
        .split_once('#')
        .unwrap()
        .1
        .split('&')
        .find_map(|p| p.strip_prefix("access_token="))
        .unwrap()
        .to_string();
    Some((app, pool, tok))
}

async fn get(app: &Router, uri: &str, tok: &str) -> (StatusCode, Value) {
    let req = Request::builder()
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {tok}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let st = resp.status();
    let b = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (st, serde_json::from_slice(&b).unwrap_or(Value::Null))
}

/// Every nullable text/timestamp left NULL; NOT-NULL columns get the minimum.
#[tokio::test]
async fn reads_tolerate_all_null_columns() {
    let Some((app, pool, tok)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let uid = "000000000000000099";

    // user — nullable discord_handle/avatar_url/arma_character/ban_reason/created_at/updated_at → NULL.
    sqlx::query("INSERT INTO users (discord_id, username, role) VALUES ($1, 'Null User', 'enlisted') ON CONFLICT (discord_id) DO NOTHING")
        .bind(uid).execute(&pool).await.unwrap();
    let (st, v) = get(&app, &format!("/api/v1/users/{uid}/stats"), &tok).await;
    assert_eq!(st, StatusCode::OK, "user with NULL columns: {v}");
    assert_eq!(v["stats"]["discord_id"], uid);

    // mission (live) — nullable custom_terrain_name/thumbnail/briefing/rejection/created_at/updated_at → NULL.
    let mid = Uuid::new_v4();
    sqlx::query("INSERT INTO missions (id, title, author_id, terrain, game_mode, weather, time_of_day, max_players, status) \
                 VALUES ($1, 'Null Op', $2, 'everon', 'pve_coop', 'clear', '14:00', 16, 'live')")
        .bind(mid).bind(uid).execute(&pool).await.unwrap();
    let (st, v) = get(&app, &format!("/api/v1/missions/{mid}"), &tok).await;
    assert_eq!(st, StatusCode::OK, "mission with NULL columns: {v}");
    assert_eq!(v["title"], "Null Op"); // NULL briefing/thumbnail/etc → "" (omitted via omitempty)

    // announcement (published) — nullable snippet/thumbnail/discord_message_id/timestamps → NULL.
    let aid = Uuid::new_v4();
    sqlx::query("INSERT INTO announcements (id, title, body, tag, author_id, status, is_pinned, pushed_to_discord) \
                 VALUES ($1, 'Null News', 'body', 'update', $2, 'published', false, false)")
        .bind(aid).bind(uid).execute(&pool).await.unwrap();
    let (st, v) = get(&app, &format!("/api/v1/announcements/{aid}"), &tok).await;
    assert_eq!(st, StatusCode::OK, "announcement with NULL columns: {v}");
    assert_eq!(v["created_at"], "0001-01-01T00:00:00Z"); // NULL time → Go zero time

    // server + status — nullable ingame_time/ingame_weather/updated_at → NULL.
    let sid: Uuid = sqlx::query_scalar("INSERT INTO servers (name, ip, port, is_active) VALUES ('Null Srv', '127.0.0.1'::inet, 2099, true) RETURNING id")
        .fetch_one(&pool).await.unwrap();
    sqlx::query("INSERT INTO server_statuses (server_id, is_online, player_count, max_players, server_fps, uptime_seconds) VALUES ($1, true, 5, 64, 30, 10)")
        .bind(sid).execute(&pool).await.unwrap();
    let (st, v) = get(&app, &format!("/api/v1/servers/{sid}/status"), &tok).await;
    assert_eq!(st, StatusCode::OK, "server_status with NULL columns: {v}");
    assert_eq!(v["status"]["player_count"], 5);

    // orbat_slot — nullable callsign/loadout/tag → NULL (via event → event_mission → slot).
    let eid: Uuid = sqlx::query_scalar("INSERT INTO events (start_time, status, registration_locked, max_slots, created_by, created_at, updated_at) \
                 VALUES (now() + interval '30 days', 'scheduled', false, 16, $1, now(), now()) RETURNING id")
        .bind(uid).fetch_one(&pool).await.unwrap();
    let emid: Uuid = sqlx::query_scalar("INSERT INTO event_missions (event_id, mission_id, start_time, created_at, updated_at) VALUES ($1, $2, now(), now(), now()) RETURNING id")
        .bind(eid).bind(mid).fetch_one(&pool).await.unwrap();
    sqlx::query("INSERT INTO orbat_slots (event_mission_id, faction, squad, role, slot_index) VALUES ($1, 'USA', 'Alpha', 'SL', 0)")
        .bind(emid).execute(&pool).await.unwrap();
    let (st, v) = get(&app, &format!("/api/v1/event-missions/{emid}/orbat"), &tok).await;
    assert_eq!(st, StatusCode::OK, "orbat_slot with NULL columns: {v}");
    assert_eq!(v["data"][0]["slots"][0]["role"], "SL");
}
