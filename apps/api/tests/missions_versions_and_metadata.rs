//! Saving a version and the mission-row metadata that save moves: the `updated_at` clock and
//! audit row, the weather contract on create and patch, the authored payload title mirrored onto
//! the library row, and re-pointing the current-version tip.
//!
//! Skips without `TEST_DATABASE_URL`.

use axum::http::StatusCode;
use serde_json::Value;

mod common;
mod missions_support;

use missions_support::*;

/// `POST /missions/:id/versions` must bump `missions.updated_at` and write an audit
/// row. Library orders by `updated_at`; approvals projects it as `submitted_at`. Before this
/// fix the handler only wrote `current_version_id`, so both clocks stayed frozen and the
/// save left no trail in `GET /admin/audit-logs`.
///
/// Perturbation RED: drop the `updated_at = now()` clause (or the `write_audit` call) and
/// either the timestamp assert or the audit-row assert fails.
#[tokio::test]
async fn create_version_bumps_updated_at_and_writes_audit() {
    let Some((app, pool, maker, admin)) = app_pool_and_tokens().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let title = format!("Version-Version-Save-{stamp}");
    let (st, b) = call(
        &app,
        "POST",
        "/api/v1/missions",
        Some(&maker),
        None,
        Some(&format!(
            r#"{{"title":"{title}","terrain":"everon","game_mode":"pve_coop","max_players":16}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{}", String::from_utf8_lossy(&b));
    let mid = json(&b)["id"].as_str().unwrap().to_string();
    let before: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT updated_at FROM missions WHERE id = $1::uuid")
            .bind(&mid)
            .fetch_one(&pool)
            .await
            .expect("mission updated_at before save");

    // Pin the clock in the past so a same-second `now()` cannot falsely pass equality.
    sqlx::query("UPDATE missions SET updated_at = now() - interval '1 hour' WHERE id = $1::uuid")
        .bind(&mid)
        .execute(&pool)
        .await
        .unwrap();
    let pinned: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT updated_at FROM missions WHERE id = $1::uuid")
            .bind(&mid)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(
        pinned < before,
        "pin must land strictly before the create-time stamp: pinned={pinned} before={before}"
    );

    let notes = format!("version editor notes {stamp}");
    let ver = format!(
        r#"{{"semver":"0.2.0","editor_notes":"{notes}","payload":{{"editor":{{"slots":[]}}}}}}"#
    );
    let (st, b) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{mid}/versions"),
        Some(&maker),
        None,
        Some(&ver),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::CREATED,
        "version save: {}",
        String::from_utf8_lossy(&b)
    );
    let body = json(&b);
    assert_eq!(body["semver"], "0.2.0");
    assert_eq!(
        body["editor_notes"],
        notes.as_str(),
        "editor_notes must round-trip on the returned MissionVersion: {body}"
    );

    let after: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT updated_at FROM missions WHERE id = $1::uuid")
            .bind(&mid)
            .fetch_one(&pool)
            .await
            .expect("mission updated_at after save");
    assert!(
        after > pinned,
        "create_version must bump missions.updated_at (library + approvals clocks): \
         after={after} pinned={pinned}"
    );

    let (st, b) = call(
        &app,
        "GET",
        &format!("/api/v1/admin/audit-logs?limit=100&q={title}"),
        Some(&admin),
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    let rows = json(&b);
    let saves: Vec<&Value> = rows["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["action"] == "mission.version" && r["target_id"] == mid.as_str())
        .collect();
    assert_eq!(
        saves.len(),
        1,
        "exactly one mission.version audit row for this save: {rows}"
    );
    assert_eq!(saves[0]["target_type"], "mission");
    assert_eq!(saves[0]["severity"], "info");
    assert!(
        saves[0]["message"]
            .as_str()
            .is_some_and(|m| m.contains("0.2.0") && m.contains(&title)),
        "audit message must name the semver and title: {}",
        saves[0]
    );
}

/// CREATE contract: omitted or `""` weather → 201 + `clear`.
///
/// The sibling unit tests pin the handler source; this is the live HTTP layer
/// `admin_approvals_cms_field_tools` already exercises incidentally (POST without weather).
/// Explicit pin so a regression that 400s omitted weather cannot hide behind the source pins alone.
///
/// Assert-flip check: expect `weather == "dense_fog"` on the omit path — fails while production
/// still defaults to Clear.
#[tokio::test]
async fn create_mission_omitted_or_blank_weather_defaults_to_clear() {
    let Some((app, tok)) = app_and_token("mission_maker").await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let t = Some(tok.as_str());
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    // Omit weather entirely (`#[serde(default)]` → `""` → Clear).
    let title_omit = format!("Weather-Create-Omit-{stamp}");
    let (st, b) = call(
        &app,
        "POST",
        "/api/v1/missions",
        t,
        None,
        Some(&format!(
            r#"{{"title":"{title_omit}","terrain":"everon","game_mode":"pve_coop","max_players":16}}"#
        )),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::CREATED,
        "CREATE omit weather: {}",
        String::from_utf8_lossy(&b)
    );
    let omit = json(&b);
    assert_eq!(
        omit["weather"], "clear",
        "omitted weather must default to clear: {omit}"
    );

    // Explicit empty string is the same serde/default path and must also land Clear.
    let title_blank = format!("Weather-Create-Blank-{stamp}");
    let (st, b) = call(
        &app,
        "POST",
        "/api/v1/missions",
        t,
        None,
        Some(&format!(
            r#"{{"title":"{title_blank}","terrain":"everon","game_mode":"pve_coop","weather":"","max_players":16}}"#
        )),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::CREATED,
        "CREATE weather=\"\": {}",
        String::from_utf8_lossy(&b)
    );
    let blank = json(&b);
    assert_eq!(
        blank["weather"], "clear",
        "blank weather on CREATE must default to clear: {blank}"
    );
}

/// PATCH contract: after `dense_fog`, `{"weather":""}` → 400 and the row stays.
///
/// A `valid_weather` that maps `""` → Clear makes this PATCH answer 200 and rewrite the row.
/// The unit tests cover the helper; this covers the wire + persistence.
///
/// Assert-flip check: expect `StatusCode::OK` on the blank PATCH — fails while production 400s.
#[tokio::test]
async fn patch_blank_weather_rejects_and_preserves_dense_fog() {
    let Some((app, pool, maker, _)) = app_pool_and_tokens().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let title = format!("Weather-Patch-Blank-{stamp}");

    let (st, b) = call(
        &app,
        "POST",
        "/api/v1/missions",
        Some(&maker),
        None,
        Some(&format!(
            r#"{{"title":"{title}","terrain":"everon","game_mode":"pve_coop","max_players":16}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{}", String::from_utf8_lossy(&b));
    let mid = json(&b)["id"].as_str().unwrap().to_string();

    let (st, b) = call(
        &app,
        "PATCH",
        &format!("/api/v1/missions/{mid}"),
        Some(&maker),
        None,
        Some(r#"{"weather":"dense_fog"}"#),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::OK,
        "set dense_fog: {}",
        String::from_utf8_lossy(&b)
    );
    assert_eq!(json(&b)["weather"], "dense_fog");

    let (st, b) = call(
        &app,
        "PATCH",
        &format!("/api/v1/missions/{mid}"),
        Some(&maker),
        None,
        Some(r#"{"weather":""}"#),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::BAD_REQUEST,
        "blank weather PATCH must 400: {}",
        String::from_utf8_lossy(&b)
    );
    assert_eq!(
        json(&b)["error"],
        "invalid weather",
        "blank weather error body: {}",
        String::from_utf8_lossy(&b)
    );

    let stored: String =
        sqlx::query_scalar("SELECT weather::text FROM missions WHERE id = $1::uuid")
            .bind(&mid)
            .fetch_one(&pool)
            .await
            .expect("weather after rejected blank PATCH");
    assert_eq!(
        stored, "dense_fog",
        "rejected blank PATCH must leave dense_fog untouched"
    );

    let (st, b) = call(
        &app,
        "GET",
        &format!("/api/v1/missions/{mid}"),
        Some(&maker),
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    assert_eq!(
        json(&b)["weather"],
        "dense_fog",
        "GET must still serve dense_fog after blank PATCH reject: {}",
        String::from_utf8_lossy(&b)
    );
}

/// `create_version` mirrors a non-blank payload `title` onto `missions.title`.
///
/// Create with a stale library title, Save a version whose payload carries an authored title,
/// then GET the mission row and assert the title moved. Whitespace-only payload title must NOT
/// clobber the row.
///
/// Fails when the `title = $3` arm is dropped from `create_version`: the first assert fails.
///
/// This has to be an integration test: source pins on the handler alone cannot prove
/// the SQL UPDATE actually lands on the row.
#[tokio::test]
async fn create_version_mirrors_authored_payload_title_onto_mission_row() {
    let Some((app, _pool, maker, _admin)) = app_pool_and_tokens().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let stale = format!("Title-Stale-{stamp}");
    let authored = format!("Title-Authored-{stamp}");
    let (st, b) = call(
        &app,
        "POST",
        "/api/v1/missions",
        Some(&maker),
        None,
        Some(&format!(
            r#"{{"title":"{stale}","terrain":"everon","game_mode":"pve_coop","max_players":16}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{}", String::from_utf8_lossy(&b));
    let mid = json(&b)["id"].as_str().unwrap().to_string();
    assert_eq!(json(&b)["title"], stale.as_str());

    let ver = format!(
        r#"{{"semver":"0.3.0","payload":{{"title":"  {authored}  ","schemaVersion":1,"map":{{"terrain":"everon"}},"environment":{{}},"editor":{{"factions":[],"squads":[],"slots":[],"editorLayers":[]}}}}}}"#
    );
    let (st, b) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{mid}/versions"),
        Some(&maker),
        None,
        Some(&ver),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::CREATED,
        "version save: {}",
        String::from_utf8_lossy(&b)
    );

    let (st, b) = call(
        &app,
        "GET",
        &format!("/api/v1/missions/{mid}"),
        Some(&maker),
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    assert_eq!(
        json(&b)["title"],
        authored.as_str(),
        "create_version must mirror trimmed payload title onto missions.title; got {}",
        String::from_utf8_lossy(&b)
    );

    // Whitespace-only payload title must leave the row alone (non-blank guard).
    let ver_ws = r#"{"semver":"0.3.1","payload":{"title":"   ","schemaVersion":1,"map":{"terrain":"everon"},"environment":{},"editor":{"factions":[],"squads":[],"slots":[],"editorLayers":[]}}}"#;
    let (st, b) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{mid}/versions"),
        Some(&maker),
        None,
        Some(ver_ws),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::CREATED,
        "whitespace title save: {}",
        String::from_utf8_lossy(&b)
    );
    let (st, b) = call(
        &app,
        "GET",
        &format!("/api/v1/missions/{mid}"),
        Some(&maker),
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    assert_eq!(
        json(&b)["title"],
        authored.as_str(),
        "whitespace-only payload title must not clobber missions.title"
    );
}

/// `POST /missions/:id/versions/:vid/set-current` re-points the tip.
///
/// The handler carries its own unit tests; this is the live HTTP layer. Create two
/// non-vacuous versions (0.1.0 is the seed), leave the tip on the newer, then set-current to the
/// older and assert `current_version_id`, and that submission compiles the older payload into the
/// artifact under review.
///
/// Perturbation RED: drop the UPDATE in `set_current_version` → tip stays on 0.3.0 and the
/// submitted artifact carries BRAVO / RIFLEMAN.
#[tokio::test]
async fn set_current_version_repaints_tip_over_http() {
    let Some((app, tok)) = app_and_token("mission_maker").await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let t = Some(tok.as_str());
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let title = format!("Tip-SetCurrent-{stamp}");
    let (st, b) = call(
        &app,
        "POST",
        "/api/v1/missions",
        t,
        None,
        Some(&format!(
            r#"{{"title":"{title}","terrain":"everon","game_mode":"pve_coop","max_players":16}}"#
        )),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{}", String::from_utf8_lossy(&b));
    let mid = json(&b)["id"].as_str().unwrap().to_string();
    let versions = format!("/api/v1/missions/{mid}/versions");

    // Older tip candidate — ALPHA / SL. Distinct from the newer BRAVO / RIFLEMAN so the artifact
    // proves which version the tip actually points at (not just that some version exists).
    let older = r#"{"semver":"0.2.0","payload":{"editor":{
        "factions":[{"id":"f1","key":"BLUFOR","name":"US Army","squadIds":["sq1"]}],
        "squads":[{"id":"sq1","factionId":"f1","callsign":"ALPHA","slotIds":["s1"]}],
        "slots":[{"id":"s1","squadId":"sq1","index":0,"role":"SL",
            "position":{"x":4839.2,"y":6620.8,"z":0,"rotation":270}}],
        "editorLayers":[]}}}"#;
    let (st, b) = call(&app, "POST", &versions, t, None, Some(older)).await;
    assert_eq!(
        st,
        StatusCode::CREATED,
        "older version: {}",
        String::from_utf8_lossy(&b)
    );
    let older_vid = json(&b)["id"].as_str().unwrap().to_string();

    let newer = r#"{"semver":"0.3.0","payload":{"editor":{
        "factions":[{"id":"f1","key":"BLUFOR","name":"US Army","squadIds":["sq1"]}],
        "squads":[{"id":"sq1","factionId":"f1","callsign":"BRAVO","slotIds":["s2"]}],
        "slots":[{"id":"s2","squadId":"sq1","index":0,"role":"RIFLEMAN",
            "position":{"x":4900.0,"y":6700.0,"z":0,"rotation":90}}],
        "editorLayers":[]}}}"#;
    let (st, b) = call(&app, "POST", &versions, t, None, Some(newer)).await;
    assert_eq!(
        st,
        StatusCode::CREATED,
        "newer version: {}",
        String::from_utf8_lossy(&b)
    );
    let newer_vid = json(&b)["id"].as_str().unwrap().to_string();
    assert_ne!(older_vid, newer_vid, "two distinct version rows required");

    // create_version advances the tip — confirm before set-current so a no-op handler cannot hide.
    let (st, b) = call(
        &app,
        "GET",
        &format!("/api/v1/missions/{mid}"),
        t,
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    let before = json(&b);
    assert_eq!(
        before["current_version_id"].as_str(),
        Some(newer_vid.as_str()),
        "tip must sit on 0.3.0 before set-current: {before}"
    );
    assert_eq!(before["current_version"]["semver"], "0.3.0");

    let (st, b) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{mid}/versions/{older_vid}/set-current"),
        t,
        None,
        None,
    )
    .await;
    assert_eq!(
        st,
        StatusCode::OK,
        "set-current: {}",
        String::from_utf8_lossy(&b)
    );
    let after = json(&b);
    assert_eq!(
        after["current_version_id"].as_str(),
        Some(older_vid.as_str()),
        "set-current response must re-point current_version_id to the older row: {after}"
    );

    let (st, b) = call(
        &app,
        "GET",
        &format!("/api/v1/missions/{mid}"),
        t,
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    let get = json(&b);
    assert_eq!(
        get["current_version_id"].as_str(),
        Some(older_vid.as_str()),
        "GET must show tip on older version after set-current: {get}"
    );
    assert_eq!(get["current_version"]["semver"], "0.2.0");
    assert_eq!(
        get["current_version"]["id"].as_str(),
        Some(older_vid.as_str())
    );

    // Submission compiles the re-pointed tip, not the abandoned newer version.
    let (st, b) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{mid}/submit"),
        t,
        None,
        None,
    )
    .await;
    assert_eq!(
        st,
        StatusCode::OK,
        "submit: {}",
        String::from_utf8_lossy(&b)
    );
    let (st, b) = call(
        &app,
        "GET",
        &format!("/api/v1/missions/{mid}/reviews"),
        t,
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    let reviews = json(&b);
    assert_eq!(
        reviews["reviews"][0]["mission_version_id"],
        older_vid.as_str()
    );
    let artifact = reviews["reviews"][0]["artifact_id"]
        .as_str()
        .unwrap()
        .to_string();
    let (st, b) = call(
        &app,
        "GET",
        &format!("/api/v1/missions/{mid}/artifacts/{artifact}/document"),
        t,
        None,
        None,
    )
    .await;
    assert_eq!(
        st,
        StatusCode::OK,
        "artifact document: {}",
        String::from_utf8_lossy(&b)
    );
    let tip_old = json(&b);
    assert_eq!(tip_old["slots"][0]["uid"], "s1");
    assert_eq!(tip_old["slots"][0]["groupCallsign"], "ALPHA");
    assert_eq!(tip_old["slots"][0]["role"], "SL");
}
