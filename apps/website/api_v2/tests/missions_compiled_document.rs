//! The mission document the mod consumes: the draft-to-`/compiled` lifecycle, the schema
//! gate in front of the served document, the save-side refusal of control characters, and the
//! cargo-capacity refusal a stored over-capacity version triggers.
//!
//! Skips without `TEST_DATABASE_URL`.

use axum::http::StatusCode;
use serde_json::Value;

mod common;
mod missions_support;

use missions_support::*;

#[tokio::test]
async fn mission_lifecycle_and_compiled() {
    let Some((app, tok)) = app_and_token("mission_maker").await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let t = Some(tok.as_str());

    // Create draft.
    let create =
        r#"{"title":"Rust Op","terrain":"everon","game_mode":"pve_coop","max_players":16}"#;
    let (st, b) = call(&app, "POST", "/api/v1/missions", t, None, Some(create)).await;
    assert_eq!(
        st,
        StatusCode::CREATED,
        "create: {}",
        String::from_utf8_lossy(&b)
    );
    let m = json(&b);
    let id = m["id"].as_str().unwrap().to_string();
    assert_eq!(m["status"], "draft");
    assert_eq!(m["terrain"], "everon");
    assert_eq!(m["time_of_day"], "14:00:00"); // default 14:00 via ::time cast

    // Overview: card + armory[] + current_version.
    let (st, b) = call(
        &app,
        "GET",
        &format!("/api/v1/missions/{id}"),
        t,
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let d = json(&b);
    assert!(d["armory"].is_array());
    assert_eq!(d["bookmarked"], false);
    assert_eq!(d["current_version"]["semver"], "0.1.0");

    // Library list envelope — paginate; the residue-independent pin lives in the paging suite.
    let (st, b) = call(&app, "GET", "/api/v1/missions", t, None, None).await;
    assert_eq!(st, StatusCode::OK);
    let list = json(&b);
    assert!(list["total"].is_number());
    assert!(
        find_id_in_missions_list(&app, tok.as_str(), "/api/v1/missions", &id).await,
        "created mission missing from GET /missions (paginated): total={}",
        list["total"]
    );

    // Patch title.
    let (st, b) = call(
        &app,
        "PATCH",
        &format!("/api/v1/missions/{id}"),
        t,
        None,
        Some(r#"{"title":"Rust Op 2"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(json(&b)["title"], "Rust Op 2");

    // Save version + dup 409.
    let ver = r#"{"semver":"0.2.0","payload":{"editor":{"slots":[]}}}"#;
    let (st, b) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{id}/versions"),
        t,
        None,
        Some(ver),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::CREATED,
        "version: {}",
        String::from_utf8_lossy(&b)
    );
    let vid = json(&b)["id"].as_str().unwrap().to_string();
    let (st, _) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{id}/versions"),
        t,
        None,
        Some(ver),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT, "dup semver");
    let (st, b) = call(
        &app,
        "GET",
        &format!("/api/v1/missions/{id}/versions/{vid}"),
        t,
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(json(&b)["semver"], "0.2.0");

    // Armory replace + read.
    let arm = r#"{"items":[{"faction":"USA","category":"rifle","item_name":"M4","sort_order":0}]}"#;
    let (st, b) = call(
        &app,
        "PUT",
        &format!("/api/v1/missions/{id}/armory"),
        t,
        None,
        Some(arm),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(json(&b)["data"][0]["item_name"], "M4");

    // Bookmark toggle + scoped list.
    let (st, b) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{id}/bookmark"),
        t,
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(json(&b)["bookmarked"], true);
    // Bookmarked scope — same ORDER BY / LIMIT ratchet as the global list.
    assert!(
        find_id_in_missions_list(&app, tok.as_str(), "/api/v1/missions?scope=bookmarked", &id)
            .await,
        "bookmarked mission missing from GET /missions?scope=bookmarked (paginated)"
    );
    let (st, b) = call(
        &app,
        "DELETE",
        &format!("/api/v1/missions/{id}/bookmark"),
        t,
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(json(&b)["bookmarked"], false);

    // Export envelope (camelCase).
    let (st, b) = call(
        &app,
        "GET",
        &format!("/api/v1/missions/{id}/export"),
        t,
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let ex = json(&b);
    assert_eq!(ex["exportFormatVersion"], 1);
    assert_eq!(ex["missionId"], id.as_str());
    assert_eq!(ex["gameMode"], "pve_coop");
    assert_eq!(ex["maxPlayers"], 16);
    assert!(ex["armory"].is_array());

    // Compiled: no service token → 401; with token, slotless payload → 409 (flatten ran).
    let (st, _) = call(
        &app,
        "GET",
        &format!("/api/v1/missions/{id}/compiled"),
        None,
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let (st, b) = call(
        &app,
        "GET",
        &format!("/api/v1/missions/{id}/compiled"),
        None,
        Some("test-service-token"),
        None,
    )
    .await;
    assert_eq!(
        st,
        StatusCode::CONFLICT,
        "compiled: {}",
        String::from_utf8_lossy(&b)
    );
    assert_eq!(json(&b)["error"], "no placed slots");
}

/// `/compiled` holds the flattened document to `mission.schema.json`
/// before serving it. The document is the whole website↔mod interface, and the mod
/// hard-fails on a violation with the reason visible only in the game console; the
/// website used to answer 200 regardless.
///
/// Both halves matter: a well-formed mission must still be served (the gate must not
/// be over-eager), and a slot that lost its `id` — which compiles to `uid: ""`, a
/// `minLength: 1` violation the deliberately-unconstrained editor-payload schema
/// cannot catch on write — must be refused with a diagnostic that names it.
#[tokio::test]
async fn compiled_document_is_schema_validated_before_serving() {
    let Some((app, tok)) = app_and_token("mission_maker").await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let t = Some(tok.as_str());
    let create =
        r#"{"title":"Gate Op","terrain":"everon","game_mode":"pve_coop","max_players":16}"#;
    let (st, b) = call(&app, "POST", "/api/v1/missions", t, None, Some(create)).await;
    assert_eq!(st, StatusCode::CREATED, "{}", String::from_utf8_lossy(&b));
    let id = json(&b)["id"].as_str().unwrap().to_string();

    // A well-formed mission still compiles and is served verbatim.
    let good = r#"{"semver":"0.2.0","payload":{"editor":{
        "factions":[{"id":"f1","key":"BLUFOR","name":"US Army","squadIds":["sq1"]}],
        "squads":[{"id":"sq1","factionId":"f1","callsign":"Alpha","name":"A 1-1","slotIds":["s1"]}],
        "slots":[{"id":"s1","squadId":"sq1","index":0,"role":"SL",
            "position":{"x":4839.2,"y":6620.8,"z":0,"rotation":270}}],
        "editorLayers":[]}}}"#;
    let (st, b) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{id}/versions"),
        t,
        None,
        Some(good),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{}", String::from_utf8_lossy(&b));

    let (st, b) = call(
        &app,
        "GET",
        &format!("/api/v1/missions/{id}/compiled"),
        None,
        Some("test-service-token"),
        None,
    )
    .await;
    assert_eq!(
        st,
        StatusCode::OK,
        "valid mission must still be served: {}",
        String::from_utf8_lossy(&b)
    );
    let doc = json(&b);
    assert_eq!(doc["slots"][0]["uid"], "s1");
    assert_eq!(doc["slots"][0]["role"], "SL");

    // Same mission, a slot that lost its id → `uid: ""` → schema-invalid.
    let bad = r#"{"semver":"0.3.0","payload":{"editor":{
        "factions":[{"id":"f1","key":"BLUFOR","name":"US Army","squadIds":["sq1"]}],
        "squads":[{"id":"sq1","factionId":"f1","callsign":"Alpha","name":"A 1-1","slotIds":[""]}],
        "slots":[{"id":"","squadId":"sq1","index":0,"role":"SL",
            "position":{"x":4839.2,"y":6620.8,"z":0,"rotation":270}}],
        "editorLayers":[]}}}"#;
    let (st, b) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{id}/versions"),
        t,
        None,
        Some(bad),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::CREATED,
        "the write side cannot catch this: {}",
        String::from_utf8_lossy(&b)
    );

    let (st, b) = call(
        &app,
        "GET",
        &format!("/api/v1/missions/{id}/compiled"),
        None,
        Some("test-service-token"),
        None,
    )
    .await;
    // 500, not 4xx: the document is server-generated and the caller — a game server
    // that sent nothing but an id — can do nothing about it.
    assert_eq!(
        st,
        StatusCode::INTERNAL_SERVER_ERROR,
        "schema-invalid document must not be served: {}",
        String::from_utf8_lossy(&b)
    );
    let err = json(&b);
    assert_eq!(err["error"], "compiled mission failed schema validation");
    assert_eq!(err["details"]["schema"], "mission.schema.json");
    assert!(err["details"]["findingCount"].as_u64().unwrap() >= 1);
    let findings = err["details"]["findings"].as_array().unwrap();
    assert!(
        findings.iter().any(|f| f.as_str().unwrap().contains("uid")),
        "the diagnostic must name what is wrong, got {findings:?}"
    );
}

