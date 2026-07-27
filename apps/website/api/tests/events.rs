//! Event + ORBAT + registration lifecycle. dev-login is a single fixed identity, so
//! the multi-actor conflict paths (taken slot, reserved squad) are seeded via direct
//! SQL for a second user id, then driven through the real handler — deterministically
//! exercising the G7b race-loser code (conditional claim reject + reservation guard).
//! Skips without `TEST_DATABASE_URL`.
//!
//! # Fixture ownership (T-334)
//!
//! `cargo test -p website-api` builds 22 test binaries and runs them concurrently against
//! **one** database. This suite's seeded actors therefore live in a private id range that
//! no other suite touches — see [`OTHER`] and [`THIRD`]. They previously sat on
//! `000000000000000002` / `000000000000000003`, and `...003` was **double-booked** with
//! `tests/telemetry.rs`'s `PLAYER_DISCORD` (`telemetry.rs:17`), whose seed carries
//! `ON CONFLICT (discord_id) DO UPDATE SET arma_id = EXCLUDED.arma_id` — i.e. one binary
//! rewriting the other's fixture row mid-run. Both ids are also the dev seed's own
//! (`seeds/content_golden.sql:162-167`).
//!
//! The dev-login extractor now lives in [`common::dev_login_token`], which reports the
//! status, the body and the asking suite on failure instead of panicking on a missing
//! `Location` header. That module also records why the T-365 "dev-login 403s a banned
//! account" diagnosis is false — read it there before re-deriving it.
//!
//! # Intra-suite seed race (T-479)
//!
//! Several tests call [`common::seed_user`] for [`OTHER`] / [`THIRD`] under
//! `cargo test` (parallel by default). Pre-fix `arma()` returned the **fixed** string
//! `events-arma-{discord_id}` — one global `idx_users_arma_id` slot. Parallel seeds (or a
//! leftover foreign holder) panic in `seed_user` with duplicate key on that string
//! (`event_orbat_registration_and_race` / `events-arma-000000000000334002` on a cold gate
//! DB). Cure (same family as T-516/T-517): [`DB_LOCK`] serialises DB-touching tests,
//! [`arma`] mints via [`common::unique_arma`] (AtomicU64 + UUID), and `seed_user` itself
//! releases any foreign holder of the target `arma_id` before upsert.

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

/// Serialise DB-touching tests — they share [`OTHER`] / [`THIRD`] and event rows on one
/// gate DB (T-479). Pattern: `identity_link.rs` / `null_tolerance.rs` (T-516).
static DB_LOCK: std::sync::LazyLock<tokio::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

/// This suite's second actor: the one already holding a seat when the caller claims.
///
/// Namespaced to T-334 so no other test binary can write it. Verified unused across the
/// whole repository — the 18 sibling suites, `src/`, and `seeds/` — before it was picked.
const OTHER: &str = "000000000000334002";
/// A third seeded identity — the one that must stay on the waitlist while someone else
/// moves between seats (T-324). Same T-334 private range as [`OTHER`], and the id that
/// used to collide with `tests/telemetry.rs`.
const THIRD: &str = "000000000000334003";
/// The identity `dev-login` mints for every role (`handlers::dev::DEV_USER_ID`).
///
/// Still shared with every other dev-login caller — that is inherent to the handler, not
/// something this suite can namespace away. Nothing here asserts on that row's columns;
/// it is only ever the *subject* of a request whose effects are checked in this suite's
/// own `events` / `orbat_slots` rows.
const DEV_USER: &str = common::DEV_LOGIN_USER;

/// `arma_id` for a seeded actor (T-479).
///
/// Must be unique across the whole database (`idx_users_arma_id`). A fixed
/// `events-arma-{discord_id}` is one slot and races under parallel IT — mint a durable
/// unique string instead (AtomicU64 + UUID via [`common::unique_arma`]).
fn arma(discord_id: &str) -> String {
    common::unique_arma(&format!("events-arma-{discord_id}"))
}

/// Class-R: two seeds must not share a fixed `arma_id` string (the T-479 cold-gate flake).
///
/// Perturbation: change [`arma`] back to `format!("events-arma-{discord_id}")` → both
/// equal the literal below → assert fails.
#[test]
fn t479_arma_mint_never_collides_on_fixed_string() {
    let a = arma(OTHER);
    let b = arma(OTHER);
    assert_ne!(
        a, b,
        "arma() must mint distinct ids — fixed events-arma-{{discord}} collides under parallel IT"
    );
    let fixed = format!("events-arma-{OTHER}");
    assert_ne!(a, fixed, "mint must not be the pre-T-479 fixed string: {a}");
    assert_ne!(b, fixed, "mint must not be the pre-T-479 fixed string: {b}");
    assert!(
        a.starts_with(&format!("events-arma-{OTHER}-")),
        "traceable prefix required: {a}"
    );
}

/// Class-R: suite snowflakes must stay off telemetry / identity / dev-login ranges (T-517).
///
/// Perturbation: set OTHER/THIRD equal to a known foreign actor → assert fails.
#[test]
fn t479_actor_snowflakes_are_suite_private() {
    // telemetry.rs PLAYER_DISCORD (pre-T-517 collision class)
    const TELEMETRY_PLAYER: &str = "000000000000400003";
    // identity_link.rs ACTOR / PAD
    const IDENTITY_ACTOR: &str = "000000000000400001";
    const IDENTITY_PAD: &str = "000000000000400013";
    assert_ne!(OTHER, THIRD);
    assert_ne!(OTHER, DEV_USER);
    assert_ne!(THIRD, DEV_USER);
    assert_ne!(OTHER, TELEMETRY_PLAYER);
    assert_ne!(THIRD, TELEMETRY_PLAYER);
    assert_ne!(OTHER, IDENTITY_ACTOR);
    assert_ne!(THIRD, IDENTITY_ACTOR);
    assert_ne!(OTHER, IDENTITY_PAD);
    assert_ne!(THIRD, IDENTITY_PAD);
}

async fn boot() -> Option<(Router, PgPool)> {
    let url = common::require_test_database_url()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let app = app::router(AppState::new(
        pool.clone(),
        Config::for_tests(url, "events-secret"),
    ));
    Some((app, pool))
}

