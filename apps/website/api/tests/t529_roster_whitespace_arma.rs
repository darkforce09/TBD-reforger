//! T-529 — `ingest_event_roster` filters whitespace `arma_id` and emits btrimmed keys.
//!
//! # Owns expansion (called out)
//!
//! Wave owns list is `handlers/events.rs` + `apps/website/api/tests/**`. This IT binary is
//! the Class-R / IT half: plant a whitespace-only `users.arma_id` on an assigned seat and
//! assert GET `/ingest/events/:id/roster` does **not** emit it as a seating key. Also pins
//! that a padded real id emits the trimmed form (agree with T-350 / link-confirm / telemetry).

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use website_api::config::Config;
use website_api::state::AppState;
use website_api::{app, db};

mod common;

/// Serialise DB-touching tests — share ACTOR / WS_ARMA on one gate DB.
static DB_LOCK: std::sync::LazyLock<tokio::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

/// Private actor — must not share `DEV_LOGIN_USER` or T-350/T-528 ranges.
const ACTOR: &str = "000000000000529001";
/// Stored whitespace-only `arma_id` (single space — ticket pin).
const WS_ARMA: &str = " ";
/// Unique non-whitespace seed released before we overwrite with WS_ARMA / padded.
const SEED_ARMA: &str = "t529-seed-arma-529001";
/// Real content id used for the positive + padded-emit cases (trimmed form).
const REAL_ARMA: &str = "t529-real-arma-529001";
const SVC: &str = "test-service-token";

/// Editor payload: one BLUFOR squad / one SL seat. Attach without explicit `orbat` so
/// materialize and roster `pair_slots` walk the same graph.
const EDITOR_PAYLOAD: &str = r#"{
  "editor": {
    "factions": [{"id":"f1","key":"BLUFOR","name":"US","squadIds":["sq1"]}],
    "squads": [{"id":"sq1","factionId":"f1","callsign":"A","name":"Alpha","slotIds":["s1"]}],
    "slots": [{
      "id":"s1","squadId":"sq1","index":0,"role":"SL",
      "position":{"x":100,"y":200,"z":0,"rotation":0}
    }],
    "editorLayers": []
  }
}"#;

async fn boot() -> Option<(Router, AppState, PgPool)> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let cfg = Config::for_tests(url, "t529-secret");
    let state = AppState::new(pool.clone(), cfg);
    Some((app::router(state.clone()), state, pool))
}

async fn cleanup(pool: &PgPool) {
    sqlx::query("UPDATE users SET arma_id = NULL WHERE arma_id = ANY($1)")
        .bind(vec![
            WS_ARMA.to_string(),
            SEED_ARMA.to_string(),
            REAL_ARMA.to_string(),
            format!("  {REAL_ARMA}  "),
        ])
        .execute(pool)
        .await
        .expect("t529 release arma");
    sqlx::query("DELETE FROM orbat_slots WHERE assigned_to = $1")
        .bind(ACTOR)
        .execute(pool)
        .await
        .expect("t529 clear seats");
    sqlx::query("DELETE FROM event_registrations WHERE discord_id = $1")
        .bind(ACTOR)
        .execute(pool)
        .await
        .expect("t529 clear regs");
}

