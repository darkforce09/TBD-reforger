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

/// Walk a `{data,total,limit,offset}` missions list until `id` appears or the pages end.
///
/// **Do not shrink this back to "is it on page 1" (T-410).** `list_missions` is
/// `ORDER BY updated_at DESC LIMIT 20`, and `updated_at` is nullable with no default —
/// Postgres sorts NULLS FIRST on DESC. Residue missions with NULL `updated_at` occupy
/// page 1 forever; a freshly created row is not guaranteed to be on it. Same shape as
/// the T-399 approvals ratchet (`admin_field::find_in_approvals`).
async fn find_id_in_missions_list(app: &Router, bearer: &str, uri_base: &str, id: &str) -> bool {
    const PAGE: usize = 100;
    const MAX_OFFSET: usize = 1_000;
    let mut offset = 0usize;
    let sep = if uri_base.contains('?') { '&' } else { '?' };
    loop {
        let uri = format!("{uri_base}{sep}limit={PAGE}&offset={offset}");
        let (st, b) = call(app, "GET", &uri, Some(bearer), None, None).await;
        assert_eq!(
            st,
            StatusCode::OK,
            "missions list at offset {offset}: {}",
            String::from_utf8_lossy(&b)
        );
        let body = json(&b);
        let rows = body["data"]
            .as_array()
            .unwrap_or_else(|| panic!("{uri} missing data: {body}"));
        if rows.iter().any(|r| r["id"].as_str() == Some(id)) {
            return true;
        }
        if rows.len() < PAGE {
            return false;
        }
        offset += rows.len();
        assert!(
            offset < MAX_OFFSET,
            "{uri_base} paging never terminated (offset {offset}) — is LIMIT being applied?"
        );
    }
}

/// Walk `GET /approvals` pages until `mission_id` appears. Twin of
/// `admin_field::find_in_approvals` (T-399); the submit-path test in this file still
/// asserted page 1 only and reds shared gate DBs once residue passes 20 (T-410).
async fn find_in_approvals(app: &Router, admin: &str, mission_id: &str) -> Option<Value> {
    const PAGE: usize = 100;
    const MAX_OFFSET: usize = 1_000;
    let mut offset = 0usize;
    loop {
        let uri = format!("/api/v1/approvals?limit={PAGE}&offset={offset}");
        let (st, b) = call(app, "GET", &uri, Some(admin), None, None).await;
        assert_eq!(
            st,
            StatusCode::OK,
            "approvals at offset {offset}: {}",
            String::from_utf8_lossy(&b)
        );
        let body = json(&b);
        let rows = body["data"]
            .as_array()
            .unwrap_or_else(|| panic!("approvals page has no `data` array: {body}"));
        if let Some(row) = rows.iter().find(|r| r["mission_id"] == mission_id) {
            return Some(row.clone());
        }
        if rows.len() < PAGE {
            return None;
        }
        offset += rows.len();
        assert!(
            offset < MAX_OFFSET,
            "approvals paging never terminated (offset {offset}) — is LIMIT being applied?"
        );
    }
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

    // Library list envelope — paginate; page-1 `.any(id)` ratchets on NULL updated_at (T-410).
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
    // Bookmarked scope — same ORDER BY / LIMIT ratchet as the global list (T-410).
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

/// One app, one pool, and BOTH tokens — minted `mission_maker` first, `admin` second.
///
/// `dev-login` upserts a single user (`handlers/dev.rs:14`) and rewrites its role, so the order is
/// load-bearing: the role lives in the JWT claims (`middleware/auth.rs:50`), never re-read from the
/// DB, so the first token stays a `mission_maker` token after the second call promotes the row. Both
/// tokens therefore address the SAME `discord_id` — which is why the non-author case below needs a
/// mission inserted for a second author directly, and cannot be expressed with a second dev-login.
async fn app_pool_and_tokens() -> Option<(Router, sqlx::PgPool, String, String)> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let (app, maker) = app_and_token("mission_maker").await?;
    let (_, admin) = app_and_token("admin").await?;
    let pool = db::connect(&url).await.expect("connect");
    Some((app, pool, maker, admin))
}

