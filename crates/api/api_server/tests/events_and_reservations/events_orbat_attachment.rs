//! Attaching an ORBAT to an event and what its seats then mean: how the seating plan is
//! grouped for display, why an attach that would seat nobody is refused with the reason it was
//! seatless, how a zero-capacity or capped operation answers a registration, and what clearing
//! an assignment leaves behind. Skips without `TEST_DATABASE_URL`.

use crate::events_support;

use axum::http::StatusCode;
use events_support::{DB_LOCK, DEV_USER, boot, call, token};
use serde_json::Value;

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
    // 500 arm (`orbat_template_for_mission`, `api_operations/src/handlers/event_mission_attachment.rs`)
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
