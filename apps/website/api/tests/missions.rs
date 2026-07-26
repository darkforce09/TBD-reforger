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

/// `call` with the `Content-Type` under the caller's control, and a body that can be sent
/// without one at all (`ct: None`). `call` always pairs a body with `application/json`, which
/// is exactly the header a fat-fingered client gets wrong, so the T-315 cases below cannot be
/// expressed through it.
async fn call_ct(
    app: &Router,
    method: &str,
    uri: &str,
    bearer: &str,
    ct: Option<&str>,
    body: Option<&str>,
) -> (StatusCode, Vec<u8>) {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {bearer}"));
    if let Some(ct) = ct {
        b = b.header(header::CONTENT_TYPE, ct);
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

/// The four-item armory every T-315 case starts from.
const ARMORY_SEED: &str = r#"{"items":[
    {"faction":"USA","category":"rifle","item_name":"M4A1","quantity":24,"icon":"m4.png","sort_order":0},
    {"faction":"USA","category":"launcher","item_name":"AT4","quantity":6,"icon":"at4.png","sort_order":1},
    {"faction":"USSR","category":"rifle","item_name":"AK-74","quantity":30,"icon":"ak74.png","sort_order":2},
    {"faction":"USSR","category":"mg","item_name":"PKM","quantity":4,"icon":"pkm.png","sort_order":3}]}"#;

/// Create a mission with the seeded armory and return `(id, armory_url)`.
async fn mission_with_armory(app: &Router, t: &str) -> (String, String) {
    let create =
        r#"{"title":"Armory Op","terrain":"everon","game_mode":"pve_coop","max_players":16}"#;
    let (st, b) = call(app, "POST", "/api/v1/missions", Some(t), None, Some(create)).await;
    assert_eq!(st, StatusCode::CREATED, "{}", String::from_utf8_lossy(&b));
    let id = json(&b)["id"].as_str().unwrap().to_string();
    let url = format!("/api/v1/missions/{id}/armory");
    let (st, b) = call(app, "PUT", &url, Some(t), None, Some(ARMORY_SEED)).await;
    assert_eq!(st, StatusCode::OK, "seed: {}", String::from_utf8_lossy(&b));
    (id, url)
}

/// How many armory rows the mission actually has, read back through the real GET.
async fn armory_len(app: &Router, url: &str, t: &str) -> usize {
    let (st, b) = call(app, "GET", url, Some(t), None, None).await;
    assert_eq!(st, StatusCode::OK);
    json(&b)["data"].as_array().expect("data array").len()
}

