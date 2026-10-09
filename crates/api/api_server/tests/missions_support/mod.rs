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

use api_configuration::configuration::Config;

use api_server::router::router;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use tower::ServiceExt;

use crate::common;

pub(crate) async fn app_and_token(role: &str) -> Option<(Router, String)> {
    let url = common::require_test_database_url()?;
    Some(app_and_token_on(url, role).await)
}

/// [`app_and_token`] over the database at `url`.
async fn app_and_token_on(url: String, role: &str) -> (Router, String) {
    let pool = api_database::connect(&url).await.expect("connect");
    api_database::migrate(&pool).await.expect("migrate");
    let app = router(api_server::composition::application_state(
        pool,
        Config::for_tests(url, "missions-secret"),
    ));
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/v1/auth/dev-login?role={role}"))
                .body(Body::empty())
                .expect("the request builds"),
        )
        .await
        .unwrap();
    let loc = resp.headers()[header::LOCATION]
        .to_str()
        .expect("the Location header is ASCII");
    let tok = loc
        .split_once('#')
        .expect("the Location carries a fragment")
        .1
        .split('&')
        .find_map(|p| p.strip_prefix("access_token="))
        .expect("the fragment carries an access token")
        .to_string();
    (app, tok)
}

/// One request with an optional bearer, one optional extra `(name, value)` header and an
/// optional JSON body; answers the status and the raw body bytes.
pub(crate) async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    bearer: Option<&str>,
    extra_header: Option<(&str, &str)>,
    body: Option<&str>,
) -> (StatusCode, Vec<u8>) {
    let mut b = Request::builder().method(method).uri(uri);
    if let Some(t) = bearer {
        b = b.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    if let Some((name, value)) = extra_header {
        b = b.header(name, value);
    }
    if body.is_some() {
        b = b.header(header::CONTENT_TYPE, "application/json");
    }
    let req = b
        .body(body.map_or(Body::empty(), |s| Body::from(s.to_string())))
        .expect("the request builds");
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("the response body reads to the end")
        .to_vec();
    (status, bytes)
}

pub(crate) fn json(bytes: &[u8]) -> Value {
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
/// Same shape as the approvals ratchet (`admin_approvals_cms_field_tools::find_in_approvals`).
pub(crate) async fn find_id_in_missions_list(
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
/// `admin_approvals_cms_field_tools::find_in_approvals`. A page-1-only lookup fails on a shared
/// database the moment
/// queue residue passes 20 rows.
pub(crate) async fn find_in_approvals(
    app: &Router,
    admin: &str,
    mission_id: &str,
) -> Option<Value> {
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

/// Save [`common::COMPILABLE_EDITOR_PAYLOAD`] as version `semver` of the mission, which makes it the
/// current version a submission compiles. Returns the version id.
pub(crate) async fn save_compilable_version(
    app: &Router,
    bearer: &str,
    mission_id: &str,
    semver: &str,
) -> String {
    let body = format!(
        r#"{{"semver":"{semver}","payload":{}}}"#,
        common::COMPILABLE_EDITOR_PAYLOAD
    );
    let (st, b) = call(
        app,
        "POST",
        &format!("/api/v1/missions/{mission_id}/versions"),
        Some(bearer),
        None,
        Some(&body),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::CREATED,
        "save version: {}",
        String::from_utf8_lossy(&b)
    );
    json(&b)["id"]
        .as_str()
        .expect("the `id` field is a string")
        .to_string()
}

/// `call` with the `Content-Type` under the caller's control, and a body that can be sent
/// without one at all (`ct: None`). `call` always pairs a body with `application/json`, which
/// is exactly the header a fat-fingered client gets wrong, so the malformed-body cases in the
/// armory suite cannot be expressed through it.
pub(crate) async fn call_ct(
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
        .expect("the request builds");
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("the response body reads to the end")
        .to_vec();
    (status, bytes)
}

/// The four-item armory every armory case starts from.
pub(crate) const ARMORY_SEED: &str = r#"{"items":[
    {"faction":"USA","category":"rifle","item_name":"M4A1","quantity":24,"icon":"m4.png","sort_order":0},
    {"faction":"USA","category":"launcher","item_name":"AT4","quantity":6,"icon":"at4.png","sort_order":1},
    {"faction":"USSR","category":"rifle","item_name":"AK-74","quantity":30,"icon":"ak74.png","sort_order":2},
    {"faction":"USSR","category":"mg","item_name":"PKM","quantity":4,"icon":"pkm.png","sort_order":3}]}"#;

/// Create a mission with the seeded armory and return `(id, armory_url)`.
pub(crate) async fn mission_with_armory(app: &Router, t: &str) -> (String, String) {
    let create =
        r#"{"title":"Armory Op","terrain":"everon","game_mode":"pve_coop","max_players":16}"#;
    let (st, b) = call(app, "POST", "/api/v1/missions", Some(t), None, Some(create)).await;
    assert_eq!(st, StatusCode::CREATED, "{}", String::from_utf8_lossy(&b));
    let id = json(&b)["id"]
        .as_str()
        .expect("the `id` field is a string")
        .to_string();
    let url = format!("/api/v1/missions/{id}/armory");
    let (st, b) = call(app, "PUT", &url, Some(t), None, Some(ARMORY_SEED)).await;
    assert_eq!(st, StatusCode::OK, "seed: {}", String::from_utf8_lossy(&b));
    (id, url)
}

/// How many armory rows the mission actually has, read back through the real GET.
pub(crate) async fn armory_len(app: &Router, url: &str, t: &str) -> usize {
    let (st, b) = call(app, "GET", url, Some(t), None, None).await;
    assert_eq!(st, StatusCode::OK);
    json(&b)["data"].as_array().expect("data array").len()
}

pub(crate) fn b_id(bytes: &[u8]) -> String {
    json(bytes)["id"]
        .as_str()
        .expect("the `id` field is a string")
        .to_string()
}

/// The single mission dossier of a one-mission event.
pub(crate) async fn dossier(app: &Router, eid: &str, t: &str) -> Value {
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
pub(crate) fn event_hub_cards(dossier: &Value) -> Vec<(String, usize)> {
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
pub(crate) async fn mission_in_event_with_orbat_faction(
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
pub(crate) async fn app_pool_and_tokens() -> Option<(Router, sqlx::PgPool, String, String)> {
    let url = common::require_test_database_url()?;
    Some(app_pool_and_tokens_on(url).await)
}

/// [`app_pool_and_tokens`] on an isolated database of the binary
/// ([`common::require_isolated_test_database_url`]), for a test that changes rows every compile
/// reads, such as the current modpack.
pub(crate) async fn isolated_app_pool_and_tokens(
    scope: &str,
) -> (Router, sqlx::PgPool, String, String) {
    app_pool_and_tokens_on(common::require_isolated_test_database_url(scope)).await
}

async fn app_pool_and_tokens_on(url: String) -> (Router, sqlx::PgPool, String, String) {
    let (app, maker) = app_and_token_on(url.clone(), "mission_maker").await;
    let (_, admin) = app_and_token_on(url.clone(), "admin").await;
    let pool = api_database::connect(&url).await.expect("connect");
    (app, pool, maker, admin)
}

/// Default library page (`limit` omitted → 20, `offset` omitted → 0). The overflow suite uses
/// it to prove page-1 membership is insufficient without shared-DB residue.
pub(crate) async fn id_on_default_missions_page1(
    app: &Router,
    bearer: &str,
    uri_base: &str,
    id: &str,
) -> bool {
    let sep = if uri_base.contains('?') { '&' } else { '?' };
    // Explicit limit=20 matches the `ListQuery` default (`api_missions::handlers::mission_library`);
    // omit would
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