/// The mission approval state machine, driven end to end — T-234.
///
/// The queue can only ever contain what `POST /missions/:id/submit` put there: `apply_status_patch`
/// refuses `pending_approval` (asserted below), so this route is the sole writer and its authors are
/// the sole source of `GET /approvals`. That makes three things worth pinning, each of which was
/// wrong or absent before T-234:
///
/// 1. **A resubmission clears the whole previous review round.** Only `rejection_reason` was wiped,
///    so a rejected-then-resubmitted mission was served as `pending_approval` while still carrying
///    the earlier reviewer's `reviewed_by`/`reviewed_at` — "already reviewed" on a mission awaiting
///    review, and a stale ordering key for anything that ever lists reviewed missions.
/// 2. **The submission is audited.** `mission.approve` and `mission.reject` both write an audit row;
///    the transition that CREATES the reviewer's work did not, and the mission row has no
///    `submitted_by`/`submitted_at` of its own — so nothing recorded who queued it.
/// 3. **Only the author or an admin may submit, and only from `draft`/`rejected`.**
#[tokio::test]
async fn mission_submit_is_the_only_door_into_the_approvals_queue() {
    let Some((app, pool, maker, admin)) = app_pool_and_tokens().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    // The title carries a run-unique, URL-safe stamp because the audit assertion below reads the
    // log back through `?q=` (an ILIKE on `message`, the only server-side filter there is) and then
    // counts the hits. A fixed title would also match every previous run's rows in a test database
    // the suite never truncates, and the count would drift upward run over run.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let title = format!("T234-Submit-Path-{stamp}");
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
    let submit = format!("/api/v1/missions/{mid}/submit");

    // PATCH is not a second door: the only status values it accepts are `archived` and `draft`
    // (`apply_status_patch`). If this ever starts returning 200, the queue has a writer that skips
    // every guard below and this test's premise is void.
    let (st, b) = call(
        &app,
        "PATCH",
        &format!("/api/v1/missions/{mid}"),
        Some(&maker),
        None,
        Some(r#"{"status":"pending_approval"}"#),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::BAD_REQUEST,
        "PATCH must not be able to enqueue: {}",
        String::from_utf8_lossy(&b)
    );

    // --- draft -> pending, and it reaches the admin queue ---
    let (st, b) = call(&app, "POST", &submit, Some(&maker), None, None).await;
    assert_eq!(st, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    assert_eq!(json(&b)["status"], "pending_approval");

    // The author cannot read the queue they just fed — it is admin-tier.
    let (st, _) = call(&app, "GET", "/api/v1/approvals", Some(&maker), None, None).await;
    assert_eq!(st, StatusCode::FORBIDDEN, "queue must be admin-only");

    // Paginate the queue — page-1 LIMIT 20 reds once residue > 20 (measured total 26 on
    // tbd_gate_it during wave 5; T-399 missed this site, T-410 closes it).
    let row = find_in_approvals(&app, &admin, &mid)
        .await
        .unwrap_or_else(|| panic!("submitted mission missing from GET /approvals (paginated)"));
    assert_eq!(row["title"], title.as_str());
    assert!(
        row["submitted_at"]
            .as_str()
            .is_some_and(|s| s.ends_with('Z')),
        "the queue row must carry a submitted_at: {row}"
    );

    // A second submit is a 409, not a duplicate enqueue.
    let (st, _) = call(&app, "POST", &submit, Some(&maker), None, None).await;
    assert_eq!(st, StatusCode::CONFLICT, "double submit");

    // --- reject, then resubmit: the previous review round must be gone ---
    let (st, b) = call(
        &app,
        "POST",
        &format!("/api/v1/approvals/{mid}/reject"),
        Some(&admin),
        None,
        Some(r#"{"reason":"objective markers overlap the spawn"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    assert_eq!(json(&b)["status"], "rejected");
    assert_eq!(
        json(&b)["rejection_reason"],
        "objective markers overlap the spawn"
    );
    assert!(
        json(&b)["reviewed_by"].is_string(),
        "the rejection must stamp a reviewer, or the next assertion proves nothing"
    );

    let (st, b) = call(&app, "POST", &submit, Some(&maker), None, None).await;
    assert_eq!(st, StatusCode::OK, "resubmit a rejected mission");
    let m = json(&b);
    assert_eq!(m["status"], "pending_approval");
    // `skip_serializing_if = "Option::is_none"` means "cleared" is "absent", exactly as on a
    // mission that has never been reviewed — not a literal null.
    assert!(
        m.get("reviewed_by").is_none() && m.get("reviewed_at").is_none(),
        "a resubmitted mission must carry no reviewer stamp: {m}"
    );
    assert!(
        m.get("rejection_reason").is_none(),
        "the old rejection reason must be gone: {m}"
    );

    // --- the submission is on the audit trail ---
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
    let submits: Vec<&Value> = rows["data"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["action"] == "mission.submit" && r["target_id"] == mid.as_str())
        .collect();
    assert_eq!(
        submits.len(),
        2,
        "both the first submit and the resubmit must be audited: {rows}"
    );
    assert_eq!(submits[0]["target_type"], "mission");
    assert_eq!(submits[0]["severity"], "info");

    // --- approve, and then no further submit is possible ---
    let (st, b) = call(
        &app,
        "POST",
        &format!("/api/v1/approvals/{mid}/approve"),
        Some(&admin),
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    assert_eq!(json(&b)["status"], "live");
    let (st, _) = call(&app, "POST", &submit, Some(&maker), None, None).await;
    assert_eq!(
        st,
        StatusCode::CONFLICT,
        "a live mission is not submittable"
    );

    // --- authorisation + the remaining refused source states ---
    // A second author, inserted directly: dev-login only ever mints the one user, so this is the
    // only way to hold a token that is NOT the author's.
    let other = "999000000000000009";
    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_character, role, \
         is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'T234 Other Author', 't234other', '', '', 'mission_maker', false, '', now(), now()) \
         ON CONFLICT (discord_id) DO NOTHING",
    )
    .bind(other)
    .execute(&pool)
    .await
    .unwrap();
    let seed = |author: &'static str, status: &'static str| {
        let pool = pool.clone();
        async move {
            sqlx::query_scalar::<_, uuid::Uuid>(
                "INSERT INTO missions (title, author_id, terrain, custom_terrain_name, game_mode, \
                 weather, time_of_day, max_players, status, thumbnail_url, briefing, \
                 rejection_reason, created_at, updated_at) \
                 VALUES ($1, $2, 'everon', '', 'pve_coop', 'clear', '14:00'::time, 10, \
                 $3::mission_status, '', '', '', now(), now()) RETURNING id",
            )
            .bind(format!("T234 {status} by {author}"))
            .bind(author)
            .bind(status)
            .fetch_one(&pool)
            .await
            .unwrap()
            .to_string()
        }
    };

    let foreign = seed(other, "draft").await;
    let (st, b) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{foreign}/submit"),
        Some(&maker),
        None,
        None,
    )
    .await;
    assert_eq!(
        st,
        StatusCode::FORBIDDEN,
        "a mission_maker must not submit someone else's mission: {}",
        String::from_utf8_lossy(&b)
    );
    // The admin override is the same one PATCH and DELETE already grant, and it is deliberate:
    // an admin acting for an author must be able to queue the mission. Pinned so that removing
    // `can_edit`'s admin arm here becomes a visible decision, not a silent one.
    let (st, b) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{foreign}/submit"),
        Some(&admin),
        None,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "{}", String::from_utf8_lossy(&b));
    assert_eq!(json(&b)["status"], "pending_approval");

    let archived = seed("000000000000000001", "archived").await;
    let (st, _) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{archived}/submit"),
        Some(&maker),
        None,
        None,
    )
    .await;
    assert_eq!(
        st,
        StatusCode::CONFLICT,
        "an archived mission is not submittable"
    );
}

