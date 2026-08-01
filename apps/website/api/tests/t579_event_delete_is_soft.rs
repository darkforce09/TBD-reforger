//! T-579 — `DELETE /api/v1/events/:id` is a **soft** delete, proven against database state.
//!
//! # Why this suite exists
//!
//! The SPA's delete confirm told the operator "The operation, its attached missions' ORBATs, and
//! all registrations are removed. This cannot be undone." `delete_event` runs
//! `UPDATE events SET deleted_at = now() WHERE id = $1` and nothing else. Nothing cascades. A
//! confirm dialog reporting a destruction that never happened is this program's signature defect
//! aimed at the person clicking the button, and it survived four waves because the claim lived in a
//! `&'static str` in one crate and the behaviour lived in one SQL statement in another, with
//! nothing joining them.
//!
//! This is the behavioural half of that join. It drives the **real** handler through the **real**
//! router and then looks in the database:
//!
//!   * the `events` row is still there, with `deleted_at` stamped;
//!   * `event_missions`, `orbat_slots` and `event_registrations` are **all still there** — the
//!     three things the old copy promised were gone;
//!   * and the operation really has left the schedule (`GET /events/:id` 404s, the list omits it),
//!     because "nothing is destroyed" must not be mistaken for "nothing happened".
//!
//! The frontend half is `frontend/src/event_manager.rs`
//! `delete_confirm_copy_matches_the_soft_delete_handler`, which bans the destructive claims from
//! `DELETE_EVENT_CONFIRM_DESC`. Each test names the other's file in its failure message, so
//! whichever side moves first, the red points at the side that has to move with it.
//!
//! # If you are here because this suite went red
//!
//! You probably changed `delete_event` into a hard delete. That may well be right — the FK that
//! makes it correct has shipped since T-262 (`0018_foreign_keys.sql` constraint 1). But the dialog
//! copy is now wrong in the *other* direction: it currently promises the operator that nothing is
//! erased and the operation can be restored. Rewrite `DELETE_EVENT_CONFIRM_DESC` in the same commit
//! and check what happens to `event_registrations.state = 'attended'` (attendance history) and to
//! `matches.event_id` (no foreign key — a hard delete orphans it silently).
//!
//! Skips without `TEST_DATABASE_URL`, like every suite here.

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

mod common;

/// The constant on the other side of the contract. Quoted in failure messages so a red here hands
/// over the exact symbol to go and fix rather than a vague "update the frontend".
const FRONTEND_COPY: &str = "frontend/src/event_manager.rs DELETE_EVENT_CONFIRM_DESC";

