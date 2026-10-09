//! Claiming, moving and giving up an ORBAT seat: the race-loser branches, the bad-body
//! rejections that must not strand a seat, and the invariant that one caller holds at most one
//! seat per event-mission and the registration names it. Skips without `TEST_DATABASE_URL`.
//!
//! dev-login is a single fixed identity, so the multi-actor paths (taken slot, reserved squad,
//! a waitlister who must not be promoted) are seeded by direct SQL for the second and third
//! actor and then driven through the real handler.

use crate::events_support;

use axum::http::StatusCode;
use events_support::{DB_LOCK, DEV_USER, OTHER, arma, boot, call, token};

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
    events_support::seed_member(&pool, OTHER, "Other", &arma(OTHER), "enlisted").await;

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

    // Race loser: slot1 held by the other user → this claim loses the WHERE → 409.
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

/// The registration upsert must not orphan an ORBAT seat that nobody can free.
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
    events_support::seed_member(&pool, OTHER, "Other", &arma(OTHER), "enlisted").await;

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
    let emid = mk_em(
        "Registration Op",
        "Alpha",
        r#"{"role":"SL"},{"role":"RTO"}"#,
    )
    .await;
    let other_emid = mk_em("Registration Op B", "Bravo", r#"{"role":"SL"}"#).await;

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
    for (label, body, status) in [
        (
            "malformed json",
            Some(r#"{"slot_id":"#),
            StatusCode::BAD_REQUEST,
        ),
        ("empty object", Some("{}"), StatusCode::BAD_REQUEST),
        ("wrong json type", Some("[]"), StatusCode::BAD_REQUEST),
        (
            "no body / no content-type",
            None,
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ),
    ] {
        assert_eq!(
            register(&emid, body).await,
            status,
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

    // That request also stands the seat down. Blanking the registration while leaving
    // `assigned_to` naming the caller is the orphan factory, and the convenient way to
    // reproduce the shape below.
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
    // recoverable by their occupant; a withdraw that looks the seat up through the column that
    // is blank and then deletes the row anyway makes the state terminal.
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
    assert_eq!(
        reg_slot(&emid).await,
        Some(None),
        "withdrawal preserves the signup with no seat"
    );
    sqlx::query(
        "WITH removed_history AS (DELETE FROM event_registration_history WHERE registration_id IN (
        SELECT id FROM event_registrations WHERE event_mission_id = $1 AND discord_id = $2))
        DELETE FROM event_registrations WHERE event_mission_id = $1 AND discord_id = $2",
    )
    .bind(emid.parse::<uuid::Uuid>().unwrap())
    .bind(DEV_USER)
    .execute(&pool)
    .await
    .unwrap();
    assert!(
        reg_slot(&emid).await.is_none(),
        "explicitly seed a legacy orphan with no signup"
    );
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

    // Retried withdrawal is idempotent against the retained historical signup.
    assert_eq!(withdraw(&emid).await, StatusCode::OK);

    // No multi-seat seed: a partial unique on
    // (event_mission_id, assigned_to) WHERE assigned_to IS NOT NULL makes the
    // two-seat shape unreachable (and the index cannot be DEFERRABLE).
    // The recovery intent lives in Part 2 above — orphan seats freed via
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