/// A control character in a squad callsign, end to end, through the real routes.
///
/// A squad callsign of `AL<TAB>PHA` was a mission the write side accepted with a 201 and the read
/// side then refused with a 500 that only an API log ever saw. The author got no signal at all;
/// the operator got "the server is still running the previous mission". This asserts the whole
/// inversion: the SAVE is now refused, with a 400 that names the field, the value and the
/// character — and, because a save that never happened cannot be served, the follow-up `/compiled`
/// still returns the last GOOD version rather than a 500.
///
/// RED: a `create_version` that answers 201 fails the very first assertion.
#[tokio::test]
async fn control_character_in_a_callsign_is_refused_at_save_not_at_fetch() {
    let Some((app, tok)) = app_and_token("mission_maker").await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let t = Some(tok.as_str());
    let create =
        r#"{"title":"Wire Op","terrain":"everon","game_mode":"pve_coop","max_players":16}"#;
    let (st, b) = call(&app, "POST", "/api/v1/missions", t, None, Some(create)).await;
    assert_eq!(st, StatusCode::CREATED, "{}", String::from_utf8_lossy(&b));
    let id = json(&b)["id"].as_str().unwrap().to_string();
    let versions = format!("/api/v1/missions/{id}/versions");
    let compiled = format!("/api/v1/missions/{id}/compiled");

    // A clean save first, so the mission has something servable to fall back to. (0.1.0 is taken:
    // POST /missions seeds an initial version.)
    let good = r#"{"semver":"0.2.0","payload":{"editor":{
        "factions":[{"id":"f1","key":"BLUFOR","name":"US Army","squadIds":["sq1"]}],
        "squads":[{"id":"sq1","factionId":"f1","callsign":"ALPHA","slotIds":["s1"]}],
        "slots":[{"id":"s1","squadId":"sq1","index":0,"role":"SL",
            "position":{"x":4839.2,"y":6620.8,"z":0,"rotation":270}}],
        "editorLayers":[]}}}"#;
    let (st, b) = call(&app, "POST", &versions, t, None, Some(good)).await;
    assert_eq!(st, StatusCode::CREATED, "{}", String::from_utf8_lossy(&b));

    // Same mission, one TAB in the callsign. `\t` here is the JSON escape, so the stored value
    // carries a real control character — exactly what an editor paste can produce.
    let bad = r#"{"semver":"0.3.0","payload":{"editor":{
        "factions":[{"id":"f1","key":"BLUFOR","name":"US Army","squadIds":["sq1"]}],
        "squads":[{"id":"sq1","factionId":"f1","callsign":"AL\tPHA","slotIds":["s1"]}],
        "slots":[{"id":"s1","squadId":"sq1","index":0,"role":"SL",
            "position":{"x":4839.2,"y":6620.8,"z":0,"rotation":270}}],
        "editorLayers":[]}}}"#;
    let (st, b) = call(&app, "POST", &versions, t, None, Some(bad)).await;
    assert_eq!(
        st,
        StatusCode::BAD_REQUEST,
        "the save must be refused: {}",
        String::from_utf8_lossy(&b)
    );
    let err = json(&b);
    assert_eq!(err["error"], "invalid mission payload");
    let details = err["details"].as_array().expect("details array");
    let named = details
        .iter()
        .filter_map(Value::as_str)
        .find(|d| d.starts_with("/editor/squads/0/callsign:"))
        .unwrap_or_else(|| panic!("no finding naming the field: {details:?}"));
    assert!(
        named.contains("TAB (U+0009)") && named.contains(r#""AL\tPHA""#),
        "the finding must name the character and echo the value: {named}"
    );

    // The refused save left no version behind, so the game server still gets the last good one —
    // no 500, and nothing for the mod to fail over to a stale cache for.
    let (st, b) = call(
        &app,
        "GET",
        &compiled,
        None,
        Some("test-service-token"),
        None,
    )
    .await;
    assert_eq!(
        st,
        StatusCode::OK,
        "the last good version must still serve: {}",
        String::from_utf8_lossy(&b)
    );
    assert_eq!(json(&b)["slots"][0]["groupCallsign"], "ALPHA");
}

