//! Mission lifecycle + the live `/compiled` route (gate G6 end-to-end). Skips
//! without `TEST_DATABASE_URL`.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use tower::ServiceExt;
use website_api::config::Config;
use website_api::state::AppState;
use website_api::{app, db};

async fn app_and_token(role: &str) -> Option<(Router, String)> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let app = app::router(AppState::new(
        pool,
        Config::for_tests(url, "missions-secret"),
    ));
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
    let tok = loc
        .split_once('#')
        .unwrap()
        .1
        .split('&')
        .find_map(|p| p.strip_prefix("access_token="))
        .unwrap()
        .to_string();
    Some((app, tok))
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    bearer: Option<&str>,
    svc: Option<&str>,
    body: Option<&str>,
) -> (StatusCode, Vec<u8>) {
    let mut b = Request::builder().method(method).uri(uri);
    if let Some(t) = bearer {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    if let Some(s) = svc {
        b = b.header("x-service-token", s);
    }
    if body.is_some() {
        b = b.header(header::CONTENT_TYPE, "application/json");
    }
    let req = b
        .body(body.map_or(Body::empty(), |s| Body::from(s.to_string())))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec();
    (status, bytes)
}

fn json(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap_or(Value::Null)
}

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

    // Library list envelope.
    let (st, b) = call(&app, "GET", "/api/v1/missions", t, None, None).await;
    assert_eq!(st, StatusCode::OK);
    let list = json(&b);
    assert!(
        list["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == id.as_str())
    );
    assert!(list["total"].is_number());

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
    let (_, b) = call(
        &app,
        "GET",
        "/api/v1/missions?scope=bookmarked",
        t,
        None,
        None,
    )
    .await;
    assert!(
        json(&b)["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == id.as_str())
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

/// T-181.31 — `/compiled` holds the flattened document to `mission.schema.json`
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

/// T-181.44 — the T-181.42 callsign, end to end, through the real routes.
///
/// A squad callsign of `AL<TAB>PHA` was a mission the write side accepted with a 201 and the read
/// side then refused with a 500 that only an API log ever saw. The author got no signal at all;
/// the operator got "the server is still running the previous mission". This asserts the whole
/// inversion: the SAVE is now refused, with a 400 that names the field, the value and the
/// character — and, because a save that never happened cannot be served, the follow-up `/compiled`
/// still returns the last GOOD version rather than a 500.
///
/// It fails on base at the very first assertion: `create_version` answered 201.
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

#[tokio::test]
async fn enlisted_cannot_create_mission() {
    let Some((app, tok)) = app_and_token("enlisted").await else {
        return;
    };
    let create = r#"{"title":"X","terrain":"everon","game_mode":"pve_coop","max_players":16}"#;
    let (st, _) = call(
        &app,
        "POST",
        "/api/v1/missions",
        Some(&tok),
        None,
        Some(create),
    )
    .await;
    assert_eq!(st, StatusCode::FORBIDDEN);
}