/// T-315 — `PUT /missions/:id/armory` is destroy-then-rewrite: the transaction opens with an
/// unconditional `DELETE FROM mission_armories WHERE mission_id = $1`. So every way the body can
/// be wrong is a way to lose the whole armory, and the armory is not versioned with the mission —
/// there is nothing to roll back to.
///
/// `#[serde(default)]` on `items` made `{}` decode as `items: []`, which is not "the caller said
/// nothing", it is "the caller said the new armory is empty". Four real rows were deleted, nothing
/// was inserted, and the answer was **200**. On base this test fails at the first `{}` assertion
/// with `200 {"data":[]}` and a row count of 0.
///
/// The three sibling vectors (no body, wrong `Content-Type`, malformed JSON) were already safe —
/// this handler kept its `map_err` — and they are asserted here so a future `.ok().unwrap_or_default()`
/// cannot quietly reopen them.
#[tokio::test]
async fn armory_survives_a_body_that_never_mentions_it() {
    let Some((app, tok)) = app_and_token("mission_maker").await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let t = tok.as_str();
    let (_, url) = mission_with_armory(&app, t).await;
    assert_eq!(armory_len(&app, &url, t).await, 4, "seeded");

    // The ticket: a body that simply never mentions the armory.
    let (st, b) = call_ct(&app, "PUT", &url, t, Some("application/json"), Some("{}")).await;
    assert_eq!(
        st,
        StatusCode::BAD_REQUEST,
        "`{{}}` must not be a wholesale delete: {}",
        String::from_utf8_lossy(&b)
    );
    assert_eq!(
        json(&b)["error"],
        "items is required, and every item needs a faction and an item_name"
    );
    assert_eq!(armory_len(&app, &url, t).await, 4, "`{{}}` kept the rows");

    // No body at all.
    let (st, _) = call_ct(&app, "PUT", &url, t, None, None).await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "missing body");
    assert_eq!(
        armory_len(&app, &url, t).await,
        4,
        "missing body kept the rows"
    );

    // A well-formed body the extractor refuses because the header is wrong.
    let (st, _) = call_ct(
        &app,
        "PUT",
        &url,
        t,
        Some("text/plain"),
        Some(r#"{"items":[]}"#),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "wrong Content-Type");
    assert_eq!(
        armory_len(&app, &url, t).await,
        4,
        "wrong Content-Type kept the rows"
    );

    // Truncated JSON — the shape a dropped connection or a hand-built request produces.
    let (st, _) = call_ct(
        &app,
        "PUT",
        &url,
        t,
        Some("application/json"),
        Some(r#"{"items":["#),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "malformed JSON");
    assert_eq!(
        armory_len(&app, &url, t).await,
        4,
        "malformed kept the rows"
    );

    // The other half: clearing the armory is a legitimate thing to ask for, it just has to be
    // said out loud. Requiring the field must not cost the author the ability to empty it.
    let (st, b) = call(&app, "PUT", &url, Some(t), None, Some(r#"{"items":[]}"#)).await;
    assert_eq!(
        st,
        StatusCode::OK,
        "an explicit empty armory is legitimate: {}",
        String::from_utf8_lossy(&b)
    );
    assert_eq!(armory_len(&app, &url, t).await, 0, "explicit clear applied");
}

/// T-315 — the same mistake one level down. `item_name` was defaulted too, so `{"items":[{}]}`
/// deleted four real rows and inserted a nameless, factionless one: a blank line in the faction
/// dossier that cannot be identified or removed except by replacing the whole armory again.
/// Measured **200** on the pre-fix binary.
///
/// The guard runs before the transaction opens, so a rejected item never reaches the DELETE at
/// all rather than relying on the rollback, and it trims — otherwise `" "` is refused while
/// `" M4A1 "` is stored with its padding and never matches anything.
#[tokio::test]
async fn armory_item_without_a_name_is_refused_before_the_delete() {
    let Some((app, tok)) = app_and_token("mission_maker").await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let t = tok.as_str();
    let (_, url) = mission_with_armory(&app, t).await;

    // An item with no fields at all now fails to decode — `item_name` and, since T-346, `faction`
    // are both required at the type level — so this one is caught by the extractor, not the
    // positional guard below.
    let (st, b) = call(&app, "PUT", &url, Some(t), None, Some(r#"{"items":[{}]}"#)).await;
    assert_eq!(
        st,
        StatusCode::BAD_REQUEST,
        "a nameless item must not replace the armory: {}",
        String::from_utf8_lossy(&b)
    );
    assert_eq!(
        json(&b)["error"],
        "items is required, and every item needs a faction and an item_name"
    );
    assert_eq!(armory_len(&app, &url, t).await, 4, "rows untouched");

    // A whitespace-only name decodes fine and is the same lie, so the runtime guard catches it —
    // and names which item is at fault, because a 30-item armory rejected as one opaque 400 is a
    // bug report, not a diagnostic.
    let padded =
        r#"{"items":[{"faction":"USA","item_name":"M4A1"},{"faction":"USA","item_name":"   "}]}"#;
    let (st, b) = call(&app, "PUT", &url, Some(t), None, Some(padded)).await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "blank name");
    assert_eq!(json(&b)["error"], "items[1].item_name is required");
    assert_eq!(armory_len(&app, &url, t).await, 4, "rows untouched");

    // A real name that arrived with padding is accepted and stored trimmed, so the stored value
    // agrees with the value the guard tested.
    let ok = r#"{"items":[{"faction":"USA","category":"rifle","item_name":"  M4A1  ","quantity":2,"sort_order":0}]}"#;
    let (st, b) = call(&app, "PUT", &url, Some(t), None, Some(ok)).await;
    assert_eq!(st, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    assert_eq!(json(&b)["data"][0]["item_name"], "M4A1");
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

/// Replay of the Event Hub's own resolution, so the T-346 test below measures **what a player
/// sees** rather than what the column happens to hold.
///
/// `event_hub.rs:294-302` picks the faction list the dossier renders — the mission's `orbat_slots`
/// factions, falling back to the armory's own keys only when that list is empty — and `:412-418`
/// then fills each card by `find`ing the armory group whose `faction` is **byte-equal** to it.
/// Returns `(faction, items_that_card_renders)` per card.
fn event_hub_cards(dossier: &Value) -> Vec<(String, usize)> {
    let armory = dossier["armory_by_faction"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let listed: Vec<String> = dossier["factions"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|f| f.as_str().unwrap_or_default().to_string())
        .collect();
    let faction_list: Vec<String> = if listed.is_empty() {
        armory
            .iter()
            .map(|g| g["faction"].as_str().unwrap_or_default().to_string())
            .collect()
    } else {
        listed
    };
    faction_list
        .into_iter()
        .map(|f| {
            let rendered = armory
                .iter()
                .find(|g| g["faction"].as_str() == Some(f.as_str()))
                .and_then(|g| g["items"].as_array())
                .map(|i| i.len())
                .unwrap_or(0);
            (f, rendered)
        })
        .collect()
}

/// The seeded mission attached to a fresh event under an ORBAT that declares `faction`. That ORBAT
/// is what puts `faction` into the dossier's `factions` list (`events.rs:894` reads `orbat_slots`),
/// which is what makes it the join key. Returns `(armory_url, event_id)`.
async fn mission_in_event_with_orbat_faction(
    app: &Router,
    t: &str,
    faction: &str,
) -> (String, String) {
    let (mid, url) = mission_with_armory(app, t).await;
    let (_, b) = call(
        app,
        "POST",
        "/api/v1/events",
        Some(t),
        None,
        Some(r#"{"start_time":"2027-07-01T00:00:00Z"}"#),
    )
    .await;
    let eid = b_id(&b);
    let attach = format!(
        r#"{{"mission_id":"{mid}","start_time":"2027-07-01T00:00:00Z","orbat":[{{"faction":"{faction}","callsign":"ALPHA","squad":"Alpha 1-1","slots":[{{"role":"SL"}},{{"role":"RTO"}}]}}]}}"#
    );
    let (st, b) = call(
        app,
        "POST",
        &format!("/api/v1/events/{eid}/missions"),
        Some(t),
        None,
        Some(&attach),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::CREATED,
        "attach: {}",
        String::from_utf8_lossy(&b)
    );
    (url, eid)
}

fn b_id(bytes: &[u8]) -> String {
    json(bytes)["id"].as_str().unwrap().to_string()
}

/// The single mission dossier of a one-mission event.
async fn dossier(app: &Router, eid: &str, t: &str) -> Value {
    let (st, b) = call(
        app,
        "GET",
        &format!("/api/v1/events/{eid}"),
        Some(t),
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "hub: {}", String::from_utf8_lossy(&b));
    json(&b)["missions"][0].clone()
}

/// T-346 — `faction` is the Event Hub's **join key**, not a presentation hint, and it was both
/// `#[serde(default)]` and bound untrimmed two lines above the correctly-trimmed `item_name`.
///
/// The harm is not "a column has a space in it". `get_event` groups the armory by `faction`
/// (`events.rs:796`) and the SPA matches those groups against the mission's `orbat_slots` factions
/// by exact equality (`event_hub.rs:415`) — a different table. A `faction` that does not match one
/// byte-for-byte renders a dossier card with **no items**: the author gets 200 and their own value
/// echoed back, the players get an empty armory.
///
/// Measured on the pre-fix binary, ORBAT declaring `USA`, four seeded rows:
/// - `faction: "  USA  "` → **200**, stored `"  USA  "`, the USA card rendered **0** items.
/// - `{"items":[{"item_name":"M4A1"}]}` → **200**, stored `""`, the USA card rendered **0** items.
///
/// The second needs **no whitespace at all** — it is the `#[serde(default)]`, so a trim-only fix
/// would have looked like it worked while leaving that half fully broken.
///
/// **A padded `faction` is refused, not trimmed**, and that is the whole point. The other side of
/// the join is written verbatim (`events.rs:391` ← `orbat.rs:23-25`, itself defaulted and
/// untrimmed), so trimming only here would make the two sites *disagree*: measured on the pre-fix
/// binary, an ORBAT declaring `"  USA  "` with an armory row `"  USA  "` renders **correctly
/// today**, and a unilateral trim turns that into 0 items — T-343's trap at `events.rs:1735`
/// and `:1923`, reproduced. Refusing keeps the stored bytes exactly what the caller sent, which
/// agrees with the other side whether or not it ever starts trimming.
#[tokio::test]
async fn armory_faction_is_the_event_hub_join_key() {
    let Some((app, tok)) = app_and_token("admin").await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let t = tok.as_str();
    let (url, eid) = mission_in_event_with_orbat_faction(&app, t, "USA").await;

    // The measurement is only meaningful if the join works when both sides agree.
    assert_eq!(
        event_hub_cards(&dossier(&app, &eid, t).await),
        vec![("USA".to_string(), 2)],
        "baseline: the USA card renders its two seeded rows"
    );

    // Half one — the `#[serde(default)]`, reachable with no whitespace anywhere. Pre-fix this
    // answered 200, stored `""`, and emptied the card.
    let defaulted = r#"{"items":[{"category":"rifle","item_name":"M4A1","sort_order":0}]}"#;
    let (st, b) = call(&app, "PUT", &url, Some(t), None, Some(defaulted)).await;
    assert_eq!(
        st,
        StatusCode::BAD_REQUEST,
        "an item that never names a faction must not replace the armory: {}",
        String::from_utf8_lossy(&b)
    );
    assert_eq!(
        json(&b)["error"],
        "items is required, and every item needs a faction and an item_name"
    );
    assert_eq!(armory_len(&app, &url, t).await, 4, "rows untouched");

    // Half two — padding. Refused rather than canonicalised, so the stored bytes never diverge
    // from what `orbat_slots` holds.
    let padded =
        r#"{"items":[{"faction":"  USA  ","category":"rifle","item_name":"M4A1","sort_order":0}]}"#;
    let (st, b) = call(&app, "PUT", &url, Some(t), None, Some(padded)).await;
    assert_eq!(
        st,
        StatusCode::BAD_REQUEST,
        "a padded faction must not be stored: {}",
        String::from_utf8_lossy(&b)
    );
    assert_eq!(
        json(&b)["error"],
        "items[0].faction must not have leading or trailing whitespace"
    );
    assert_eq!(armory_len(&app, &url, t).await, 4, "rows untouched");

    // Whitespace-only decodes fine and is the same lie as no faction, so the runtime guard takes
    // it — and names which item, because a 30-item armory rejected as one opaque 400 is a bug
    // report, not a diagnostic.
    let blank =
        r#"{"items":[{"faction":"USA","item_name":"M4A1"},{"faction":"   ","item_name":"AT4"}]}"#;
    let (st, b) = call(&app, "PUT", &url, Some(t), None, Some(blank)).await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "blank faction");
    assert_eq!(json(&b)["error"], "items[1].faction is required");
    assert_eq!(armory_len(&app, &url, t).await, 4, "rows untouched");

    // The dossier still populates for a body that states the faction the ORBAT actually declares —
    // requiring the field must not cost the author a working armory.
    let ok = r#"{"items":[
        {"faction":"USA","category":"rifle","item_name":"  M4A1  ","quantity":24,"sort_order":0},
        {"faction":"USA","category":"launcher","item_name":"AT4","quantity":6,"sort_order":1},
        {"faction":"USSR","category":"rifle","item_name":"AK-74","quantity":30,"sort_order":2}]}"#;
    let (st, b) = call(&app, "PUT", &url, Some(t), None, Some(ok)).await;
    assert_eq!(st, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    assert_eq!(json(&b)["data"][0]["faction"], "USA", "stored verbatim");
    assert_eq!(json(&b)["data"][0]["item_name"], "M4A1", "name trimmed");
    assert_eq!(
        event_hub_cards(&dossier(&app, &eid, t).await),
        vec![("USA".to_string(), 2)],
        "the USA card renders its two rows again, and does not absorb USSR's"
    );

    // And the guard is `!= trim()`, not "contains a space": a faction whose name legitimately has
    // interior whitespace is stored byte-identical. An over-strict fix fails here.
    let interior =
        r#"{"items":[{"faction":"US Army","category":"rifle","item_name":"M4A1","sort_order":0}]}"#;
    let (st, b) = call(&app, "PUT", &url, Some(t), None, Some(interior)).await;
    assert_eq!(st, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    assert_eq!(json(&b)["data"][0]["faction"], "US Army");
}
