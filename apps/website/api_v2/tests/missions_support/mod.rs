//! Shared harness for the mission suites: router boot, the request helper every
//! assertion goes through, the paginating list/queue lookups, and the seeded
//! mission/event/armory fixtures.
//!
//! # This directory contributes no test binary
//!
//! Cargo builds one test target per top-level `tests/*.rs` file; files under a
//! `tests/<dir>/` subdirectory are compiled *into* whichever suite writes
//! `mod missions_support;` and add no target of their own.

// Each mission suite compiles its own copy of this module and uses a different subset of
// it, so a helper unused by one suite is not dead code — but rustc judges each binary on
// its own and the gate runs `clippy --all-targets -- -D warnings`.
#![allow(dead_code)]

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use tower::ServiceExt;
use website_api::core::application_state::AppState;
use website_api::core::configuration::Config;
use website_api::core::database;
use website_api::core::http_router;

use crate::common;

pub async fn app_and_token(role: &str) -> Option<(Router, String)> {
    let url = common::require_test_database_url()?;
    let pool = database::connect(&url).await.expect("connect");
    database::migrate(&pool).await.expect("migrate");
    let app = http_router::router(AppState::new(
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

pub async fn call(
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

pub fn json(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap_or(Value::Null)
}

/// Walk a `{data,total,limit,offset}` missions list until `id` appears or the pages end.
///
/// **Do not shrink this back to "is it on page 1".** `list_missions` is
/// `ORDER BY updated_at DESC` with default `LIMIT 20`. A freshly created row lands on
/// page 1 whenever fewer than 20 rows sort ahead of it — which is the steady state of a
/// clean gate DB. Page-1 `.any(id)` therefore stays green without shared-DB residue and
/// does not pin the helper. The overflow suite plants >20 newer-`updated_at` fillers so
/// default page 1 misses while this walk still finds.
/// Same shape as the approvals ratchet (`admin_field::find_in_approvals`).
pub async fn find_id_in_missions_list(
    app: &Router,
    bearer: &str,
    uri_base: &str,
    id: &str,
) -> bool {
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
/// `admin_field::find_in_approvals`. A page-1-only lookup reds a shared gate DB the moment
/// queue residue passes 20 rows.
pub async fn find_in_approvals(app: &Router, admin: &str, mission_id: &str) -> Option<Value> {
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
/// is exactly the header a fat-fingered client gets wrong, so the malformed-body cases in the
/// armory suite cannot be expressed through it.
pub async fn call_ct(
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

/// The four-item armory every armory case starts from.
pub const ARMORY_SEED: &str = r#"{"items":[
    {"faction":"USA","category":"rifle","item_name":"M4A1","quantity":24,"icon":"m4.png","sort_order":0},
    {"faction":"USA","category":"launcher","item_name":"AT4","quantity":6,"icon":"at4.png","sort_order":1},
    {"faction":"USSR","category":"rifle","item_name":"AK-74","quantity":30,"icon":"ak74.png","sort_order":2},
    {"faction":"USSR","category":"mg","item_name":"PKM","quantity":4,"icon":"pkm.png","sort_order":3}]}"#;

/// Create a mission with the seeded armory and return `(id, armory_url)`.
pub async fn mission_with_armory(app: &Router, t: &str) -> (String, String) {
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
pub async fn armory_len(app: &Router, url: &str, t: &str) -> usize {
    let (st, b) = call(app, "GET", url, Some(t), None, None).await;
    assert_eq!(st, StatusCode::OK);
    json(&b)["data"].as_array().expect("data array").len()
}

pub fn b_id(bytes: &[u8]) -> String {
    json(bytes)["id"].as_str().unwrap().to_string()
}

/// The single mission dossier of a one-mission event.
pub async fn dossier(app: &Router, eid: &str, t: &str) -> Value {
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

/// Replay of the Event Hub's own resolution, so the armory-faction test measures **what a player
/// sees** rather than what the column happens to hold.
///
/// `event_hub.rs:294-302` picks the faction list the dossier renders — the mission's `orbat_slots`
/// factions, falling back to the armory's own keys only when that list is empty — and `:412-418`
/// then fills each card by `find`ing the armory group whose `faction` is **byte-equal** to it.
/// Returns `(faction, items_that_card_renders)` per card.
pub fn event_hub_cards(dossier: &Value) -> Vec<(String, usize)> {
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
pub async fn mission_in_event_with_orbat_faction(
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

/// One app, one pool, and BOTH tokens — `mission_maker` first, `admin` second.
///
/// Each role has its own discord_id (`…004` maker, `…001` admin), so the second login does
/// not rewrite the first row's role. Both JWTs therefore address **distinct** users. A third
/// author still needs a direct INSERT when the case needs a mission_maker who is neither the
/// maker nor the admin under test.
pub async fn app_pool_and_tokens() -> Option<(Router, sqlx::PgPool, String, String)> {
    let url = common::require_test_database_url()?;
    let (app, maker) = app_and_token("mission_maker").await?;
    let (_, admin) = app_and_token("admin").await?;
    let pool = database::connect(&url).await.expect("connect");
    Some((app, pool, maker, admin))
}

/// Default library page (`limit` omitted → 20, `offset` omitted → 0). The overflow suite uses
/// it to prove page-1 membership is insufficient without shared-DB residue.
pub async fn id_on_default_missions_page1(
    app: &Router,
    bearer: &str,
    uri_base: &str,
    id: &str,
) -> bool {
    let sep = if uri_base.contains('?') { '&' } else { '?' };
    // Explicit limit=20 matches `ListQuery` default (`handlers/missions.rs`); omit would
    // also be 20, but spelling it makes the page-1 contract obvious in failures.
    let uri = format!("{uri_base}{sep}limit=20&offset=0");
    let (st, b) = call(app, "GET", &uri, Some(bearer), None, None).await;
    assert_eq!(
        st,
        StatusCode::OK,
        "default page-1 {uri}: {}",
        String::from_utf8_lossy(&b)
    );
    json(&b)["data"]
        .as_array()
        .unwrap_or_else(|| panic!("{uri} missing data: {}", json(&b)))
        .iter()
        .any(|r| r["id"].as_str() == Some(id))
}

/// Seed a mission whose LATEST version stores exactly one authored zone carrying `rules`, then
/// return its id. Inserts the version directly and points `current_version_id` at it — the same
/// bypass the over-capacity test uses, because the editor has no zone-rules draw tool, so the
/// only way a `zones[].rules` payload reaches the DB in a test is a direct insert.
pub async fn seed_zone_rules_mission(pool: &sqlx::PgPool, rules_json: &str) -> String {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    // Insert the mission row directly (the seed helper takes `&pool`, not the router). `author_id`
    // mirrors dev-login's admin/maker seed id; status draft so it never leaks into a live-only list.
    let id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO missions \
         (id, title, author_id, terrain, game_mode, weather, time_of_day, max_players, status, created_at, updated_at) \
         VALUES ($1, $2, '000000000000000001', 'everon', 'pve_coop', 'clear', '14:00:00'::time, 16, 'draft', now(), now())",
    )
    .bind(id)
    .bind(format!("T683 Zone {stamp}"))
    .execute(pool)
    .await
    .expect("seed mission row");
    let mid = id.to_string();

    // One authored zone with the given rules. `zones` at the editor-payload ROOT (flatten.rs
    // `EditorPayload.zones`), each element's `rules` object — the exact path the handler reads.
    let payload = format!(
        r#"{{"schemaVersion":1,"zones":[{{"id":"z1","type":"objective_capture","label":"OBJ","faction":"BLUFOR","shape":{{"circle":{{"x":100.0,"z":100.0,"r":50.0}}}},"rules":{rules_json}}}],"editor":{{"factions":[],"squads":[],"slots":[],"editorLayers":[]}}}}"#
    );
    let vid = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO mission_versions (id, mission_id, semver, json_payload, editor_notes, created_by, created_at) \
         VALUES ($1, $2::uuid, '0.1.0', $3::jsonb, '', '000000000000000001', now())",
    )
    .bind(vid)
    .bind(&mid)
    .bind(&payload)
    .execute(pool)
    .await
    .expect("insert zone-rules version");
    sqlx::query("UPDATE missions SET current_version_id = $1 WHERE id = $2::uuid")
        .bind(vid)
        .bind(&mid)
        .execute(pool)
        .await
        .expect("point tip at zone-rules version");
    mid
}

/// Find one `data[]` row by its `key` in the overrides response.
pub fn find_override<'a>(body: &'a Value, key: &str) -> Option<&'a Value> {
    body["data"]
        .as_array()?
        .iter()
        .find(|r| r["key"].as_str() == Some(key))
}