/// T-258 — `POST /missions/:id/versions` must bump `missions.updated_at` and write an audit
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
    let title = format!("T258-Version-Save-{stamp}");
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

    let notes = format!("t258 editor notes {stamp}");
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

/// T-512 HTTP IT — T-509 CREATE contract: omitted or `""` weather → 201 + `clear`.
///
/// Class-R in `handlers/missions.rs` pins the handler source; this is the live HTTP layer
/// `admin_field` already exercises incidentally (POST without weather). Explicit pin so a
/// regression that 400s omitted weather cannot hide behind Class-R alone.
///
/// RED (assert-flip): expect `weather == "dense_fog"` on the omit path — fails while production
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
    let title_omit = format!("T512-Create-Omit-{stamp}");
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
        "omitted weather must default to clear (T-509): {omit}"
    );

    // Explicit empty string is the same serde/default path and must also land Clear.
    let title_blank = format!("T512-Create-Blank-{stamp}");
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
        "blank weather on CREATE must default to clear (T-509): {blank}"
    );
}

/// T-512 HTTP IT — T-377 PATCH contract: after `dense_fog`, `{"weather":""}` → 400 and row stays.
///
/// Pre-T-377 `valid_weather` mapped `""` → Clear, so this PATCH answered 200 and rewrote the row.
/// Class-R covers the helper; this IT covers the wire + persistence.
///
/// RED (assert-flip): expect `StatusCode::OK` on the blank PATCH — fails while production 400s.
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
    let title = format!("T512-Patch-Blank-{stamp}");

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
        "blank weather PATCH must 400 (T-377): {}",
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

/// T-505 — `create_version` mirrors a non-blank payload `title` onto `missions.title`.
///
/// Create with a stale library title, Save a version whose payload carries an authored title,
/// then GET the mission row and assert the title moved. Whitespace-only payload title must NOT
/// clobber the row.
///
/// Perturbation RED: drop the `title = $3` arm in `create_version` → first assert fails.
///
/// **Out-of-owns note:** this IT lives in `apps/website/api/tests/missions.rs` (not in the slice
/// owns list). Called out deliberately — Class-R pins in `handlers/missions.rs` alone cannot
/// prove the SQL UPDATE.
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
    let stale = format!("T505-Stale-{stamp}");
    let authored = format!("T505-Authored-{stamp}");
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
