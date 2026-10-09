//! Mission archive lifecycle: an archived mission leaves the library and restores, and an
//! upcoming event that names the mission blocks its archive.

use crate::common;

use api_configuration::configuration::Config;

use api_server::router::router;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

async fn boot() -> Option<(Router, PgPool)> {
    let url = common::require_test_database_url()?;
    let pool = api_database::connect(&url).await.expect("connect");
    api_database::migrate(&pool).await.expect("migrate");
    let app = router(api_server::composition::application_state(
        pool.clone(),
        Config::for_tests(url, "lx-secret"),
    ));
    Some((app, pool))
}

async fn tok(app: &Router, role: &str) -> String {
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
    loc.split_once('#')
        .expect("the Location carries a fragment")
        .1
        .split('&')
        .find_map(|p| p.strip_prefix("access_token="))
        .expect("the fragment carries an access token")
        .to_string()
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    t: &str,
    body: Option<&str>,
) -> (StatusCode, Value) {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {t}"));
    if body.is_some() {
        b = b.header(header::CONTENT_TYPE, "application/json");
    }
    let req = b
        .body(body.map_or(Body::empty(), |s| Body::from(s.to_string())))
        .expect("the request builds");
    let resp = app.clone().oneshot(req).await.unwrap();
    let st = resp.status();
    let by = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("the response body reads to the end");
    (st, serde_json::from_slice(&by).unwrap_or(Value::Null))
}

async fn mk_mission(app: &Router, t: &str) -> String {
    let (_, m) = call(
        app,
        "POST",
        "/api/v1/missions",
        t,
        Some(r#"{"title":"LX","terrain":"everon","game_mode":"pve_coop","max_players":16}"#),
    )
    .await;
    m["id"]
        .as_str()
        .expect("the `id` field is a string")
        .to_string()
}

#[tokio::test]
async fn mission_archive_lifecycle() {
    let Some((app, _)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let t = tok(&app, "admin").await;
    let id = mk_mission(&app, &t).await;
    let (st, r) = call(
        &app,
        "PATCH",
        &format!("/api/v1/missions/{id}"),
        &t,
        Some(r#"{"status":"archived"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(r["status"], "archived");
    // archived → live directly is not a permitted transition.
    let (st, _) = call(
        &app,
        "PATCH",
        &format!("/api/v1/missions/{id}"),
        &t,
        Some(r#"{"status":"live"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    // archived → draft (unarchive) is.
    let (st, r) = call(
        &app,
        "PATCH",
        &format!("/api/v1/missions/{id}"),
        &t,
        Some(r#"{"status":"draft"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(r["status"], "draft");
}

#[tokio::test]
async fn mission_archive_blocked_by_upcoming_event() {
    let Some((app, _)) = boot().await else { return };
    let t = tok(&app, "admin").await;
    let id = mk_mission(&app, &t).await;
    let (_, e) = call(
        &app,
        "POST",
        "/api/v1/events",
        &t,
        Some(r#"{"start_time":"2030-01-01T00:00:00Z"}"#),
    )
    .await;
    let eid = e["id"].as_str().unwrap();
    // The ORBAT is explicit, and the attach status is ASSERTED. `mk_mission` publishes a
    // version with no ORBAT, so attaching it with an empty body and a discarded status would
    // silently materialize a **zero-slot** event mission. Such an attach is a 409
    // ("this mission's ORBAT describes no slots"), because such an operation seats nobody and
    // yet refuses nobody: capacity is `count(orbat_slots) = 0` and the registration guard reads
    // `capacity > 0 && registered >= capacity`, which at 0 is simply off. This test is about
    // ARCHIVE BLOCKING and wants a real attached operation; discarding the status is what would
    // let its setup depend on that hole.
    let attach = format!(
        r#"{{"mission_id":"{id}","start_time":"2030-01-01T00:00:00Z","orbat":[{{"faction":"USA","callsign":"A","squad":"LX Alpha","slots":[{{"role":"SL"}}]}}]}}"#
    );
    let (st, b) = call(
        &app,
        "POST",
        &format!("/api/v1/events/{eid}/missions"),
        &t,
        Some(&attach),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "attach the operation: {b}");
    let (st, _) = call(
        &app,
        "PATCH",
        &format!("/api/v1/missions/{id}"),
        &t,
        Some(r#"{"status":"archived"}"#),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::CONFLICT,
        "archive blocked by upcoming event"
    );
}
