//! The game-runtime roster (`game_runtime_roster::event_roster`) filters whitespace `arma_id`
//! and emits btrimmed keys.
//!
//! # Scope
//!
//! This binary is the HTTP half: plant a whitespace-only `users.arma_id` on an
//! assigned seat and assert GET `/game-runtime/events/:id/roster` does **not** emit it as a
//! seating key. Also pins that a padded real id emits the trimmed form (agreeing with refresh /
//! link-confirm / telemetry).

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use website_api::core::application_state::AppState;
use website_api::core::configuration::Config;
use website_api::core::database;
use website_api::core::http_router;

mod common;

/// Serialise DB-touching tests — share ACTOR / WS_ARMA on one gate DB.
static DB_LOCK: std::sync::LazyLock<tokio::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

/// Private actor — must not share `DEV_LOGIN_USER` or the refresh / profile suite ranges.
const ACTOR: &str = "000000000000529001";
/// Stored whitespace-only `arma_id` (single space).
const WS_ARMA: &str = " ";
/// Unique non-whitespace seed released before we overwrite with WS_ARMA / padded.
const SEED_ARMA: &str = "roster-ws-seed-arma-1";
/// Real content id used for the positive + padded-emit cases (trimmed form).
const REAL_ARMA: &str = "roster-ws-real-arma-1";

/// Editor payload: one BLUFOR squad / one SL seat. Attach without explicit `orbat`, so the
/// ORBAT the attach materializes and the artifact the deployment binds walk the same graph.
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
    let url = common::require_test_database_url()?;
    let pool = database::connect(&url).await.expect("connect");
    database::migrate(&pool).await.expect("migrate");
    let cfg = Config::for_tests(url, "roster-ws-secret");
    let state = AppState::new(pool.clone(), cfg);
    Some((http_router::router(state.clone()), state, pool))
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
        .expect("roster-ws release arma");
    sqlx::query(
        "DELETE FROM mission_deployment_slots WHERE orbat_slot_id IN
             (SELECT id FROM orbat_slots WHERE assigned_to = $1)",
    )
    .bind(ACTOR)
    .execute(pool)
    .await
    .expect("roster-ws clear seat bindings");
    sqlx::query("DELETE FROM orbat_slots WHERE assigned_to = $1")
        .bind(ACTOR)
        .execute(pool)
        .await
        .expect("roster-ws clear seats");
    sqlx::query("WITH removed_participation AS (DELETE FROM event_registration_participation WHERE registration_id IN (SELECT id FROM event_registrations WHERE discord_id = $1)), removed_history AS (DELETE FROM event_registration_history WHERE registration_id IN (SELECT id FROM event_registrations WHERE discord_id = $1)) DELETE FROM event_registrations WHERE discord_id = $1")
        .bind(ACTOR)
        .execute(pool)
        .await
        .expect("roster-ws clear regs");
}

async fn admin_token(app: &Router) -> String {
    common::dev_login_token(app, "roster_whitespace_arma_id", "admin").await
}