async fn token(app: &Router, role: &str) -> String {
    common::dev_login_token(app, "events", role).await
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
async fn event_orbat_registration_and_race() {
    let _serial = DB_LOCK.lock().await;
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let admin = token(&app, "admin").await;
    let leader = token(&app, "leader").await;
    let enl = token(&app, "enlisted").await;
    // A distinct second user for the seeded conflict paths.
    common::seed_user(&pool, OTHER, "Other", &arma(OTHER), "enlisted").await;

    // Mission (admin ≥ mission_maker) + event + attach with a 2-slot ORBAT.
    let (st, m) = call(
        &app,
        "POST",
        "/api/v1/missions",
        &admin,
        Some(r#"{"title":"Ev Op","terrain":"everon","game_mode":"pve_coop","max_players":16}"#),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "mission: {m}");
    let mission_id = m["id"].as_str().unwrap().to_string();
    let (st, e) = call(
        &app,
        "POST",
        "/api/v1/events",
        &admin,
        Some(r#"{"start_time":"2027-01-01T00:00:00Z"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "event: {e}");
    let event_id = e["id"].as_str().unwrap().to_string();
    let attach = format!(
        r#"{{"mission_id":"{mission_id}","start_time":"2027-01-01T00:00:00Z","orbat":[{{"faction":"USA","callsign":"A","squad":"Alpha","slots":[{{"role":"SL"}},{{"role":"RTO"}}]}}]}}"#
    );
    let (st, em) = call(
        &app,
        "POST",
        &format!("/api/v1/events/{event_id}/missions"),
        &admin,
        Some(&attach),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "attach: {em}");
    let emid = em["id"].as_str().unwrap().to_string();

    // Hub + ORBAT.
    let (st, hub) = call(
        &app,
        "GET",
        &format!("/api/v1/events/{event_id}"),
        &enl,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(hub["missions"][0]["total"], 2);
    assert_eq!(hub["missions"][0]["factions"][0], "USA");
    let (st, orbat) = call(
        &app,
        "GET",
        &format!("/api/v1/event-missions/{emid}/orbat"),
        &enl,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(orbat["data"][0]["squad"], "Alpha");
    let slot0 = orbat["data"][0]["slots"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let slot1 = orbat["data"][0]["slots"][1]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Claim slot0, idempotent re-claim, withdraw (frees the slot).
    let (st, r) = call(
        &app,
        "POST",
        &format!("/api/v1/event-missions/{emid}/register"),
        &admin,
        Some(&format!(r#"{{"slot_id":"{slot0}"}}"#)),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "claim: {r}");
    assert_eq!(r["state"], "registered");
    assert_eq!(r["slot_id"], slot0.as_str());
    let (st, _) = call(
        &app,
        "POST",
        &format!("/api/v1/event-missions/{emid}/register"),
        &admin,
        Some(&format!(r#"{{"slot_id":"{slot0}"}}"#)),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "idempotent own-slot re-claim");
    let (st, _) = call(
        &app,
        "DELETE",
        &format!("/api/v1/event-missions/{emid}/register"),
        &admin,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);

    // G7b race-loser: slot1 held by the other user → this claim loses the WHERE → 409.
    sqlx::query("UPDATE orbat_slots SET assigned_to = $1, assigned_at = now() WHERE id = $2")
        .bind(OTHER)
        .bind(slot1.parse::<uuid::Uuid>().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    let (st, r) = call(
        &app,
        "POST",
        &format!("/api/v1/event-missions/{emid}/register"),
        &admin,
        Some(&format!(r#"{{"slot_id":"{slot1}"}}"#)),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT, "taken slot must 409: {r}");
    assert_eq!(r["error"], "slot already taken");
    sqlx::query("UPDATE orbat_slots SET assigned_to = NULL, assigned_at = NULL WHERE id = $1")
        .bind(slot1.parse::<uuid::Uuid>().unwrap())
        .execute(&pool)
        .await
        .unwrap();

    // Reservation guard: Alpha reserved by the other user → non-admin claim → 409.
    sqlx::query("INSERT INTO orbat_reservations (event_mission_id, squad, reserved_by) VALUES ($1, 'Alpha', $2)")
        .bind(emid.parse::<uuid::Uuid>().unwrap())
        .bind(OTHER)
        .execute(&pool)
        .await
        .unwrap();
    let (st, r) = call(
        &app,
        "POST",
        &format!("/api/v1/event-missions/{emid}/register"),
        &enl,
        Some(&format!(r#"{{"slot_id":"{slot1}"}}"#)),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT, "reserved squad: {r}");
    assert_eq!(r["error"], "squad is reserved by a leader");
    sqlx::query("DELETE FROM orbat_reservations WHERE event_mission_id = $1")
        .bind(emid.parse::<uuid::Uuid>().unwrap())
        .execute(&pool)
        .await
        .unwrap();

    // Self reserve/release (leader tier), members, tiers.
    let (st, _) = call(
        &app,
        "POST",
        &format!("/api/v1/event-missions/{emid}/squads/reserve"),
        &leader,
        Some(r#"{"squad":"Alpha"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED);
    let (st, _) = call(
        &app,
        "POST",
        &format!("/api/v1/event-missions/{emid}/squads/release"),
        &leader,
        Some(r#"{"squad":"Alpha"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let (st, mem) = call(&app, "GET", "/api/v1/members", &leader, None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(mem["data"].is_array());
    let (st, _) = call(
        &app,
        "POST",
        "/api/v1/events",
        &enl,
        Some(r#"{"start_time":"2027-01-01T00:00:00Z"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN, "enlisted cannot create event");
    let (st, _) = call(
        &app,
        "POST",
        &format!("/api/v1/event-missions/{emid}/squads/reserve"),
        &enl,
        Some(r#"{"squad":"Alpha"}"#),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::FORBIDDEN,
        "enlisted cannot reserve (needs leader)"
    );
}

/// T-318 — the registration upsert used to orphan an ORBAT seat that nobody could free.
///
/// Two independent failures, both covered here because fixing either alone leaves the bug
/// live: a bad body was collapsed into "no seat" and blanked `event_registrations.slot_id`
/// on the way past (creating orphans), and `withdraw` looked the seat up *through* that same
/// column (so it could never clean one up). The invariant the whole test is really asserting
/// is that `orbat_slots.assigned_to` and `event_registrations.slot_id` cannot be left
/// disagreeing in a way that strands a seat.
#[tokio::test]
async fn register_rejects_bad_bodies_and_withdraw_frees_orphaned_seats() {
    let _serial = DB_LOCK.lock().await;
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let admin = token(&app, "admin").await;
    common::seed_user(&pool, OTHER, "Other", &arma(OTHER), "enlisted").await;

    // Two event-missions: the one under test, plus a second one that must stay untouched
    // when we withdraw from the first (the by-user release has to be event-scoped).
    let mk_em = async |title: &str, squad: &str, slots: &str| -> String {
        let (_, m) = call(
            &app,
            "POST",
            "/api/v1/missions",
            &admin,
            Some(&format!(
                r#"{{"title":"{title}","terrain":"everon","game_mode":"pve_coop","max_players":16}}"#
            )),
        )
        .await;
        let mission_id = m["id"].as_str().unwrap().to_string();
        let (_, e) = call(
            &app,
            "POST",
            "/api/v1/events",
            &admin,
            Some(r#"{"start_time":"2027-03-01T00:00:00Z"}"#),
        )
        .await;
        let event_id = e["id"].as_str().unwrap().to_string();
        let (st, em) = call(
            &app,
            "POST",
            &format!("/api/v1/events/{event_id}/missions"),
            &admin,
            Some(&format!(
                r#"{{"mission_id":"{mission_id}","start_time":"2027-03-01T00:00:00Z","orbat":[{{"faction":"USA","callsign":"A","squad":"{squad}","slots":[{slots}]}}]}}"#
            )),
        )
        .await;
        assert_eq!(st, StatusCode::CREATED, "attach: {em}");
        em["id"].as_str().unwrap().to_string()
    };
    let emid = mk_em("T-318 Op", "Alpha", r#"{"role":"SL"},{"role":"RTO"}"#).await;
    let other_emid = mk_em("T-318 Op B", "Bravo", r#"{"role":"SL"}"#).await;

    let slots = async |em: &str| -> Vec<String> {
        let (_, o) = call(
            &app,
            "GET",
            &format!("/api/v1/event-missions/{em}/orbat"),
            &admin,
            None,
        )
        .await;
        o["data"][0]["slots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["id"].as_str().unwrap().to_string())
            .collect()
    };
    let mine = slots(&emid).await;
    let (slot0, slot1) = (mine[0].clone(), mine[1].clone());
    let far_slot = slots(&other_emid).await[0].clone();

    let seat = async |id: &str| -> Option<String> {
        sqlx::query_scalar::<_, Option<String>>("SELECT assigned_to FROM orbat_slots WHERE id = $1")
            .bind(id.parse::<uuid::Uuid>().unwrap())
            .fetch_one(&pool)
            .await
            .unwrap()
    };
    let reg_slot = async |em: &str| -> Option<Option<uuid::Uuid>> {
        sqlx::query_scalar::<_, Option<uuid::Uuid>>(
            "SELECT slot_id FROM event_registrations WHERE event_mission_id = $1 AND discord_id = $2",
        )
        .bind(em.parse::<uuid::Uuid>().unwrap())
        .bind(DEV_USER)
        .fetch_optional(&pool)
        .await
        .unwrap()
    };
    let register = async |em: &str, body: Option<&str>| -> StatusCode {
        call(
            &app,
            "POST",
            &format!("/api/v1/event-missions/{em}/register"),
            &admin,
            body,
        )
        .await
        .0
    };
    let withdraw = async |em: &str| -> StatusCode {
        call(
            &app,
            "DELETE",
            &format!("/api/v1/event-missions/{em}/register"),
            &admin,
            None,
        )
        .await
        .0
    };

    // ── Part 1: a bad body is a 400, not a silent claim-blanking 200 ─────────
    let claim = format!(r#"{{"slot_id":"{slot0}"}}"#);
    assert_eq!(register(&emid, Some(&claim)).await, StatusCode::OK);
    assert_eq!(seat(&slot0).await.as_deref(), Some(DEV_USER), "claimed");

    // Each of these used to return 200 and null the registration's `slot_id` while leaving
    // `assigned_to` set — the orphan. `{}` is in the list on purpose: it is well-formed JSON,
    // and only decodes as "no seat" if `slot_id` carries `#[serde(default)]`.
    for (label, body) in [
        ("malformed json", Some(r#"{"slot_id":"#)),
        ("empty object", Some("{}")),
        ("wrong json type", Some("[]")),
        ("no body / no content-type", None),
    ] {
        assert_eq!(
            register(&emid, body).await,
            StatusCode::BAD_REQUEST,
            "{label} must be rejected"
        );
        assert_eq!(
            seat(&slot0).await.as_deref(),
            Some(DEV_USER),
            "{label} must not release the seat"
        );
        assert_eq!(
            reg_slot(&emid).await.flatten().map(|u| u.to_string()),
            Some(slot0.clone()),
            "{label} must not blank the registration"
        );
    }

    // Registering with no seat on purpose is still a legal request — it just has to say so.
    assert_eq!(
        register(&emid, Some(r#"{"slot_id":""}"#)).await,
        StatusCode::OK,
        "explicit empty slot_id is the bench registration"
    );

    // T-324: that request also stands the seat down now. It used to be the orphan factory —
    // registration blanked, `assigned_to` left naming the caller — which is what made it the
    // convenient way to reproduce the shape below.
    assert!(
        reg_slot(&emid).await.unwrap().is_none(),
        "registration blank"
    );
    assert_eq!(
        seat(&slot0).await,
        None,
        "a bench registration gives the seat up rather than stranding it"
    );

    // ── Part 2: withdraw frees the seat through `assigned_to`, not `slot_id` ─
    // The orphan shape — claim held, registration blank — is no longer reachable through the
    // API, so seed it. Rows in this state exist from before both fixes and have to stay
    // recoverable by their occupant; pre-T-318 this was terminal, because withdraw looked the
    // seat up through the column that was blank and then deleted the row anyway.
    sqlx::query("UPDATE orbat_slots SET assigned_to = $1, assigned_at = now() WHERE id = $2")
        .bind(DEV_USER)
        .bind(slot0.parse::<uuid::Uuid>().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(seat(&slot0).await.as_deref(), Some(DEV_USER), "seat held");
    assert_eq!(withdraw(&emid).await, StatusCode::OK);
    assert_eq!(seat(&slot0).await, None, "withdraw must free the orphan");

    // The worst pre-existing state: seat claimed with NO registration row at all, which is
    // where every orphan ended up after its owner's first (silently useless) withdraw.
    sqlx::query("UPDATE orbat_slots SET assigned_to = $1, assigned_at = now() WHERE id = $2")
        .bind(DEV_USER)
        .bind(slot0.parse::<uuid::Uuid>().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    assert!(reg_slot(&emid).await.is_none(), "no registration row");
    assert_eq!(
        withdraw(&emid).await,
        StatusCode::OK,
        "a stranded seat must be releasable by its occupant"
    );
    assert_eq!(seat(&slot0).await, None);

    // ── Part 2b: the broader delete must not over-free ───────────────────────
    // Someone else's seat in the same operation, and my own seat in a different one.
    sqlx::query("UPDATE orbat_slots SET assigned_to = $1, assigned_at = now() WHERE id = $2")
        .bind(OTHER)
        .bind(slot1.parse::<uuid::Uuid>().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    let far_claim = format!(r#"{{"slot_id":"{far_slot}"}}"#);
    assert_eq!(
        register(&other_emid, Some(&far_claim)).await,
        StatusCode::OK
    );
    assert_eq!(register(&emid, Some(&claim)).await, StatusCode::OK);

    assert_eq!(withdraw(&emid).await, StatusCode::OK);
    assert_eq!(seat(&slot0).await, None, "my seat here is freed");
    assert_eq!(
        seat(&slot1).await.as_deref(),
        Some(OTHER),
        "another user's seat in the same event-mission must survive"
    );
    assert_eq!(
        seat(&far_slot).await.as_deref(),
        Some(DEV_USER),
        "my seat in a different event-mission must survive"
    );

    // And holding nothing here is still a 404 — the fallback widened what withdraw can free,
    // not who is allowed to call it or what it reports when there is nothing to do.
    assert_eq!(withdraw(&emid).await, StatusCode::NOT_FOUND);

    // T-511: multi-seat seed retired. A partial unique on
    // (event_mission_id, assigned_to) WHERE assigned_to IS NOT NULL makes the
    // legacy two-seat shape unreachable (and the index cannot be DEFERRABLE).
    // T-318 recovery intent remains in Part 2 above — orphan seats freed via
    // `assigned_to`, not `slot_id`. Prove the structural guard: a second seat
    // for the same occupant must raise unique_violation (SQLSTATE 23505).
    assert_eq!(register(&emid, Some(&claim)).await, StatusCode::OK);
    assert_eq!(
        seat(&slot0).await.as_deref(),
        Some(DEV_USER),
        "one seat held"
    );
    let dup =
        sqlx::query("UPDATE orbat_slots SET assigned_to = $1, assigned_at = now() WHERE id = $2")
            .bind(DEV_USER)
            .bind(slot1.parse::<uuid::Uuid>().unwrap())
            .execute(&pool)
            .await;
    let err = dup
        .expect_err("second seat for same assigned_to must fail under idx_orbat_slots_em_assigned");
    let db = err
        .as_database_error()
        .expect("sqlx DatabaseError for unique_violation");
    assert_eq!(
        db.code().as_deref(),
        Some("23505"),
        "SQLSTATE unique_violation, got {db:?}"
    );
    let held: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM orbat_slots WHERE event_mission_id = $1 AND assigned_to = $2",
    )
    .bind(emid.parse::<uuid::Uuid>().unwrap())
    .bind(DEV_USER)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(held, 1, "partial unique keeps a single occupant seat");
    assert_eq!(withdraw(&emid).await, StatusCode::OK);
    let held: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM orbat_slots WHERE event_mission_id = $1 AND assigned_to = $2",
    )
    .bind(emid.parse::<uuid::Uuid>().unwrap())
    .bind(DEV_USER)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        held, 0,
        "withdraw still frees the seat the caller holds here"
    );
}

/// T-324 — two factions fielding a squad of the same name must render as two cards.
///
/// `get_orbat` grouped on the squad NAME alone, so a same-named squad in a second faction was
/// folded into the first faction's card: one card, the first faction's label, both factions' slots
/// in it, and the second faction absent from the response entirely — its seats could not be seen
/// or picked. Grouping is now keyed on `(faction, squad)`.
///
/// The collision is mostly hidden today because `idx_orbat_slot` is unique on
/// `(event_mission_id, squad, slot_index)`, so attaching two same-named squads that both start at
/// slot 0 fails on duplicate key first. It is only *mostly* hidden: non-overlapping slot indices
/// collide on the name without colliding on the index, which is what this test seeds — so the bug
/// is reachable now, and stays reachable when that index widens to include `faction`.
#[tokio::test]
async fn orbat_groups_by_faction_and_squad() {
    let _serial = DB_LOCK.lock().await;
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let admin = token(&app, "admin").await;
    let (_, m) = call(
        &app,
        "POST",
        "/api/v1/missions",
        &admin,
        Some(r#"{"title":"T-324 Factions","terrain":"everon","game_mode":"pve_coop","max_players":16}"#),
    )
    .await;
    let mission_id = m["id"].as_str().unwrap().to_string();
    let (_, e) = call(
        &app,
        "POST",
        "/api/v1/events",
        &admin,
        Some(r#"{"start_time":"2027-07-01T00:00:00Z"}"#),
    )
    .await;
    let event_id = e["id"].as_str().unwrap().to_string();
    let (st, em) = call(
        &app,
        "POST",
        &format!("/api/v1/events/{event_id}/missions"),
        &admin,
        Some(&format!(
            r#"{{"mission_id":"{mission_id}","start_time":"2027-07-01T00:00:00Z","orbat":[{{"faction":"BLUFOR","callsign":"ALPHA","squad":"Alpha 1-1","slots":[{{"role":"SL"}},{{"role":"RTO"}}]}}]}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "attach: {em}");
    let emid = em["id"].as_str().unwrap().to_string();

    // The same squad NAME under a second faction. Seeded, because `materialize_slots` numbers
    // each squad from 0 and the current unique index rejects the second one at that number —
    // indices 2 and 3 collide on the name only, which is the case under test.
    sqlx::query(
        "INSERT INTO orbat_slots (event_mission_id, faction, squad, callsign, role, slot_index) \
         VALUES ($1, 'OPFOR', 'Alpha 1-1', 'GHOST', 'Team Leader', 2), \
                ($1, 'OPFOR', 'Alpha 1-1', 'GHOST', 'Marksman', 3)",
    )
    .bind(emid.parse::<uuid::Uuid>().unwrap())
    .execute(&pool)
    .await
    .unwrap();

    let (st, o) = call(
        &app,
        "GET",
        &format!("/api/v1/event-missions/{emid}/orbat"),
        &admin,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let cards = o["data"].as_array().unwrap();
    assert_eq!(
        cards.len(),
        2,
        "one card per (faction, squad) — pre-fix this was 1 and OPFOR was gone: {o}"
    );
    assert_eq!(cards[0]["faction"], "BLUFOR");
    assert_eq!(cards[0]["squad"], "Alpha 1-1");
    assert_eq!(
        cards[0]["total"], 2,
        "and does not absorb the other faction"
    );
    assert_eq!(cards[1]["faction"], "OPFOR");
    assert_eq!(cards[1]["squad"], "Alpha 1-1");
    assert_eq!(cards[1]["total"], 2);
    assert_eq!(
        cards[1]["slots"][0]["role"], "Team Leader",
        "OPFOR's own slots, not a duplicate of BLUFOR's"
    );
}

/// T-324 — a second claim MOVES the caller's seat; it does not mint a second one.
///
/// The bug T-318 measured and left standing: claim slot0, claim slot1, both requests entirely
/// valid and both 200, and the caller ends up holding two `orbat_slots` rows while their single
/// `event_registrations` row names one. Measured against the pre-fix binary over real HTTP, a
/// 2-slot ORBAT then reported `filled: 2, registered: 1` — an operation that reads FULL with one
/// person signed up, and stays that way until someone withdraws.
///
/// The invariant under test is one seat per caller per event-mission, and that it is the seat the
/// registration names. Everything else here is a bound on the release: it must not reach another
/// user's seat, another operation's seat, or the waitlist.
#[tokio::test]
async fn register_moves_the_caller_s_seat() {
    let _serial = DB_LOCK.lock().await;
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let admin = token(&app, "admin").await;
    for id in [OTHER, THIRD] {
        // `arma_id` carries its own unique index, so seeded users cannot all share the empty
        // string the way one of them can — `arma()` derives a distinct one per actor.
        common::seed_user(&pool, id, "Seeded", &arma(id), "enlisted").await;
    }

    // The operation under test (3 slots), plus a second one the release must never reach.
    let mk_em = async |title: &str, squad: &str, slots: &str| -> String {
        let (_, m) = call(
            &app,
            "POST",
            "/api/v1/missions",
            &admin,
            Some(&format!(
                r#"{{"title":"{title}","terrain":"everon","game_mode":"pve_coop","max_players":16}}"#
            )),
        )
        .await;
        let mission_id = m["id"].as_str().unwrap().to_string();
        let (_, e) = call(
            &app,
            "POST",
            "/api/v1/events",
            &admin,
            Some(r#"{"start_time":"2027-06-01T00:00:00Z"}"#),
        )
        .await;
        let event_id = e["id"].as_str().unwrap().to_string();
        let (st, em) = call(
            &app,
            "POST",
            &format!("/api/v1/events/{event_id}/missions"),
            &admin,
            Some(&format!(
                r#"{{"mission_id":"{mission_id}","start_time":"2027-06-01T00:00:00Z","orbat":[{{"faction":"USA","callsign":"A","squad":"{squad}","slots":[{slots}]}}]}}"#
            )),
        )
        .await;
        assert_eq!(st, StatusCode::CREATED, "attach: {em}");
        em["id"].as_str().unwrap().to_string()
    };
    let ids = async |em: &str| -> Vec<String> {
        let (_, o) = call(
            &app,
            "GET",
            &format!("/api/v1/event-missions/{em}/orbat"),
            &admin,
            None,
        )
        .await;
        o["data"][0]["slots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["id"].as_str().unwrap().to_string())
            .collect()
    };
    let emid = mk_em(
        "T-324 Op",
        "Alpha",
        r#"{"role":"SL"},{"role":"RTO"},{"role":"AR"}"#,
    )
    .await;
    let other_emid = mk_em("T-324 Op B", "Bravo", r#"{"role":"SL"}"#).await;
    let slots = ids(&emid).await;
    let (slot0, slot1, slot2) = (slots[0].clone(), slots[1].clone(), slots[2].clone());
    let far_slot = ids(&other_emid).await[0].clone();

    let uid = |s: &str| s.parse::<uuid::Uuid>().unwrap();
    let seat = async |id: &str| -> Option<String> {
        sqlx::query_scalar::<_, Option<String>>("SELECT assigned_to FROM orbat_slots WHERE id = $1")
            .bind(uid(id))
            .fetch_one(&pool)
            .await
            .unwrap()
    };
    let seat_ts = async |id: &str| -> bool {
        sqlx::query_scalar::<_, bool>(
            "SELECT assigned_at IS NOT NULL FROM orbat_slots WHERE id = $1",
        )
        .bind(uid(id))
        .fetch_one(&pool)
        .await
        .unwrap()
    };
    let held = async |em: &str, who: &str| -> i64 {
        sqlx::query_scalar(
            "SELECT count(*) FROM orbat_slots WHERE event_mission_id = $1 AND assigned_to = $2",
        )
        .bind(uid(em))
        .bind(who)
        .fetch_one(&pool)
        .await
        .unwrap()
    };
    let reg = async |em: &str, who: &str| -> Option<(Option<uuid::Uuid>, String)> {
        sqlx::query_as::<_, (Option<uuid::Uuid>, String)>(
            "SELECT slot_id, state::text FROM event_registrations WHERE event_mission_id = $1 AND discord_id = $2",
        )
        .bind(uid(em))
        .bind(who)
        .fetch_optional(&pool)
        .await
        .unwrap()
    };
    let register = async |em: &str, body: &str| -> StatusCode {
        call(
            &app,
            "POST",
            &format!("/api/v1/event-missions/{em}/register"),
            &admin,
            Some(body),
        )
        .await
        .0
    };
    let withdraw = async |em: &str| -> StatusCode {
        call(
            &app,
            "DELETE",
            &format!("/api/v1/event-missions/{em}/register"),
            &admin,
            None,
        )
        .await
        .0
    };

    // Two bystanders seeded directly (dev-login mints one identity): one holding a seat in the
    // same squad, one waiting.
    sqlx::query("UPDATE orbat_slots SET assigned_to = $1, assigned_at = now() WHERE id = $2")
        .bind(OTHER)
        .bind(uid(&slot2))
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO event_registrations (event_mission_id, discord_id, slot_id, state) VALUES ($1, $2, $3, 'registered')",
    )
    .bind(uid(&emid))
    .bind(OTHER)
    .bind(uid(&slot2))
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO event_registrations (event_mission_id, discord_id, slot_id, state) VALUES ($1, $2, NULL, 'waitlisted')",
    )
    .bind(uid(&emid))
    .bind(THIRD)
    .execute(&pool)
    .await
    .unwrap();
    // And a seat for the caller in a different operation, which is not this request's business.
    assert_eq!(
        register(&other_emid, &format!(r#"{{"slot_id":"{far_slot}"}}"#)).await,
        StatusCode::OK
    );

    // ── The move ────────────────────────────────────────────────────────────
    assert_eq!(
        register(&emid, &format!(r#"{{"slot_id":"{slot0}"}}"#)).await,
        StatusCode::OK
    );
    assert_eq!(seat(&slot0).await.as_deref(), Some(DEV_USER));
    assert_eq!(
        register(&emid, &format!(r#"{{"slot_id":"{slot0}"}}"#)).await,
        StatusCode::OK,
        "re-claiming the seat you already hold is still idempotent"
    );
    assert_eq!(held(&emid, DEV_USER).await, 1, "and does not duplicate it");

    assert_eq!(
        register(&emid, &format!(r#"{{"slot_id":"{slot1}"}}"#)).await,
        StatusCode::OK
    );
    // This is the whole ticket: pre-fix, `held` was 2 here and slot0 still named the caller.
    assert_eq!(held(&emid, DEV_USER).await, 1, "one seat, not two");
    assert_eq!(seat(&slot0).await, None, "the seat moved off must be free");
    assert!(
        !seat_ts(&slot0).await,
        "and its assigned_at cleared with it"
    );
    assert_eq!(seat(&slot1).await.as_deref(), Some(DEV_USER));
    assert_eq!(
        reg(&emid, DEV_USER).await,
        Some((Some(uid(&slot1)), "registered".into())),
        "the registration names the seat that is actually held"
    );

    // ── Bounds on the release ───────────────────────────────────────────────
    assert_eq!(
        seat(&slot2).await.as_deref(),
        Some(OTHER),
        "another user's seat in the same operation is untouched"
    );
    assert_eq!(
        seat(&far_slot).await.as_deref(),
        Some(DEV_USER),
        "the caller's seat in a different operation is untouched"
    );
    assert_eq!(
        reg(&emid, THIRD).await.unwrap().1,
        "waitlisted",
        "moving between seats must not promote — the caller never left"
    );
    assert_eq!(
        reg(&emid, OTHER).await.unwrap().1,
        "registered",
        "and must not disturb anyone else's registration"
    );

    // ── The bench branch gives the seat up rather than orphaning it ─────────
    // `{"slot_id":""}` nulls the registration's `slot_id` by design, so leaving `assigned_to`
    // set is exactly the T-318 orphan. It is now the one thing a valid request cannot produce.
    assert_eq!(register(&emid, r#"{"slot_id":""}"#).await, StatusCode::OK);
    assert_eq!(held(&emid, DEV_USER).await, 0, "benched holds no seat");
    assert_eq!(
        reg(&emid, DEV_USER).await,
        Some((None, "registered".into())),
        "still registered, just without a seat"
    );
    assert_eq!(
        reg(&emid, THIRD).await.unwrap().1,
        "waitlisted",
        "standing down from a seat is not a departure either"
    );

    // ── A leader assignment is a seat move too ──────────────────────────────
    // Same defect, different door: `assign_slot` claimed the new seat and left the old one.
    sqlx::query("UPDATE orbat_slots SET assigned_to = $1, assigned_at = now() WHERE id = $2")
        .bind(THIRD)
        .bind(uid(&slot0))
        .execute(&pool)
        .await
        .unwrap();
    let (st, a) = call(
        &app,
        "PUT",
        &format!("/api/v1/event-missions/{emid}/slots/{slot1}/assign"),
        &admin,
        Some(&format!(r#"{{"discord_id":"{THIRD}"}}"#)),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "assign: {a}");
    assert_eq!(held(&emid, THIRD).await, 1, "assigned one seat, not two");
    assert_eq!(seat(&slot0).await, None);
    assert_eq!(seat(&slot1).await.as_deref(), Some(THIRD));

    // ── Withdrawal is still the thing that promotes ─────────────────────────
    // The contrast that makes the "no promotion" decision above meaningful: when the caller
    // actually leaves, the registered head-count drops and the waitlist moves. `assign_slot`
    // just registered THIRD, so seed a fresh waitlister to be promoted.
    sqlx::query("UPDATE event_registrations SET state = 'waitlisted', slot_id = NULL WHERE event_mission_id = $1 AND discord_id = $2")
        .bind(uid(&emid))
        .bind(OTHER)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE orbat_slots SET assigned_to = NULL, assigned_at = NULL WHERE id = $1")
        .bind(uid(&slot2))
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(withdraw(&emid).await, StatusCode::OK);
    assert_eq!(
        reg(&emid, OTHER).await.unwrap().1,
        "registered",
        "a real withdrawal still promotes the oldest waitlisted"
    );
    assert_eq!(
        seat(&far_slot).await.as_deref(),
        Some(DEV_USER),
        "and still does not reach another operation"
    );
}

/// T-348 — a whitespace-only `name_override` must not overwrite a real operation name.
///
/// The write is an `UPDATE`, so this uses T-317's instrument: a seeded sentinel asserted **by
/// value**, never "is not empty". `""` over `""` would look like success against the broken
/// handler, and so would a length check against `"   "`.
///
/// What makes the bug expensive is the breadth: a whitespace string is non-empty, so it defeats
/// six separate `is_empty()` fallbacks at once — `deployments.rs:97`, `dashboard.rs:79`,
/// `dashboard.rs:142`, and the SPA's `event_hub.rs:200`, `orbat_selection.rs:71`,
/// `event_manager.rs:831`. This measures the first of those through `GET /me/deployments`, which
/// is keyed to the caller's own registration rather than a global `ORDER BY`, so the assertion is
/// deterministic. HTML collapses whitespace, so the harm is not "a name with a space in it" — the
/// heading renders empty, and in the admin sidebar the row the operator would click to undo it is
/// itself unlabelled.
///
/// The last two blocks pin the directions an over-strict fix would break: `""` still clears the
/// override, and a padded-but-real name is stored byte-identical.
#[tokio::test]
async fn blank_name_override_does_not_overwrite_a_real_operation_name() {
    let _serial = DB_LOCK.lock().await;
    const SENTINEL: &str = "SENTINEL Operation Nightfall [T-348]";
    const FALLBACK: &str = "SENTINEL Mission Title [T-348]";

    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let admin = token(&app, "admin").await;

    let (st, m) = call(
        &app,
        "POST",
        "/api/v1/missions",
        &admin,
        Some(&format!(
            r#"{{"title":"{FALLBACK}","terrain":"everon","game_mode":"pve_coop","max_players":16}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "mission: {m}");
    let mission_id = m["id"].as_str().unwrap().to_string();

    let (st, e) = call(
        &app,
        "POST",
        "/api/v1/events",
        &admin,
        Some(&format!(
            r#"{{"start_time":"2027-03-03T00:00:00Z","name_override":"{SENTINEL}","max_slots":8}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "event: {e}");
    let event_id = e["id"].as_str().unwrap().to_string();

    let attach = format!(
        r#"{{"mission_id":"{mission_id}","start_time":"2027-03-03T00:00:00Z","orbat":[{{"faction":"USA","callsign":"A","squad":"Alpha","slots":[{{"role":"SL"}}]}}]}}"#
    );
    let (st, em) = call(
        &app,
        "POST",
        &format!("/api/v1/events/{event_id}/missions"),
        &admin,
        Some(&attach),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "attach: {em}");
    let emid = em["id"].as_str().unwrap().to_string();
    let (_, orbat) = call(
        &app,
        "GET",
        &format!("/api/v1/event-missions/{emid}/orbat"),
        &admin,
        None,
    )
    .await;
    let slot0 = orbat["data"][0]["slots"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let (st, r) = call(
        &app,
        "POST",
        &format!("/api/v1/event-missions/{emid}/register"),
        &admin,
        Some(&format!(r#"{{"slot_id":"{slot0}"}}"#)),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "register: {r}");

    // The stored bytes, and the name a player is actually shown.
    let uid = |s: &str| s.parse::<uuid::Uuid>().unwrap();
    let stored = async || -> String {
        sqlx::query_scalar("SELECT COALESCE(name_override, '<NULL>') FROM events WHERE id = $1")
            .bind(uid(&event_id))
            .fetch_one(&pool)
            .await
            .unwrap()
    };
    let shown = async || -> String {
        let (_, d) = call(&app, "GET", "/api/v1/me/deployments", &admin, None).await;
        d["upcoming"]
            .as_array()
            .expect("upcoming")
            .iter()
            .find(|u| u["event_id"] == event_id.as_str())
            .expect("my registration is listed")["name"]
            .as_str()
            .unwrap()
            .to_string()
    };

    assert_eq!(stored().await, SENTINEL, "baseline: the override is stored");
    assert_eq!(
        shown().await,
        SENTINEL,
        "baseline: and it is the name the deployments list shows"
    );

    // ── The bug. Pre-fix this answered 200 and both values became "   ". ──
    let (st, b) = call(
        &app,
        "PATCH",
        &format!("/api/v1/events/{event_id}"),
        &admin,
        Some(r#"{"name_override":"   "}"#),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "blank name_override: {b}");
    assert_eq!(
        b["error"],
        "name_override must not be blank — send \"\" to clear it and fall back to the mission's \
         title"
    );
    assert_eq!(stored().await, SENTINEL, "the operation name was clobbered");
    assert_eq!(shown().await, SENTINEL, "the displayed name vanished");

    // A tab and a newline are the same lie as a space.
    for blank in [r#""\t""#, r#""\n  ""#] {
        let (st, b) = call(
            &app,
            "PATCH",
            &format!("/api/v1/events/{event_id}"),
            &admin,
            Some(&format!(r#"{{"name_override":{blank}}}"#)),
        )
        .await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "blank {blank}: {b}");
        assert_eq!(stored().await, SENTINEL, "clobbered by {blank}");
    }

    // ── `""` is a real instruction, not a blank: clear the override, fall back to the
    // mission's title. This is the behaviour the six guards exist to provide, and the reason
    // trimming `"   "` down to `""` would not have been a fix — it discards the name too. ──
    let (st, b) = call(
        &app,
        "PATCH",
        &format!("/api/v1/events/{event_id}"),
        &admin,
        Some(r#"{"name_override":""}"#),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "empty clears the override: {b}");
    assert_eq!(stored().await, "");
    assert_eq!(
        shown().await,
        FALLBACK,
        "with no override the mission's title is shown"
    );

    // ── The over-rejection direction. A padded real name renders correctly today (HTML
    // collapses the padding), nothing joins on this column, and the SPA dirty-check at
    // `event_manager.rs:536` compares these bytes — so it is accepted and stored verbatim. ──
    let padded = format!("  {SENTINEL}  ");
    let (st, b) = call(
        &app,
        "PATCH",
        &format!("/api/v1/events/{event_id}"),
        &admin,
        Some(&format!(r#"{{"name_override":"{padded}"}}"#)),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "a padded real name is not refused: {b}");
    assert_eq!(stored().await, padded, "stored byte-identical, not trimmed");
    assert_eq!(shown().await, padded);

    // ── And the same three cases on the create path. ──
    for (label, value) in [("space", "  \\t "), ("newline", "\\n")] {
        let (st, b) = call(
            &app,
            "POST",
            "/api/v1/events",
            &admin,
            Some(&format!(
                r#"{{"start_time":"2027-03-04T00:00:00Z","name_override":"{value}"}}"#
            )),
        )
        .await;
        assert_eq!(
            st,
            StatusCode::BAD_REQUEST,
            "create must not accept a {label} name: {b}"
        );
    }
    let (st, b) = call(
        &app,
        "POST",
        "/api/v1/events",
        &admin,
        Some(r#"{"start_time":"2027-03-04T00:00:00Z","name_override":" Padded Create "}"#),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "create keeps a padded name: {b}");
    assert_eq!(
        b["name_override"], " Padded Create ",
        "and echoes it verbatim"
    );
    let (st, b) = call(
        &app,
        "POST",
        "/api/v1/events",
        &admin,
        Some(r#"{"start_time":"2027-03-04T00:00:00Z"}"#),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::CREATED,
        "and an absent override is fine: {b}"
    );
    // `Event::name_override` is `skip_serializing_if = "String::is_empty"`, so "no override" is
    // an absent key on the wire and the SPA's `Option<String>` sees `None`. Which is precisely
    // why a blank one is so expensive: `"   "` is non-empty, so it *is* serialised, arrives as
    // `Some("   ")`, and walks through every `.filter(|s| !s.is_empty())` on the client.
    assert!(
        b["name_override"].is_null(),
        "an empty override is omitted from the response: {b}"
    );
}

/// T-348 — `cms.rs`: a blank announcement title or body must be refused on both writes, and an
/// unrecognised status must not silently become a draft.
///
/// These cases live in `tests/events.rs` because T-348 owns this test file and no cms one; they
/// are announcement tests, not event tests.
///
/// The stakes on the PATCH are higher than the create's: `push_announcement_discord` and the
/// `push_to_discord` call in `create_announcement` both read the **stored** row, so a body
/// blanked by a PATCH is what would ship to the channel. Nothing here sets `push_to_discord` —
/// `Config::for_tests` leaves `discord_webhook_url` empty so `push_announcement` bails before any
/// request, and the guards under test return before the `INSERT`/`UPDATE` that a push reads.
#[tokio::test]
async fn blank_announcement_fields_are_refused_and_an_unknown_status_is_not_a_silent_draft() {
    let _serial = DB_LOCK.lock().await;
    const TITLE: &str = "SENTINEL Announcement [T-348]";
    const BODY: &str = "<p>SENTINEL body [T-348]</p>";

    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let admin = token(&app, "admin").await;

    let (st, a) = call(
        &app,
        "POST",
        "/api/v1/cms/announcements",
        &admin,
        Some(&format!(
            r#"{{"title":"{TITLE}","body":"{BODY}","tag":"update"}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "sentinel announcement: {a}");
    let aid = a["id"].as_str().unwrap().to_string();

    let uid = |s: &str| s.parse::<uuid::Uuid>().unwrap();
    let row = async || -> (String, String, String) {
        sqlx::query_as("SELECT title, body, status::text FROM announcements WHERE id = $1")
            .bind(uid(&aid))
            .fetch_one(&pool)
            .await
            .unwrap()
    };
    assert_eq!(
        row().await,
        (TITLE.into(), BODY.into(), "draft".into()),
        "baseline"
    );

    // ── PATCH. Pre-fix there was no guard at all here: each of these returned 200 and
    // overwrote the sentinel, `""` included. ──
    for (field, payload) in [
        ("title", r#"{"title":"   "}"#),
        ("title", r#"{"title":""}"#),
        ("title", r#"{"title":"\t\n"}"#),
        ("body", r#"{"body":"   "}"#),
        ("body", r#"{"body":""}"#),
    ] {
        let (st, b) = call(
            &app,
            "PATCH",
            &format!("/api/v1/cms/announcements/{aid}"),
            &admin,
            Some(payload),
        )
        .await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "PATCH {payload}: {b}");
        assert_eq!(b["error"], format!("{field} must not be blank"));
        assert_eq!(
            row().await,
            (TITLE.into(), BODY.into(), "draft".into()),
            "PATCH {payload} clobbered the sentinel"
        );
    }

    // A real edit still lands, and a padded title is stored verbatim — the over-rejection
    // direction, same as `name_override`.
    let (st, b) = call(
        &app,
        "PATCH",
        &format!("/api/v1/cms/announcements/{aid}"),
        &admin,
        Some(r#"{"title":" Padded Title "}"#),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "a padded real title is accepted: {b}");
    assert_eq!(row().await.0, " Padded Title ", "stored byte-identical");

    // ── Create. Pre-fix each of these returned 201 and created the announcement. ──
    for payload in [
        r#"{"title":"   ","body":"<p>real</p>","tag":"update"}"#,
        r#"{"title":"Real","body":"   ","tag":"update"}"#,
        r#"{"title":"\t","body":"<p>real</p>","tag":"update"}"#,
    ] {
        let (st, b) = call(
            &app,
            "POST",
            "/api/v1/cms/announcements",
            &admin,
            Some(payload),
        )
        .await;
        assert_eq!(st, StatusCode::BAD_REQUEST, "create {payload}: {b}");
        assert_eq!(b["error"], "title and body are required");
    }

    // ── Status. Pre-fix all three of these returned 201 with `status = draft`; the PATCH
    // rejected the same strings with a 400. ──
    for bogus in ["bogus", "PUBLISHED", "Draft"] {
        let (st, b) = call(
            &app,
            "POST",
            "/api/v1/cms/announcements",
            &admin,
            Some(&format!(
                r#"{{"title":"S","body":"<p>b</p>","tag":"update","status":"{bogus}"}}"#
            )),
        )
        .await;
        assert_eq!(
            st,
            StatusCode::BAD_REQUEST,
            "create must not silently draft {bogus}: {b}"
        );
        assert_eq!(b["error"], "invalid status");
    }

    // The three real values are honoured, and absent still means draft.
    for (payload, want) in [
        (r#""status":"archived","#, "archived"),
        (r#""status":"published","#, "published"),
        (r#""status":"draft","#, "draft"),
        ("", "draft"),
    ] {
        let (st, b) = call(
            &app,
            "POST",
            "/api/v1/cms/announcements",
            &admin,
            Some(&format!(
                r#"{{{payload}"title":"S","body":"<p>b</p>","tag":"update"}}"#
            )),
        )
        .await;
        assert_eq!(st, StatusCode::CREATED, "create {payload}: {b}");
        assert_eq!(
            b["status"], want,
            "create {payload} stored the wrong status"
        );
        // Only a published announcement gets a publish timestamp — and so only a published one
        // is eligible for the Discord push guarded by the same flag.
        assert_eq!(
            b["published_at"].is_null(),
            want != "published",
            "published_at for {payload}"
        );
    }
}

/// A zero-slot attach is refused, and the reasons it could be zero do not share one answer.
///
/// Pre-fix, every row of the table below returned **201** and materialized nothing:
/// `orbat_template_for_mission` answered `Vec::new()` for a missing version, a swallowed DB
/// error and an unreadable `orbat` alike, and `add_event_mission` committed regardless (T-227).
#[tokio::test]
async fn zero_slot_attach_is_refused_with_the_reason_it_was_zero() {
    let _serial = DB_LOCK.lock().await;
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let admin = token(&app, "admin").await;

    let mission = async |title: &str| -> String {
        let (st, m) = call(
            &app,
            "POST",
            "/api/v1/missions",
            &admin,
            Some(&format!(
                r#"{{"title":"{title}","terrain":"everon","game_mode":"pve_coop","max_players":16}}"#
            )),
        )
        .await;
        assert_eq!(st, StatusCode::CREATED, "mission {title}: {m}");
        m["id"].as_str().unwrap().to_string()
    };
    // `POST /missions` publishes a version of its own, so each case below sets
    // `current_version_id` to exactly the state it means to test.
    let publish = async |mission_id: &str, payload: &str| {
        let vid = uuid::Uuid::new_v4();
        sqlx::query(
            "INSERT INTO mission_versions (id, mission_id, semver, json_payload, editor_notes, created_by, created_at) \
             VALUES ($1, $2::uuid, '7.7.7', $3::jsonb, '', $4, now())",
        )
        .bind(vid)
        .bind(mission_id)
        .bind(payload)
        .bind(DEV_USER)
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query("UPDATE missions SET current_version_id = $1 WHERE id = $2::uuid")
            .bind(vid)
            .bind(mission_id)
            .execute(&pool)
            .await
            .unwrap();
    };
    let attach = async |mission_id: &str, orbat: &str| -> (StatusCode, Value) {
        let (st, e) = call(
            &app,
            "POST",
            "/api/v1/events",
            &admin,
            Some(r#"{"start_time":"2027-08-01T00:00:00Z"}"#),
        )
        .await;
        assert_eq!(st, StatusCode::CREATED, "event: {e}");
        let event_id = e["id"].as_str().unwrap();
        call(
            &app,
            "POST",
            &format!("/api/v1/events/{event_id}/missions"),
            &admin,
            Some(&format!(
                r#"{{"mission_id":"{mission_id}","start_time":"2027-08-01T00:00:00Z"{orbat}}}"#
            )),
        )
        .await
    };

    // ── 1. No published version. The mission is real and the request is well-formed; it is the
    // mission's STATE that cannot answer, so 409 and not 400. ──
    let m = mission("T227 no version").await;
    sqlx::query("UPDATE missions SET current_version_id = NULL WHERE id = $1::uuid")
        .bind(&m)
        .execute(&pool)
        .await
        .unwrap();
    let (st, b) = attach(&m, "").await;
    assert_eq!(st, StatusCode::CONFLICT, "missing version: {b}");
    assert!(
        b["error"]
            .as_str()
            .unwrap()
            .contains("no published version"),
        "missing version must say so: {b}"
    );

    // ── 2. `current_version_id` naming a row that does not exist. The column has NO foreign key
    // (`0001_initial_schema.sql:370`), so this is reachable — and it is OUR data that is wrong,
    // which makes it a logged 500 and not something to blame on the caller. This is the
    // in-suite half of "a database problem must never be silent"; the other half (a genuine
    // sqlx failure) is now a plain `?` and rides the same `From<sqlx::Error>` 500. ──
    let m = mission("T227 dangling").await;
    sqlx::query("UPDATE missions SET current_version_id = gen_random_uuid() WHERE id = $1::uuid")
        .bind(&m)
        .execute(&pool)
        .await
        .unwrap();
    let (st, b) = attach(&m, "").await;
    assert_eq!(
        st,
        StatusCode::INTERNAL_SERVER_ERROR,
        "dangling version id must not read as 'no ORBAT': {b}"
    );

    // ── 3. An `orbat` that cannot be read. Pre-fix this did not merely vanish — it fell through
    // to the editor-derived ORBAT, so a DIFFERENT seating plan than the one authored could be
    // materialized under a 201. 400, naming the payload, with serde's message attached. ──
    let m = mission("T227 unreadable").await;
    publish(
        &m,
        r#"{"orbat":[{"squad":"Alpha","slots":"not-an-array"}]}"#,
    )
    .await;
    let (st, b) = attach(&m, "").await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "unreadable orbat: {b}");
    assert!(
        b["error"].as_str().unwrap().contains("`orbat`"),
        "the message must name the payload field: {b}"
    );
    assert!(
        b["details"]["orbat"].is_string(),
        "serde's own reason must survive to the client: {b}"
    );

    // ── 4. A perfectly VALID payload that seats nobody — T-368's shape, and the one that needs
    // no mistake at all. `input.orbat.is_empty()` cannot see it, because the squad list is not
    // empty; only the slot count is. ──
    let m = mission("T227 valid but empty").await;
    publish(
        &m,
        r#"{"orbat":[{"faction":"USA","squad":"Alpha","slots":[]}]}"#,
    )
    .await;
    let (st, b) = attach(&m, "").await;
    assert_eq!(st, StatusCode::CONFLICT, "valid payload, no slots: {b}");
    assert!(
        b["error"].as_str().unwrap().contains("no slots"),
        "must say the ORBAT is seatless: {b}"
    );

    // ── 5. The other door: an `orbat` on the REQUEST with no slots. Same catastrophe, and this
    // one is the caller's payload, so 400. ──
    let m = mission("T227 request empty").await;
    let (st, b) = attach(
        &m,
        r#","orbat":[{"faction":"USA","squad":"Alpha","slots":[]}]"#,
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "request orbat, no slots: {b}");

    // ── 6. Control — one real slot still attaches, and materializes exactly one row. Without
    // this the whole test would pass by refusing everything. ──
    let m = mission("T227 control").await;
    let (st, b) = attach(
        &m,
        r#","orbat":[{"faction":"USA","callsign":"A","squad":"T227 Alpha","slots":[{"role":"SL"}]}]"#,
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "control must still attach: {b}");
    let emid = b["id"].as_str().unwrap();
    let seats: i64 =
        sqlx::query_scalar("SELECT count(*) FROM orbat_slots WHERE event_mission_id = $1::uuid")
            .bind(emid)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(seats, 1, "the control attach must materialize its slot");
}

/// A seatless operation is not an unlimited one, and `events.max_slots` is now a real bound.
///
/// The registration guard read `capacity > 0 && registered >= capacity` — at `capacity == 0`
/// that clause does not protect the comparison, it switches it off, so every seatless
/// registration was accepted as `registered` without limit (T-227). `add_event_mission` now
/// refuses to create a zero-slot mission, so the zero-capacity row here is **seeded directly**,
/// which is also how the pre-fix rows and the dev seed arrive.
#[tokio::test]
async fn a_seatless_operation_refuses_registration_and_max_slots_caps_the_event() {
    let _serial = DB_LOCK.lock().await;
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let admin = token(&app, "admin").await;
    let enl = token(&app, "enlisted").await;

    let mission = async |title: &str| -> String {
        let (_, m) = call(
            &app,
            "POST",
            "/api/v1/missions",
            &admin,
            Some(&format!(
                r#"{{"title":"{title}","terrain":"everon","game_mode":"pve_coop","max_players":16}}"#
            )),
        )
        .await;
        m["id"].as_str().unwrap().to_string()
    };
    let event = async |max_slots: i64| -> String {
        let (st, e) = call(
            &app,
            "POST",
            "/api/v1/events",
            &admin,
            Some(&format!(
                r#"{{"start_time":"2027-09-01T00:00:00Z","max_slots":{max_slots}}}"#
            )),
        )
        .await;
        assert_eq!(st, StatusCode::CREATED, "event: {e}");
        e["id"].as_str().unwrap().to_string()
    };

    // ══ 1. ZERO CAPACITY REFUSES, RATHER THAN REGISTERING FOREVER ═════════════════════════
    // Seeded past the attach guard, exactly as a row written before this fix would be.
    let mission_id = mission("T227 seatless").await;
    let ev = event(0).await;
    let seatless: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO event_missions (event_id, mission_id, start_time, created_at, updated_at) \
         VALUES ($1::uuid, $2::uuid, '2027-09-01T00:00:00Z', now(), now()) RETURNING id",
    )
    .bind(&ev)
    .bind(&mission_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    let (st, b) = call(
        &app,
        "POST",
        &format!("/api/v1/event-missions/{seatless}/register"),
        &enl,
        Some(r#"{"slot_id":""}"#),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::CONFLICT,
        "a seatless operation must refuse, not register: {b}"
    );
    // Refused BEFORE any write — no registration row may exist for a bench sign-up that failed.
    let rows: i64 =
        sqlx::query_scalar("SELECT count(*) FROM event_registrations WHERE event_mission_id = $1")
            .bind(seatless)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(rows, 0, "the refusal must not have written a registration");

    // ══ 2. `max_slots` BOUNDS THE WHOLE OPERATION ═════════════════════════════════════════
    // 4 seats in the ORBAT but a cap of 2. Pre-fix `max_slots` was validated on create,
    // editable via PATCH, rendered by the SPA as "{n} slot cap" — and read by nothing.
    let mission_id = mission("T227 capped").await;
    let ev = event(2).await;
    let (st, em) = call(
        &app,
        "POST",
        &format!("/api/v1/events/{ev}/missions"),
        &admin,
        Some(&format!(
            r#"{{"mission_id":"{mission_id}","start_time":"2027-09-01T00:00:00Z","orbat":[{{"faction":"USA","callsign":"A","squad":"T227 Capped","slots":[{{"role":"R0"}},{{"role":"R1"}},{{"role":"R2"}},{{"role":"R3"}}]}}]}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "attach: {em}");
    let emid = em["id"].as_str().unwrap().to_string();

    // Two seeded strangers fill the cap. dev-login is one fixed identity, so the other
    // occupants are seeded and only the caller under test goes through the handler — the same
    // idiom the G7b race tests use. `arma_id` has its own unique index.
    for id in [OTHER, THIRD] {
        common::seed_user(&pool, id, "Seeded", &arma(id), "enlisted").await;
        sqlx::query(
            "INSERT INTO event_registrations (event_mission_id, discord_id, slot_id, state) \
             VALUES ($1::uuid, $2, NULL, 'registered') \
             ON CONFLICT (event_mission_id, discord_id) DO UPDATE SET state = 'registered'",
        )
        .bind(&emid)
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    }

    // A free seat exists, so this refusal is the EVENT cap talking and not the ORBAT's.
    let (st, orbat) = call(
        &app,
        "GET",
        &format!("/api/v1/event-missions/{emid}/orbat"),
        &enl,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "orbat: {orbat}");
    let slot0 = orbat["data"][0]["slots"][0]["id"].as_str().unwrap();
    assert!(
        orbat["data"][0]["slots"][0]["assigned_to"].is_null(),
        "the seat under test must be free: {orbat}"
    );
    let (st, b) = call(
        &app,
        "POST",
        &format!("/api/v1/event-missions/{emid}/register"),
        &enl,
        Some(&format!(r#"{{"slot_id":"{slot0}"}}"#)),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::CONFLICT,
        "max_slots 2 with 2 registered must refuse a third person: {b}"
    );
    assert!(
        b["error"].as_str().unwrap().contains("full"),
        "the cap refusal must say the operation is full: {b}"
    );
    // And it refused before writing: the seat is still free and unclaimed.
    let taken: Option<String> =
        sqlx::query_scalar("SELECT assigned_to FROM orbat_slots WHERE id = $1::uuid")
            .bind(slot0)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(taken, None, "a capped-out refusal must not claim the seat");

    // Raising the cap lets the same request through — proving the refusal was the cap and not
    // some unrelated gate, and that `max_slots` is now genuinely the value being read.
    let (st, b) = call(
        &app,
        "PATCH",
        &format!("/api/v1/events/{ev}"),
        &admin,
        Some(r#"{"max_slots":3}"#),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "raise the cap: {b}");
    let (st, b) = call(
        &app,
        "POST",
        &format!("/api/v1/event-missions/{emid}/register"),
        &enl,
        Some(&format!(r#"{{"slot_id":"{slot0}"}}"#)),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "cap of 3 must admit the third: {b}");
    assert_eq!(b["state"], "registered");

    // Already inside the operation, so a second mission of the SAME event must not consume a
    // second unit of an attendance cap that is now exactly full. A distinct mission, because
    // `idx_event_mission` is unique on `(event_id, mission_id)`.
    let mission_2 = mission("T227 capped second").await;
    let (st, em2) = call(
        &app,
        "POST",
        &format!("/api/v1/events/{ev}/missions"),
        &admin,
        Some(&format!(
            r#"{{"mission_id":"{mission_2}","start_time":"2027-09-02T00:00:00Z","orbat":[{{"faction":"USA","callsign":"B","squad":"T227 Second","slots":[{{"role":"R0"}}]}}]}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "second attach: {em2}");
    let emid2 = em2["id"].as_str().unwrap().to_string();
    let (st, orbat2) = call(
        &app,
        "GET",
        &format!("/api/v1/event-missions/{emid2}/orbat"),
        &enl,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "orbat2: {orbat2}");
    let slot_b = orbat2["data"][0]["slots"][0]["id"].as_str().unwrap();
    let (st, b) = call(
        &app,
        "POST",
        &format!("/api/v1/event-missions/{emid2}/register"),
        &enl,
        Some(&format!(r#"{{"slot_id":"{slot_b}"}}"#)),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::OK,
        "an attendee already counted must not be refused by the cap again: {b}"
    );
}

/// T-260 — events carry per-event `server_id` + `modpack_id`.
///
/// Before: create/get/patch had zero such fields; Hub used global `/modpacks/current`.
/// After: create binds them, hub GET echoes them, PATCH can set/clear, unknown ids 400,
/// and a create that omits them leaves the keys absent (NULL in DB — safe for old rows).
#[tokio::test]
async fn event_server_and_modpack_binding() {
    let _serial = DB_LOCK.lock().await;
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let admin = token(&app, "admin").await;
    let enl = token(&app, "enlisted").await;

    // Seed a real server + modpack the advisory checks can accept. Private ids so concurrent
    // suites cannot collide (T-334 pattern).
    let modpack_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO modpacks (name, version, total_size_bytes, workshop_url, is_current, created_at) \
         VALUES ('T260 Pack', '9.9.9', 42, 'https://example.invalid/t260', false, now()) \
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("seed modpack");
    let server_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO servers (name, ip, port, required_modpack_id, is_active) \
         VALUES ('T260 Srv', '127.0.0.1'::inet, 2260, $1, true) RETURNING id",
    )
    .bind(modpack_id)
    .fetch_one(&pool)
    .await
    .expect("seed server");

    // 1. Create WITHOUT binding — keys absent on the wire (skip_serializing_if None).
    let (st, e) = call(
        &app,
        "POST",
        "/api/v1/events",
        &admin,
        Some(r#"{"start_time":"2027-06-01T19:00:00Z","name_override":"T260 unbound"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "unbound create: {e}");
    assert!(
        e.get("server_id").is_none(),
        "unbound create must omit server_id, got {e}"
    );
    assert!(
        e.get("modpack_id").is_none(),
        "unbound create must omit modpack_id, got {e}"
    );
    let unbound_id = e["id"].as_str().unwrap().to_string();

    // 2. Create WITH binding — create + hub GET echo both ids.
    let body = format!(
        r#"{{"start_time":"2027-06-02T19:00:00Z","name_override":"T260 bound","server_id":"{server_id}","modpack_id":"{modpack_id}"}}"#
    );
    let (st, e) = call(&app, "POST", "/api/v1/events", &admin, Some(&body)).await;
    assert_eq!(st, StatusCode::CREATED, "bound create: {e}");
    assert_eq!(
        e["server_id"].as_str().unwrap(),
        server_id.to_string(),
        "create must echo server_id: {e}"
    );
    assert_eq!(
        e["modpack_id"].as_str().unwrap(),
        modpack_id.to_string(),
        "create must echo modpack_id: {e}"
    );
    let bound_id = e["id"].as_str().unwrap().to_string();

    let (st, hub) = call(
        &app,
        "GET",
        &format!("/api/v1/events/{bound_id}"),
        &enl,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "hub: {hub}");
    assert_eq!(
        hub["server_id"].as_str().unwrap(),
        server_id.to_string(),
        "hub must carry per-event server_id (not global current): {hub}"
    );
    assert_eq!(
        hub["modpack_id"].as_str().unwrap(),
        modpack_id.to_string(),
        "hub must carry per-event modpack_id: {hub}"
    );

    // 3. PATCH set on the unbound event, then clear with explicit null.
    let patch = format!(r#"{{"server_id":"{server_id}","modpack_id":"{modpack_id}"}}"#);
    let (st, e) = call(
        &app,
        "PATCH",
        &format!("/api/v1/events/{unbound_id}"),
        &admin,
        Some(&patch),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "patch set: {e}");
    assert_eq!(e["server_id"].as_str().unwrap(), server_id.to_string());
    assert_eq!(e["modpack_id"].as_str().unwrap(), modpack_id.to_string());

    let (st, e) = call(
        &app,
        "PATCH",
        &format!("/api/v1/events/{unbound_id}"),
        &admin,
        Some(r#"{"server_id":null,"modpack_id":null}"#),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "patch clear: {e}");
    assert!(
        e.get("server_id").is_none(),
        "explicit null must clear server_id: {e}"
    );
    assert!(
        e.get("modpack_id").is_none(),
        "explicit null must clear modpack_id: {e}"
    );

    // 4. Unknown ids are 400 — not silent store (no FK to catch them).
    let ghost = "00000000-0000-4000-a000-000000002260";
    let (st, b) = call(
        &app,
        "POST",
        "/api/v1/events",
        &admin,
        Some(&format!(
            r#"{{"start_time":"2027-06-03T19:00:00Z","server_id":"{ghost}"}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "unknown server must 400: {b}");
    assert!(
        b["error"].as_str().unwrap_or("").contains("server_id"),
        "error must name server_id: {b}"
    );
    let (st, b) = call(
        &app,
        "POST",
        "/api/v1/events",
        &admin,
        Some(&format!(
            r#"{{"start_time":"2027-06-03T19:00:00Z","modpack_id":"{ghost}"}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "unknown modpack must 400: {b}");
    assert!(
        b["error"].as_str().unwrap_or("").contains("modpack_id"),
        "error must name modpack_id: {b}"
    );

    // 5. Columns exist and are NULL on a fresh row — migration is safe for existing events.
    let nulls: (Option<uuid::Uuid>, Option<uuid::Uuid>) =
        sqlx::query_as("SELECT server_id, modpack_id FROM events WHERE id = $1::uuid")
            .bind(&unbound_id)
            .fetch_one(&pool)
            .await
            .expect("read columns");
    assert_eq!(nulls, (None, None), "cleared row must store NULL,NULL");
}

/// T-284 — `DELETE …/slots/:id/assign` frees a claimed seat for leader/admin, and the dead
/// `events.match_id` column is gone (link is `matches.event_id`).
///
/// Pre-fix: the handler existed and was routed, but nothing in the SPA called it, and every
/// Event SELECT still projected a forever-NULL `match_id`. This test is the API half of that
/// cure — assign → clear → both `orbat_slots.assigned_to` and `event_registrations.slot_id`
/// are null, enlisted without a reserve is forbidden, and `information_schema` shows no
/// `events.match_id`.
#[tokio::test]
async fn clear_slot_frees_assignment_and_events_have_no_match_id() {
    let _serial = DB_LOCK.lock().await;
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let admin = token(&app, "admin").await;
    let leader = token(&app, "leader").await;
    let enl = token(&app, "enlisted").await;
    common::seed_user(&pool, OTHER, "Other", &arma(OTHER), "enlisted").await;

    let (st, m) = call(
        &app,
        "POST",
        "/api/v1/missions",
        &admin,
        Some(
            r#"{"title":"T284 Clear","terrain":"everon","game_mode":"pve_coop","max_players":16}"#,
        ),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "mission: {m}");
    let mission_id = m["id"].as_str().unwrap().to_string();

    let (st, e) = call(
        &app,
        "POST",
        "/api/v1/events",
        &admin,
        Some(r#"{"start_time":"2027-07-01T00:00:00Z","name_override":"T284 Op"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "event: {e}");
    let event_id = e["id"].as_str().unwrap().to_string();
    // Wire must not carry the dropped column (absent, not null).
    assert!(
        e.get("match_id").is_none(),
        "create response must omit match_id: {e}"
    );

    let attach = format!(
        r#"{{"mission_id":"{mission_id}","start_time":"2027-07-01T00:00:00Z","orbat":[{{"faction":"USA","callsign":"A","squad":"Alpha","slots":[{{"role":"SL"}},{{"role":"RTO"}}]}}]}}"#
    );
    let (st, em) = call(
        &app,
        "POST",
        &format!("/api/v1/events/{event_id}/missions"),
        &admin,
        Some(&attach),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "attach: {em}");
    let emid = em["id"].as_str().unwrap().to_string();

    let (st, orbat) = call(
        &app,
        "GET",
        &format!("/api/v1/event-missions/{emid}/orbat"),
        &admin,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let slot0 = orbat["data"][0]["slots"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Admin fills the seat (same path the SPA Assign picker uses).
    let (st, a) = call(
        &app,
        "PUT",
        &format!("/api/v1/event-missions/{emid}/slots/{slot0}/assign"),
        &admin,
        Some(&format!(r#"{{"discord_id":"{OTHER}"}}"#)),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "assign: {a}");
    let held: Option<String> =
        sqlx::query_scalar("SELECT assigned_to FROM orbat_slots WHERE id = $1::uuid")
            .bind(&slot0)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(held.as_deref(), Some(OTHER));
    let reg_slot: Option<uuid::Uuid> = sqlx::query_scalar(
        "SELECT slot_id FROM event_registrations WHERE event_mission_id = $1::uuid AND discord_id = $2",
    )
    .bind(&emid)
    .bind(OTHER)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        reg_slot.map(|u| u.to_string()).as_deref(),
        Some(slot0.as_str())
    );

    // Enlisted never reaches the handler — LeaderUser extractor rejects first
    // (same gate as assign_slot / reserve).
    let (st, r) = call(
        &app,
        "DELETE",
        &format!("/api/v1/event-missions/{emid}/slots/{slot0}/assign"),
        &enl,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN, "enlisted clear must 403: {r}");
    assert_eq!(r["error"], "insufficient role", "enlisted message: {r}");

    // Leader without a squad reserve cannot clear (same can_manage_squad gate as assign).
    let (st, r) = call(
        &app,
        "DELETE",
        &format!("/api/v1/event-missions/{emid}/slots/{slot0}/assign"),
        &leader,
        None,
    )
    .await;
    assert_eq!(
        st,
        StatusCode::FORBIDDEN,
        "unreserved leader clear must 403: {r}"
    );
    assert_eq!(
        r["error"], "reserve this squad to manage its slots",
        "forbidden message: {r}"
    );

    // Admin clears — both sides of the seat invariant go null.
    let (st, c) = call(
        &app,
        "DELETE",
        &format!("/api/v1/event-missions/{emid}/slots/{slot0}/assign"),
        &admin,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "clear: {c}");
    assert_eq!(c["cleared"], true);
    let held: Option<String> =
        sqlx::query_scalar("SELECT assigned_to FROM orbat_slots WHERE id = $1::uuid")
            .bind(&slot0)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(held, None, "clear_slot must null assigned_to");
    let reg_slot: Option<uuid::Uuid> = sqlx::query_scalar(
        "SELECT slot_id FROM event_registrations WHERE event_mission_id = $1::uuid AND discord_id = $2",
    )
    .bind(&emid)
    .bind(OTHER)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(reg_slot, None, "clear_slot must null registration.slot_id");

    // Migration 0013: column gone. A SELECT that still named it would 500 on FromRow.
    let still_there: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.columns \
         WHERE table_schema = 'public' AND table_name = 'events' AND column_name = 'match_id'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(still_there, 0, "events.match_id must be dropped");

    let (st, get) = call(
        &app,
        "GET",
        &format!("/api/v1/events/{event_id}"),
        &admin,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert!(
        get.get("match_id").is_none(),
        "GET event must omit match_id: {get}"
    );
}

/// T-332 — PATCH clears briefing/banner via `""`, and a mission can be re-attached after detach.
///
/// Before: empty-string clear worked by accident (undocumented); duplicate attach of a still-
/// attached mission 500'd on `idx_event_mission`; after detach there was no FE caller for
/// `POST /events/:id/missions` (covered by the FE Class-R). This IT pins the BE contracts.
#[tokio::test]
async fn patch_clears_briefing_banner_and_mission_reattach_works() {
    let _serial = DB_LOCK.lock().await;
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let admin = token(&app, "admin").await;

    // ── 1. Create with briefing + banner, then clear both with "". ──
    let (st, e) = call(
        &app,
        "POST",
        "/api/v1/events",
        &admin,
        Some(concat!(
            r#"{"start_time":"2027-11-01T19:00:00Z","name_override":"T332 clear","#,
            r#""briefing":"ops brief","banner_image_url":"https://example.invalid/t332.png","max_slots":8}"#,
        )),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "create: {e}");
    assert_eq!(e["briefing"], "ops brief");
    assert_eq!(e["banner_image_url"], "https://example.invalid/t332.png");
    let event_id = e["id"].as_str().unwrap().to_string();

    // Omitting the keys must leave them alone (perturbation: treating absent as clear).
    let (st, e) = call(
        &app,
        "PATCH",
        &format!("/api/v1/events/{event_id}"),
        &admin,
        Some(r#"{"max_slots":9}"#),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "patch other field: {e}");
    assert_eq!(
        e["briefing"], "ops brief",
        "absent briefing must not clear: {e}"
    );
    assert_eq!(
        e["banner_image_url"], "https://example.invalid/t332.png",
        "absent banner must not clear: {e}"
    );
    assert_eq!(e["max_slots"], 9);

    // Blessed clear: empty string.
    let (st, e) = call(
        &app,
        "PATCH",
        &format!("/api/v1/events/{event_id}"),
        &admin,
        Some(r#"{"briefing":"","banner_image_url":""}"#),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "patch clear: {e}");
    assert!(
        e.get("briefing").is_none() || e["briefing"] == "",
        "empty briefing must clear (omitted or \"\"): {e}"
    );
    assert!(
        e.get("banner_image_url").is_none() || e["banner_image_url"] == "",
        "empty banner must clear (omitted or \"\"): {e}"
    );
    let briefing: String =
        sqlx::query_scalar("SELECT COALESCE(briefing, '') FROM events WHERE id = $1::uuid")
            .bind(&event_id)
            .fetch_one(&pool)
            .await
            .expect("briefing");
    let banner: String =
        sqlx::query_scalar("SELECT COALESCE(banner_image_url, '') FROM events WHERE id = $1::uuid")
            .bind(&event_id)
            .fetch_one(&pool)
            .await
            .expect("banner");
    assert_eq!(briefing, "", "DB briefing must be empty after \"\" clear");
    assert_eq!(banner, "", "DB banner must be empty after \"\" clear");

    // ── 2. Attach → duplicate 409 → detach → re-attach 201. ──
    let (st, m) = call(
        &app,
        "POST",
        "/api/v1/missions",
        &admin,
        Some(
            r#"{"title":"T332 Mission","terrain":"everon","game_mode":"pve_coop","max_players":16}"#,
        ),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "mission: {m}");
    let mission_id = m["id"].as_str().unwrap().to_string();

    let attach_body = format!(
        r#"{{"mission_id":"{mission_id}","start_time":"2027-11-01T19:00:00Z","orbat":[{{"faction":"USA","callsign":"A","squad":"T332","slots":[{{"role":"SL"}}]}}]}}"#
    );
    let (st, em) = call(
        &app,
        "POST",
        &format!("/api/v1/events/{event_id}/missions"),
        &admin,
        Some(&attach_body),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "first attach: {em}");
    let emid = em["id"].as_str().unwrap().to_string();

    let (st, dup) = call(
        &app,
        "POST",
        &format!("/api/v1/events/{event_id}/missions"),
        &admin,
        Some(&attach_body),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::CONFLICT,
        "duplicate attach must 409 (perturbation: drop unique map → 500): {dup}"
    );
    assert!(
        dup["error"]
            .as_str()
            .unwrap_or("")
            .contains("already attached"),
        "duplicate attach message: {dup}"
    );

    let (st, _) = call(
        &app,
        "DELETE",
        &format!("/api/v1/events/{event_id}/missions/{emid}"),
        &admin,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NO_CONTENT, "detach: {st}");

    let (st, em2) = call(
        &app,
        "POST",
        &format!("/api/v1/events/{event_id}/missions"),
        &admin,
        Some(&attach_body),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::CREATED,
        "re-attach after detach must succeed: {em2}"
    );
    assert_ne!(
        em2["id"].as_str().unwrap_or(""),
        emid.as_str(),
        "re-attach must mint a new event_mission id"
    );
}

/// T-495 — `GET /api/v1/members` must honour `offset` (not only return an array).
///
/// T-412 shipped handler LIMIT/OFFSET + `{data,total,limit,offset}` and a pure-oracle unit
/// test, but the live IT path in this suite still only did `assert!(mem["data"].is_array())`.
/// That stays green if OFFSET is deleted. Cure: seed 25 suite-private members whose
/// usernames sort under a unique `q` prefix, request `offset=20` (default limit 20), and
/// assert the window starts at member index 20 plus the envelope fields.
///
/// Perturbation: drop `OFFSET` / hard-code `LIMIT 20` with no bind → page still length 20 but
/// first username is `t495_user_00` (or total/offset mismatch) → assert fails.
#[tokio::test]
async fn members_list_honours_offset_pagination() {
    let _serial = DB_LOCK.lock().await;
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    const N: usize = 25;
    const PREFIX: &str = "t495_user_";
    // Suite-private discord ids (T-495 range) — must not collide with OTHER/THIRD / other suites.
    for i in 0..N {
        let discord_id = format!("000000000000495{i:03}");
        let username = format!("{PREFIX}{i:02}");
        common::seed_user(&pool, &discord_id, &username, &arma(&discord_id), "enlisted").await;
    }

    let leader = token(&app, "leader").await;

    // Page 0 — member at index 20 must be invisible.
    let (st, page0) = call(
        &app,
        "GET",
        &format!("/api/v1/members?q={PREFIX}&limit=20&offset=0"),
        &leader,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "members page0: {page0}");
    let data0 = page0["data"]
        .as_array()
        .unwrap_or_else(|| panic!("page0 missing data array: {page0}"));
    assert_eq!(data0.len(), 20, "default first page size: {page0}");
    assert_eq!(page0["total"], N as i64, "filtered total must be seeded N: {page0}");
    assert_eq!(page0["limit"], 20, "envelope limit: {page0}");
    assert_eq!(page0["offset"], 0, "envelope offset: {page0}");
    let names0: Vec<&str> = data0
        .iter()
        .map(|m| m["username"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(names0.first().copied(), Some("t495_user_00"));
    assert!(
        !names0.contains(&"t495_user_20"),
        "offset=0 must not include member index 20: {names0:?}"
    );

    // Page at offset=20 — member 21 (0-based index 20) is the first row.
    let (st, page_off) = call(
        &app,
        "GET",
        &format!("/api/v1/members?q={PREFIX}&limit=20&offset=20"),
        &leader,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "members offset=20: {page_off}");
    let data_off = page_off["data"]
        .as_array()
        .unwrap_or_else(|| panic!("offset page missing data array: {page_off}"));
    assert_eq!(
        data_off.len(),
        5,
        "25 seeded − offset 20 → 5 rows: {page_off}"
    );
    assert_eq!(
        page_off["total"], N as i64,
        "total must stay N across pages: {page_off}"
    );
    assert_eq!(page_off["limit"], 20, "envelope limit: {page_off}");
    assert_eq!(page_off["offset"], 20, "envelope offset: {page_off}");
    assert_eq!(
        data_off[0]["username"].as_str(),
        Some("t495_user_20"),
        "offset=20 must surface member at index 20 first: {page_off}"
    );
    assert_eq!(
        data_off[4]["username"].as_str(),
        Some("t495_user_24"),
        "last row of the window: {page_off}"
    );
}
