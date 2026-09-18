//! What the event and announcement write endpoints accept, refuse, clear and echo: a blank
//! string must never overwrite a real value, `""` is a real instruction to clear, a padded but
//! real value is stored byte-identical, an unknown status is not a silent draft, and unknown
//! server / modpack ids are a 400 rather than a stored dangling pointer. Skips without
//! `TEST_DATABASE_URL`.
//!
//! The blank-string tests use a seeded sentinel asserted **by value** rather than a
//! "not empty" check: `""` written over `""` looks like success against a broken handler, and
//! so does a length check against `"   "`.

use axum::http::StatusCode;
use events_support::{DB_LOCK, boot, call, token};

mod common;
mod events_support;

/// T-348 — a whitespace-only `name_override` must not overwrite a real operation name.
///
/// The write is an `UPDATE`, so this uses T-317's instrument: a seeded sentinel asserted **by
/// value**, never "is not empty". `""` over `""` would look like success against the broken
/// handler, and so would a length check against `"   "`.
///
/// What makes the bug expensive is the breadth: a whitespace string is non-empty, so it defeats
/// six separate `is_empty()` fallbacks at once — `operations/handlers/member_service_record.rs`, `dashboard.rs:79`,
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
/// These cases live in `tests/events_field_contracts.rs` because they are the same guard as the
/// `name_override` one beside them — a blank string must not overwrite a real value — asserted on
/// the announcement writes rather than the event ones.
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
