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
