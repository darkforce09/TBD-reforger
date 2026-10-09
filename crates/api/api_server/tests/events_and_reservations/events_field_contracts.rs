//! What the event and announcement write endpoints accept, refuse, clear and echo: a blank
//! string must never overwrite a real value, `""` is a real instruction to clear, a padded but
//! real value is stored byte-identical, an unknown status is not a silent draft, and unknown
//! server / modpack ids are a 400 rather than a stored dangling pointer. Skips without
//! `TEST_DATABASE_URL`.
//!
//! The blank-string tests use a seeded sentinel asserted **by value** rather than a
//! "not empty" check: `""` written over `""` looks like success against a broken handler, and
//! so does a length check against `"   "`.

use crate::events_support;

use axum::http::StatusCode;
use events_support::{DB_LOCK, boot, call, token};

/// `cms.rs`: a blank announcement title or body must be refused on both writes, and an
/// unrecognised status must not silently become a draft.
///
/// These cases live in this module because they are the same guard as the
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
    const TITLE: &str = "SENTINEL Announcement [field-contract]";
    const BODY: &str = "<p>SENTINEL body [field-contract]</p>";

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

/// Events carry per-event `server_id` + `modpack_id`.
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
    // suites cannot collide.
    let modpack_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO modpacks (name, version, total_size_bytes, workshop_url, is_current, created_at) \
         VALUES ('Binding Pack', '9.9.9', 42, 'https://example.invalid/binding', false, now()) \
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("seed modpack");
    let server_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO servers (name, ip, port, required_modpack_id, is_active) \
         VALUES ('Binding Srv', '127.0.0.1'::inet, 2260, $1, true) RETURNING id",
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
        Some(r#"{"start_time":"2027-06-01T19:00:00Z","name_override":"Binding unbound"}"#),
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
        r#"{{"start_time":"2027-06-02T19:00:00Z","name_override":"Binding bound","server_id":"{server_id}","modpack_id":"{modpack_id}"}}"#
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
