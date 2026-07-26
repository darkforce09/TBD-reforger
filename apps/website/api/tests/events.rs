//! Event + ORBAT + registration lifecycle. dev-login is a single fixed identity, so
//! the multi-actor conflict paths (taken slot, reserved squad) are seeded via direct
//! SQL for a second user id, then driven through the real handler — deterministically
//! exercising the G7b race-loser code (conditional claim reject + reservation guard).
//! Skips without `TEST_DATABASE_URL`.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use website_api::config::Config;
use website_api::state::AppState;
use website_api::{app, db};

const OTHER: &str = "000000000000000002";
/// A third seeded identity — the one that must stay on the waitlist while someone else
/// moves between seats (T-324).
const THIRD: &str = "000000000000000003";
/// The identity `dev-login` mints for every role (`handlers::dev::DEV_USER_ID`).
const DEV_USER: &str = "000000000000000001";

async fn boot() -> Option<(Router, PgPool)> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let app = app::router(AppState::new(
        pool.clone(),
        Config::for_tests(url, "events-secret"),
    ));
    Some((app, pool))
}

async fn token(app: &Router, role: &str) -> String {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/auth/dev-login?role={role}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let loc = resp.headers()[header::LOCATION].to_str().unwrap();
    loc.split_once('#')
        .unwrap()
        .1
        .split('&')
        .find_map(|p| p.strip_prefix("access_token="))
        .unwrap()
        .to_string()
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
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let admin = token(&app, "admin").await;
    let leader = token(&app, "leader").await;
    let enl = token(&app, "enlisted").await;
    // A distinct second user for the seeded conflict paths.
    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'Other', 'other', '', '', '', 'enlisted', false, '', now(), now()) ON CONFLICT (discord_id) DO NOTHING",
    )
    .bind(OTHER)
    .execute(&pool)
    .await
    .unwrap();

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
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let admin = token(&app, "admin").await;
    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'Other', 'other', '', '', '', 'enlisted', false, '', now(), now()) ON CONFLICT (discord_id) DO NOTHING",
    )
    .bind(OTHER)
    .execute(&pool)
    .await
    .unwrap();

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

    // Multi-seat cleanup. When T-318 was written, `register` itself could put the caller in this
    // state — claim slot0, claim slot1, hold both — and this block asserted `held == 2` as a
    // known defect. T-324 closed that door (see `register_moves_the_caller_s_seat`), so the only
    // remaining source is rows minted before the fix. Those still exist in the wild, so the
    // recovery path stays under test: the state is now seeded directly rather than provoked, and
    // withdraw must still free *every* seat the caller holds, not just the one their registration
    // names.
    assert_eq!(register(&emid, Some(&claim)).await, StatusCode::OK);
    sqlx::query("UPDATE orbat_slots SET assigned_to = $1, assigned_at = now() WHERE id = $2")
        .bind(DEV_USER)
        .bind(slot1.parse::<uuid::Uuid>().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    let held: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM orbat_slots WHERE event_mission_id = $1 AND assigned_to = $2",
    )
    .bind(emid.parse::<uuid::Uuid>().unwrap())
    .bind(DEV_USER)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(held, 2, "legacy two-seat state seeded");
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
        "withdraw must free every seat the caller holds here"
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
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let admin = token(&app, "admin").await;
    for id in [OTHER, THIRD] {
        // `arma_id` carries its own unique index, so seeded users cannot all share the empty
        // string the way one of them can.
        sqlx::query(
            "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, is_banned, ban_reason, created_at, updated_at) \
             VALUES ($1, 'Seeded', 'seeded', '', $2, '', 'enlisted', false, '', now(), now()) ON CONFLICT (discord_id) DO NOTHING",
        )
        .bind(id)
        .bind(format!("t324-{id}"))
        .execute(&pool)
        .await
        .unwrap();
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