async fn boot() -> Option<(Router, PgPool)> {
    let url = common::require_test_database_url()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let app = app::router(AppState::new(
        pool.clone(),
        Config::for_tests(url, "t579-secret"),
    ));
    Some((app, pool))
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

async fn count(pool: &PgPool, sql: &'static str, id: Uuid) -> i64 {
    sqlx::query_scalar(sql)
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}

/// Whether `GET /events` (default `upcoming` scope) shows this operation.
///
/// Asserted **before** the delete as well as after. `PageParams::bounds` caps `limit` at 100 and
/// silently falls back to 20 above that, and this database accumulates future events from every
/// suite that ran before this one — so "absent from the list" could mean "on page two". Checking
/// presence first turns that into a red on the baseline assertion instead of a false green on the
/// one that matters.
async fn listed(app: &Router, admin: &str, event_id: &str) -> bool {
    let (st, list) = call(app, "GET", "/api/v1/events?limit=100", admin, None).await;
    assert_eq!(st, StatusCode::OK, "list: {list}");
    list["data"]
        .as_array()
        .expect("list body carries data[]")
        .iter()
        .any(|e| e["id"].as_str() == Some(event_id))
}

/// Build an operation that has everything the old copy claimed to destroy: an attached mission, a
/// materialised ORBAT, and a live registration on one of its seats.
async fn seed_operation(app: &Router, admin: &str) -> (String, String, String) {
    let (st, m) = call(
        app,
        "POST",
        "/api/v1/missions",
        admin,
        Some(r#"{"title":"T-579 Op","terrain":"everon","game_mode":"pve_coop","max_players":16}"#),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "mission: {m}");
    let mission_id = m["id"].as_str().unwrap().to_string();

    let (st, e) = call(
        app,
        "POST",
        "/api/v1/events",
        admin,
        Some(r#"{"start_time":"2027-03-01T00:00:00Z"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "event: {e}");
    let event_id = e["id"].as_str().unwrap().to_string();

    let attach = format!(
        r#"{{"mission_id":"{mission_id}","start_time":"2027-03-01T00:00:00Z","orbat":[{{"faction":"USA","callsign":"A","squad":"Alpha","slots":[{{"role":"SL"}},{{"role":"RTO"}}]}}]}}"#
    );
    let (st, em) = call(
        app,
        "POST",
        &format!("/api/v1/events/{event_id}/missions"),
        admin,
        Some(&attach),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "attach: {em}");
    let emid = em["id"].as_str().unwrap().to_string();

    // A real registration on a real seat, through the real endpoint.
    let (st, orbat) = call(
        app,
        "GET",
        &format!("/api/v1/event-missions/{emid}/orbat"),
        admin,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "orbat: {orbat}");
    let slot = orbat["data"][0]["slots"][0]["id"].as_str().unwrap();
    let (st, r) = call(
        app,
        "POST",
        &format!("/api/v1/event-missions/{emid}/register"),
        admin,
        Some(&format!(r#"{{"slot_id":"{slot}"}}"#)),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "register: {r}");
    assert_eq!(r["state"], "registered");

    (event_id, emid, mission_id)
}

#[tokio::test]
async fn delete_event_is_soft_and_destroys_none_of_what_the_dialog_names() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let admin = common::dev_login_token(&app, "t579", "admin").await;
    let (event_id, emid, _mission_id) = seed_operation(&app, &admin).await;
    let ev = Uuid::parse_str(&event_id).unwrap();
    let em = Uuid::parse_str(&emid).unwrap();

    // Baseline: everything the dialog talks about exists.
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM event_missions WHERE event_id = $1",
            ev
        )
        .await,
        1
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM orbat_slots WHERE event_mission_id = $1",
            em
        )
        .await,
        2
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM event_registrations WHERE event_mission_id = $1",
            em
        )
        .await,
        1
    );
    assert!(
        listed(&app, &admin, &event_id).await,
        "baseline: the operation must be on the upcoming list before it is deleted, or the \
         post-delete absence proves nothing"
    );

    let (st, body) = call(
        &app,
        "DELETE",
        &format!("/api/v1/events/{event_id}"),
        &admin,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NO_CONTENT, "delete: {body}");

    // ── 1. The row is stamped, not gone. ──
    let deleted_at: Option<chrono::DateTime<chrono::Utc>> =
        sqlx::query_scalar("SELECT deleted_at FROM events WHERE id = $1")
            .bind(ev)
            .fetch_one(&pool)
            .await
            .expect("the events row must still exist after DELETE — it is a soft delete");
    assert!(
        deleted_at.is_some(),
        "delete_event must stamp deleted_at; without it the event stays visible and the \
         delete silently does nothing"
    );

    // ── 2. Nothing cascaded. These three are exactly what the pre-T-579 dialog promised were
    //       "removed", and every one of them is still here. ──
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM event_missions WHERE event_id = $1",
            ev
        )
        .await,
        1,
        "event_missions survives a soft delete — no CASCADE fires. If this is now 0 the handler \
         became a hard delete and {FRONTEND_COPY} is wrong in the other direction: it currently \
         tells the operator nothing is erased."
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM orbat_slots WHERE event_mission_id = $1",
            em
        )
        .await,
        2,
        "the ORBAT survives — the dialog must not claim the attached missions' ORBATs are removed \
         ({FRONTEND_COPY})"
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM event_registrations WHERE event_mission_id = $1",
            em
        )
        .await,
        1,
        "registrations survive — the dialog must not claim all registrations are removed \
         ({FRONTEND_COPY}). These rows also carry `state`, which telemetry stamps `attended`: \
         they are attendance history, not schedule."
    );

    // ── 3. …and yet the operation really has gone from the schedule. "Nothing is destroyed" is
    //       not "nothing happened", and the copy has to be right about both halves. ──
    let (st, _) = call(
        &app,
        "GET",
        &format!("/api/v1/events/{event_id}"),
        &admin,
        None,
    )
    .await;
    assert_eq!(
        st,
        StatusCode::NOT_FOUND,
        "a soft-deleted event must 404 on the hub — every read path filters deleted_at IS NULL"
    );
    assert!(
        !listed(&app, &admin, &event_id).await,
        "a soft-deleted event must not appear in GET /events — the operator was told it leaves \
         the schedule"
    );

    // ── 4. And nobody can sign up any more (`register`'s ev_gate is the only child-facing path
    //       that re-checks the parent). ──
    let (st, r) = call(
        &app,
        "POST",
        &format!("/api/v1/event-missions/{emid}/register"),
        &admin,
        Some(r#"{"slot_id":""}"#),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::NOT_FOUND,
        "registration must be refused on a soft-deleted event: {r}"
    );
}