async fn admin_token(app: &Router) -> String {
    common::dev_login_token(app, "t529_roster_whitespace_arma", "admin").await
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    tok: Option<&str>,
    svc: Option<&str>,
    body: Option<&str>,
) -> (StatusCode, Value) {
    let mut b = Request::builder().method(method).uri(uri);
    if let Some(t) = tok {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    if let Some(s) = svc {
        b = b.header("x-service-token", s);
    }
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

/// Mission + published editor version + event attach (derive ORBAT) + one free slot id.
async fn seeded_event_with_slot(app: &Router, admin: &str) -> (String, String) {
    let (st, m) = call(
        app,
        "POST",
        "/api/v1/missions",
        Some(admin),
        None,
        Some(
            r#"{"title":"T529 Roster","terrain":"everon","game_mode":"pve_coop","max_players":16}"#,
        ),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "mission: {m}");
    let mid = m["id"].as_str().unwrap();

    // create_mission already stores stub 0.1.0 — publish a real editor graph as 0.2.0.
    let ver = format!(r#"{{"semver":"0.2.0","payload":{EDITOR_PAYLOAD}}}"#);
    let (st, v) = call(
        app,
        "POST",
        &format!("/api/v1/missions/{mid}/versions"),
        Some(admin),
        None,
        Some(&ver),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "version: {v}");

    let (st, e) = call(
        app,
        "POST",
        "/api/v1/events",
        Some(admin),
        None,
        Some(r#"{"start_time":"2027-11-01T00:00:00Z"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "event: {e}");
    let eid = e["id"].as_str().unwrap().to_string();

    // No explicit orbat — derive from the published version so pair_slots stays in lockstep.
    let attach = format!(r#"{{"mission_id":"{mid}","start_time":"2027-11-01T00:00:00Z"}}"#);
    let (st, em) = call(
        app,
        "POST",
        &format!("/api/v1/events/{eid}/missions"),
        Some(admin),
        None,
        Some(&attach),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "attach: {em}");
    let emid = em["id"].as_str().unwrap();

    let (st, orbat) = call(
        app,
        "GET",
        &format!("/api/v1/event-missions/{emid}/orbat"),
        Some(admin),
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "orbat: {orbat}");
    let slot_id = orbat["data"][0]["slots"][0]["id"]
        .as_str()
        .expect("slot id")
        .to_string();
    (eid, slot_id)
}

async fn assign_actor(pool: &PgPool, slot_id: &str) {
    sqlx::query("UPDATE orbat_slots SET assigned_to = $1, assigned_at = now() WHERE id = $2::uuid")
        .bind(ACTOR)
        .bind(slot_id)
        .execute(pool)
        .await
        .expect("assign seat");
}

async fn plant_arma(pool: &PgPool, arma: &str) {
    common::seed_user(pool, ACTOR, "t529-ws", SEED_ARMA, "enlisted").await;
    sqlx::query("UPDATE users SET arma_id = $1, updated_at = now() WHERE discord_id = $2")
        .bind(arma)
        .bind(ACTOR)
        .execute(pool)
        .await
        .expect("plant arma_id");
    let stored: Option<String> =
        sqlx::query_scalar("SELECT arma_id FROM users WHERE discord_id = $1")
            .bind(ACTOR)
            .fetch_one(pool)
            .await
            .expect("read arma_id");
    assert_eq!(stored.as_deref(), Some(arma));
}

async fn roster(app: &Router, event_id: &str) -> Value {
    let (st, body) = call(
        app,
        "GET",
        &format!("/api/v1/ingest/events/{event_id}/roster"),
        None,
        Some(SVC),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "roster: {body}");
    body
}

/// Class-R: whitespace-only `arma_id` must not appear in `assignments`.
///
/// Perturbation: restore `u.arma_id <> ''` + raw SELECT → `" "` is a seating key → fail.
#[tokio::test]
async fn roster_whitespace_arma_id_is_not_seated() {
    let _guard = DB_LOCK.lock().await;
    let Some((app, _state, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    cleanup(&pool).await;
    let admin = admin_token(&app).await;
    let (eid, slot_id) = seeded_event_with_slot(&app, &admin).await;
    plant_arma(&pool, WS_ARMA).await;
    assert!(WS_ARMA.trim().is_empty());
    assign_actor(&pool, &slot_id).await;

    let body = roster(&app, &eid).await;
    let assignments = body["assignments"].as_object().expect("assignments object");
    assert!(
        !assignments.contains_key(WS_ARMA),
        "whitespace arma_id must not be a seating key; got {body}"
    );
    assert!(
        assignments.keys().all(|k| !k.trim().is_empty()),
        "no whitespace-only seating keys; got {body}"
    );
    assert!(
        assignments.is_empty(),
        "whitespace-only claim must yield empty assignments; got {body}"
    );

    cleanup(&pool).await;
}

/// Real content still seats — guards against "always empty" vacuity.
#[tokio::test]
async fn roster_real_arma_id_is_seated() {
    let _guard = DB_LOCK.lock().await;
    let Some((app, _state, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    cleanup(&pool).await;
    let admin = admin_token(&app).await;
    let (eid, slot_id) = seeded_event_with_slot(&app, &admin).await;
    plant_arma(&pool, REAL_ARMA).await;
    assign_actor(&pool, &slot_id).await;

    let body = roster(&app, &eid).await;
    let assignments = body["assignments"].as_object().expect("assignments object");
    assert_eq!(
        assignments.get(REAL_ARMA).and_then(|v| v.as_str()),
        Some("s1"),
        "real arma_id must map to editor slot uid s1; got {body}"
    );

    cleanup(&pool).await;
}

/// Padded real id must emit the **trimmed** seating key (agree with telemetry / link-confirm).
#[tokio::test]
async fn roster_padded_arma_id_emits_trimmed_key() {
    let _guard = DB_LOCK.lock().await;
    let Some((app, _state, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    cleanup(&pool).await;
    let admin = admin_token(&app).await;
    let (eid, slot_id) = seeded_event_with_slot(&app, &admin).await;
    let padded = format!("  {REAL_ARMA}  ");
    plant_arma(&pool, &padded).await;
    assign_actor(&pool, &slot_id).await;

    let body = roster(&app, &eid).await;
    let assignments = body["assignments"].as_object().expect("assignments object");
    assert!(
        !assignments.contains_key(&padded),
        "padded raw must not be the seating key; got {body}"
    );
    assert_eq!(
        assignments.get(REAL_ARMA).and_then(|v| v.as_str()),
        Some("s1"),
        "padded arma_id must emit btrimmed key; got {body}"
    );

    cleanup(&pool).await;
}
