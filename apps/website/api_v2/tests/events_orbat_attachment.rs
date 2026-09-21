//! Attaching an ORBAT to an event and what its seats then mean: how the seating plan is
//! grouped for display, why an attach that would seat nobody is refused with the reason it was
//! seatless, how a zero-capacity or capped operation answers a registration, and what clearing
//! an assignment leaves behind. Skips without `TEST_DATABASE_URL`.

use axum::http::StatusCode;
use events_support::{DB_LOCK, DEV_USER, OTHER, THIRD, arma, boot, call, token};
use serde_json::Value;

mod common;
mod events_support;

/// Two factions fielding a squad of the same name must render as two cards.
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
        Some(r#"{"title":"Attachment Factions","terrain":"everon","game_mode":"pve_coop","max_players":16}"#),
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

/// A zero-slot attach is refused, and the reasons it could be zero do not share one answer.
///
/// Pre-fix, every row of the table below returned **201** and materialized nothing:
/// `orbat_template_for_mission` answered `Vec::new()` for a missing version, a swallowed DB
/// error and an unreadable `orbat` alike, and `add_event_mission` committed regardless.
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
    let m = mission("Attach no version").await;
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

    // ── 2. `current_version_id` naming a row that does not exist.
    //
    // A dangling `missions.current_version_id` is not reachable. `0018_foreign_keys.sql` adds
    // `missions_current_version_id_fkey … ON DELETE SET NULL`, so Postgres refuses the write
    // that would create the dangling pointer. Asserting that the attach answers **500** on such
    // a row — "it is OUR data that is wrong, so it must not be quiet" — would mean asserting
    // that a defect still exists.
    //
    // So the assertion moves down a layer to the thing that now guarantees it: the UPDATE is
    // **rejected with SQLSTATE 23503**. That is strictly stronger — the old test proved the
    // handler survives bad data, this proves the bad data cannot be written. The handler's
    // 500 arm (`orbat_template_for_mission`, `operations/handlers/event_mission_attachment.rs`)
    // is deliberately left in place as defence in depth: it still covers a row that predates
    // this migration on a database restored from an old dump, and constraint 18's backfill
    // NULLs exactly those.
    let m = mission("Attach dangling").await;
    let err = sqlx::query(
        "UPDATE missions SET current_version_id = gen_random_uuid() WHERE id = $1::uuid",
    )
    .bind(&m)
    .execute(&pool)
    .await
    .expect_err("missions.current_version_id must not accept a version that is absent");
    assert_eq!(
        err.as_database_error().and_then(|d| d.code()).as_deref(),
        Some("23503"),
        "a dangling current_version_id must be a foreign-key violation, not {err:?}"
    );

    // ── 3. An `orbat` that cannot be read. Pre-fix this did not merely vanish — it fell through
    // to the editor-derived ORBAT, so a DIFFERENT seating plan than the one authored could be
    // materialized under a 201. 400, naming the payload, with serde's message attached. ──
    let m = mission("Attach unreadable").await;
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

    // ── 4. A perfectly VALID payload that seats nobody — the shape that needs no mistake at
    // all. `input.orbat.is_empty()` cannot see it, because the squad list is not empty; only
    // the slot count is. ──
    let m = mission("Attach valid but empty").await;
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
    let m = mission("Attach request empty").await;
    let (st, b) = attach(
        &m,
        r#","orbat":[{"faction":"USA","squad":"Alpha","slots":[]}]"#,
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "request orbat, no slots: {b}");

    // ── 6. Control — one real slot still attaches, and materializes exactly one row. Without
    // this the whole test would pass by refusing everything. ──
    let m = mission("Attach control").await;
    let (st, b) = attach(
        &m,
        r#","orbat":[{"faction":"USA","callsign":"A","squad":"Attach Alpha","slots":[{"role":"SL"}]}]"#,
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
/// A registration guard reading `capacity > 0 && registered >= capacity` does not protect the
/// comparison at `capacity == 0`, it switches it off, so every seatless registration is
/// accepted as `registered` without limit. `add_event_mission` refuses to create a zero-slot
/// mission, so the zero-capacity row here is **seeded directly**, which is also how legacy rows
/// and the dev seed arrive.
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
    let mission_id = mission("Attach seatless").await;
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
    let mission_id = mission("Attach capped").await;
    let ev = event(2).await;
    let (st, em) = call(
        &app,
        "POST",
        &format!("/api/v1/events/{ev}/missions"),
        &admin,
        Some(&format!(
            r#"{{"mission_id":"{mission_id}","start_time":"2027-09-01T00:00:00Z","orbat":[{{"faction":"USA","callsign":"A","squad":"Attach Capped","slots":[{{"role":"R0"}},{{"role":"R1"}},{{"role":"R2"}},{{"role":"R3"}}]}}]}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "attach: {em}");
    let emid = em["id"].as_str().unwrap().to_string();

    // Two seeded strangers fill the cap. dev-login is one fixed identity, so the other
    // occupants are seeded and only the caller under test goes through the handler — the same
    // idiom the seat-race tests use. `arma_id` has its own unique index.
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
    let mission_2 = mission("Attach capped second").await;
    let (st, em2) = call(
        &app,
        "POST",
        &format!("/api/v1/events/{ev}/missions"),
        &admin,
        Some(&format!(
            r#"{{"mission_id":"{mission_2}","start_time":"2027-09-02T00:00:00Z","orbat":[{{"faction":"USA","callsign":"B","squad":"Attach Second","slots":[{{"role":"R0"}}]}}]}}"#
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

/// `DELETE …/slots/:id/assign` frees a claimed seat for leader/admin, and there is no
/// `events.match_id` column (the link is `matches.event_id`).
///
/// This is the API half of that contract — assign → clear → both `orbat_slots.assigned_to`
/// and `event_registrations.slot_id` are null, enlisted without a reserve is forbidden, and
/// `information_schema` shows no `events.match_id`.
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
            r#"{"title":"Override Clear","terrain":"everon","game_mode":"pve_coop","max_players":16}"#,
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
        Some(r#"{"start_time":"2027-07-01T00:00:00Z","name_override":"Override Op"}"#),
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