/// `bearer` is a member session or, for the game-runtime roster, a machine credential secret.
async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    bearer: Option<&str>,
    body: Option<&str>,
) -> (StatusCode, Value) {
    let mut b = Request::builder().method(method).uri(uri);
    if let Some(t) = bearer {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
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

/// Mission + published editor version + event attach (derive ORBAT), submitted, approved and
/// deployed on the event's server through the real routes. Answers the event, one free slot id
/// and the credential of the server's runtime.
async fn seeded_event_with_slot(
    app: &Router,
    pool: &PgPool,
    admin: &str,
) -> (String, String, String) {
    let (st, m) = call(
        app,
        "POST",
        "/api/v1/missions",
        Some(admin),
        Some(
            r#"{"title":"Roster Whitespace","terrain":"everon","game_mode":"pve_coop","max_players":16}"#,
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
        Some(&ver),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "version: {v}");

    let (st, e) = call(
        app,
        "POST",
        "/api/v1/events",
        Some(admin),
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
    )
    .await;
    assert_eq!(st, StatusCode::OK, "orbat: {orbat}");
    let slot_id = orbat["data"][0]["slots"][0]["id"]
        .as_str()
        .expect("slot id")
        .to_string();

    // The roster is the bindings of the deployment the server runs.
    let (st, submitted) = call(
        app,
        "POST",
        &format!("/api/v1/missions/{mid}/submit"),
        Some(admin),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "submit: {submitted}");
    let (st, history) = call(
        app,
        "GET",
        &format!("/api/v1/missions/{mid}/reviews"),
        Some(admin),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "reviews: {history}");
    let artifact = history["reviews"][0]["artifact_id"]
        .as_str()
        .unwrap()
        .to_string();
    let approval = format!(r#"{{"artifact_id":"{artifact}"}}"#);
    let (st, approved) = call(
        app,
        "POST",
        &format!("/api/v1/approvals/{mid}/approve"),
        Some(admin),
        Some(&approval),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "approve: {approved}");
    let scenario =
        r#"{"scenario_id":"{69A85365FC09E2CA}Missions/TBD_Dev_POC.conf","display_name":"Everon"}"#;
    let (st, registered) = call(
        app,
        "PUT",
        "/api/v1/fleet/scenarios/everon",
        Some(admin),
        Some(scenario),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "scenario: {registered}");
    let secret =
        common::event_runtime_credential(pool, eid.parse().unwrap(), common::DEV_LOGIN_USER).await;
    let server: String =
        sqlx::query_scalar("SELECT server_id::text FROM events WHERE id = $1::uuid")
            .bind(&eid)
            .fetch_one(pool)
            .await
            .unwrap();
    let request = format!(
        r#"{{"mission_id":"{mid}","artifact_id":"{artifact}","event_mission_id":"{emid}"}}"#
    );
    let (st, deployment) = call(
        app,
        "POST",
        &format!("/api/v1/servers/{server}/deployments"),
        Some(admin),
        Some(&request),
    )
    .await;
    assert_eq!(st, StatusCode::ACCEPTED, "deployment: {deployment}");
    assert_eq!(deployment["bound_slots"], 1, "{deployment}");
    (eid, slot_id, secret)
}

/// Roster wire version 2: the assignments as `armaId` → `slotUid`.
fn seating(body: &Value) -> std::collections::BTreeMap<String, String> {
    assert_eq!(body["version"], 2, "roster wire version: {body}");
    body["assignments"]
        .as_array()
        .expect("assignments array")
        .iter()
        .map(|assignment| {
            (
                assignment["armaId"].as_str().unwrap().to_owned(),
                assignment["slotUid"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
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
    common::seed_user(pool, ACTOR, "roster-ws-ws", SEED_ARMA, "enlisted").await;
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

/// The roster as the runtime of the event's registered server reads it.
async fn roster(app: &Router, runtime: &str, event_id: &str) -> Value {
    let (st, body) = call(
        app,
        "GET",
        &format!("/api/v1/game-runtime/events/{event_id}/roster"),
        Some(runtime),
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "roster: {body}");
    body
}

/// Whitespace-only `arma_id` must not appear in `assignments`.
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
    let (eid, slot_id, runtime) = seeded_event_with_slot(&app, &pool, &admin).await;
    plant_arma(&pool, WS_ARMA).await;
    assert!(WS_ARMA.trim().is_empty());
    assign_actor(&pool, &slot_id).await;

    let body = roster(&app, &runtime, &eid).await;
    let assignments = seating(&body);
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
    let (eid, slot_id, runtime) = seeded_event_with_slot(&app, &pool, &admin).await;
    plant_arma(&pool, REAL_ARMA).await;
    assign_actor(&pool, &slot_id).await;

    let body = roster(&app, &runtime, &eid).await;
    let assignments = seating(&body);
    assert_eq!(
        assignments.get(REAL_ARMA).map(String::as_str),
        Some("s1"),
        "real arma_id must map to editor slot uid s1; got {body}"
    );
    let assigned = &body["assignments"][0];
    assert_eq!(
        assigned["orbatSlotId"],
        slot_id.as_str(),
        "the seat a deployment request names"
    );
    assert_eq!(
        body["slots"].as_array().unwrap().len(),
        1,
        "every compiled slot is listed: {body}"
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
    let (eid, slot_id, runtime) = seeded_event_with_slot(&app, &pool, &admin).await;
    let padded = format!("  {REAL_ARMA}  ");
    plant_arma(&pool, &padded).await;
    assign_actor(&pool, &slot_id).await;

    let body = roster(&app, &runtime, &eid).await;
    let assignments = seating(&body);
    assert!(
        !assignments.contains_key(&padded),
        "padded raw must not be the seating key; got {body}"
    );
    assert_eq!(
        assignments.get(REAL_ARMA).map(String::as_str),
        Some("s1"),
        "padded arma_id must emit btrimmed key; got {body}"
    );

    cleanup(&pool).await;
}