/// Live `/compiled` refuses over-capacity versions once the registry phys catalog is loaded:
/// 4×60 cm³ of magazines into a 200 cm³ plate carrier.
///
/// Save already refuses this payload (it cannot be POSTed). The exposure is an empty-catalog
/// compile of rows that bypassed Save — so this inserts the version directly and asserts
/// `/compiled` answers 500 with the cargo finding, not 200.
///
/// RED: `get_compiled_mission` calls no-arg `flatten_to_mod_document` again.
#[tokio::test]
async fn compiled_refuses_over_capacity_when_registry_phys_is_loaded() {
    let Some((app, pool, maker, _admin)) = app_pool_and_tokens().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let vest_rn = format!("compiled_kit_vest_{stamp}");
    let mag_rn = format!("compiled_kit_mag_{stamp}");

    // A current modpack + phys rows Save/compile both read. Private resource names so parallel
    // suites cannot collide; is_current=true so load_cargo_phys_catalog includes them even if
    // another pack is also current.
    let pack_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO modpacks (name, version, total_size_bytes, workshop_url, is_current, created_at) \
         VALUES ($1, '0.0.1', 1, 'https://example.invalid/compiled-kit', true, now()) RETURNING id",
    )
    .bind(format!("Compiled Kit Pack {stamp}"))
    .fetch_one(&pool)
    .await
    .expect("seed modpack");
    sqlx::query(
        "INSERT INTO registry_items \
         (modpack_id, resource_name, display_name, category, kind, sort_order, \
          weight_kg, volume_cm3, created_at, updated_at) \
         VALUES ($1, $2, 'Mag', 'CompiledKit', 'gear_vest', 0, 0.5, 60.0, now(), now())",
    )
    .bind(pack_id)
    .bind(&mag_rn)
    .execute(&pool)
    .await
    .expect("seed mag phys");
    sqlx::query(
        "INSERT INTO registry_items \
         (modpack_id, resource_name, display_name, category, kind, sort_order, \
          max_weight_kg, max_volume_cm3, created_at, updated_at) \
         VALUES ($1, $2, 'Plate Carrier', 'CompiledKit', 'gear_vest', 1, 5.0, 200.0, now(), now())",
    )
    .bind(pack_id)
    .bind(&vest_rn)
    .execute(&pool)
    .await
    .expect("seed vest phys");

    let create = format!(
        r#"{{"title":"Compiled Kit Cargo {stamp}","terrain":"everon","game_mode":"pve_coop","max_players":16}}"#
    );
    let (st, b) = call(
        &app,
        "POST",
        "/api/v1/missions",
        Some(&maker),
        None,
        Some(&create),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "{}", String::from_utf8_lossy(&b));
    let mid = json(&b)["id"].as_str().unwrap().to_string();

    // Bypass Save (which would 400) — the residual this pins is a stored row, not an API write.
    let payload = format!(
        r#"{{"schemaVersion":1,"editor":{{
        "factions":[{{"id":"f1","key":"BLUFOR","name":"US Army","squadIds":["sq1"]}}],
        "squads":[{{"id":"sq1","factionId":"f1","callsign":"Alpha","slotIds":["s1"]}}],
        "slots":[{{"id":"s1","squadId":"sq1","index":0,"role":"RFL",
            "position":{{"x":100.0,"y":200.0,"z":0,"rotation":0}},
            "loadout":{{"version":2,"wear":{{"vest":"{vest_rn}"}},"weapons":[],
              "cargo":[{{"container":"vest","item":"{mag_rn}","qty":4}}]}}}}],
        "editorLayers":[]}}}}"#
    );
    let vid = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO mission_versions (id, mission_id, semver, json_payload, editor_notes, created_by, created_at) \
         VALUES ($1, $2::uuid, '9.9.9', $3::jsonb, '', '000000000000000001', now())",
    )
    .bind(vid)
    .bind(&mid)
    .bind(&payload)
    .execute(&pool)
    .await
    .expect("insert over-capacity version");
    sqlx::query("UPDATE missions SET current_version_id = $1 WHERE id = $2::uuid")
        .bind(vid)
        .bind(&mid)
        .execute(&pool)
        .await
        .expect("point tip at over-capacity version");

    let (st, b) = call(
        &app,
        "GET",
        &format!("/api/v1/missions/{mid}/compiled"),
        None,
        Some("test-service-token"),
        None,
    )
    .await;
    assert_eq!(
        st,
        StatusCode::INTERNAL_SERVER_ERROR,
        "over-capacity tip must not compile: {}",
        String::from_utf8_lossy(&b)
    );
    let err = json(&b);
    assert_eq!(err["error"], "could not compile mission");
    let detail = err["details"]["detail"]
        .as_str()
        .unwrap_or_else(|| panic!("compile detail missing: {err}"));
    assert!(
        detail.contains("240 / 200 cm³") && detail.contains("Plate Carrier"),
        "compile refuse must name the cargo finding: {detail}"
    );

    // Cleanup so the shared gate DB does not accumulate forever-current packs.
    let _ = sqlx::query("DELETE FROM registry_items WHERE modpack_id = $1")
        .bind(pack_id)
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM modpacks WHERE id = $1")
        .bind(pack_id)
        .execute(&pool)
        .await;
}
