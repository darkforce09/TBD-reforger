//! Mission archive/soft-delete lifecycle, editor-only ORBAT derivation, export
//! edge-cases, version body-limit override, and refresh-token purge. Ports the Go
//! missions_lifecycle / missions_orbat / missions_export / bodylimit / token_purge
//! integration tests. Skips without `TEST_DATABASE_URL`.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
use website_api::config::Config;
use website_api::services::purge_expired_refresh_tokens;
use website_api::state::AppState;
use website_api::{app, db};

const OTHER: &str = "000000000000000007";

async fn boot() -> Option<(Router, PgPool)> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let app = app::router(AppState::new(
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
                .unwrap(),
        )
        .await
        .unwrap();
    let loc = resp.headers()[header::LOCATION].to_str().unwrap();
    loc.split_once('#')
        .unwrap()
        .1
        .split('&')
        .find_map(|p| p.strip_prefix("access_token="))
        .unwrap()
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
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let st = resp.status();
    let by = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
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
    m["id"].as_str().unwrap().to_string()
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
    let attach = format!(r#"{{"mission_id":"{id}","start_time":"2030-01-01T00:00:00Z"}}"#);
    call(
        &app,
        "POST",
        &format!("/api/v1/events/{eid}/missions"),
        &t,
        Some(&attach),
    )
    .await;
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

#[tokio::test]
async fn mission_soft_delete_hides_everywhere() {
    let Some((app, _)) = boot().await else { return };
    let t = tok(&app, "admin").await;
    let id = mk_mission(&app, &t).await;
    let (st, _) = call(&app, "DELETE", &format!("/api/v1/missions/{id}"), &t, None).await;
    assert_eq!(st, StatusCode::NO_CONTENT);
    let (st, _) = call(&app, "GET", &format!("/api/v1/missions/{id}"), &t, None).await;
    assert_eq!(st, StatusCode::NOT_FOUND, "soft-deleted mission is gone");
}

#[tokio::test]
async fn editor_only_orbat_derivation() {
    let Some((app, _)) = boot().await else { return };
    let t = tok(&app, "admin").await;
    let id = mk_mission(&app, &t).await;
    // Save a version whose editor graph (no top-level orbat) has two ordered slots.
    let ver = r#"{"semver":"0.2.0","payload":{"editor":{"factions":[{"key":"USA","squadIds":["s1"]}],"squads":[{"id":"s1","name":"Alpha","slotIds":["x0","x1"]}],"slots":[{"id":"x0","index":0,"role":"SL"},{"id":"x1","index":1,"role":"RTO"}]}}}"#;
    let (st, _) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{id}/versions"),
        &t,
        Some(ver),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED);
    // Attach WITHOUT an explicit orbat → ORBAT is derived from the version payload.
    let (_, e) = call(
        &app,
        "POST",
        "/api/v1/events",
        &t,
        Some(r#"{"start_time":"2030-06-01T00:00:00Z"}"#),
    )
    .await;
    let eid = e["id"].as_str().unwrap();
    let attach = format!(r#"{{"mission_id":"{id}","start_time":"2030-06-01T00:00:00Z"}}"#);
    let (st, em) = call(
        &app,
        "POST",
        &format!("/api/v1/events/{eid}/missions"),
        &t,
        Some(&attach),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "attach: {em}");
    let emid = em["id"].as_str().unwrap();
    let (st, orbat) = call(
        &app,
        "GET",
        &format!("/api/v1/event-missions/{emid}/orbat"),
        &t,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(orbat["data"][0]["squad"], "Alpha");
    assert_eq!(orbat["data"][0]["slots"].as_array().unwrap().len(), 2);
    assert_eq!(orbat["data"][0]["slots"][0]["role"], "SL");
}

#[tokio::test]
async fn export_dangling_version_is_500() {
    let Some((app, pool)) = boot().await else {
        return;
    };
    let t = tok(&app, "admin").await;
    let id = mk_mission(&app, &t).await;
    // Point current_version_id at a version that does not exist.
    sqlx::query("UPDATE missions SET current_version_id = gen_random_uuid() WHERE id = $1")
        .bind(id.parse::<Uuid>().unwrap())
        .execute(&pool)
        .await
        .unwrap();
    let (st, _) = call(
        &app,
        "GET",
        &format!("/api/v1/missions/{id}/export"),
        &t,
        None,
    )
    .await;
    assert_eq!(
        st,
        StatusCode::INTERNAL_SERVER_ERROR,
        "dangling version must 500, never a silent empty export"
    );
}

#[tokio::test]
async fn export_visibility_non_author_404() {
    let Some((app, pool)) = boot().await else {
        return;
    };
    // A DRAFT authored by someone else.
    let mid = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO missions (id, title, author_id, terrain, custom_terrain_name, game_mode, weather, time_of_day, max_players, status, thumbnail_url, briefing, rejection_reason, created_at, updated_at) \
         VALUES ($1, 'Secret', $2, 'everon', '', 'pve_coop', 'clear', '14:00', 16, 'draft', '', '', '', now(), now())",
    )
    .bind(mid)
    .bind(OTHER)
    .execute(&pool)
    .await
    .unwrap();
    // export is mission_maker-tier; a mission_maker who is NOT the author (nor admin)
    // passes the tier but must not see another author's draft → 404 (not 403, no id leak).
    let mm = tok(&app, "mission_maker").await;
    let (st, _) = call(
        &app,
        "GET",
        &format!("/api/v1/missions/{mid}/export"),
        &mm,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn version_route_bypasses_the_1mb_global_cap() {
    let Some((app, _)) = boot().await else { return };
    let t = tok(&app, "admin").await;
    let id = mk_mission(&app, &t).await;
    // ~1.5 MB payload: over the 1 MB global cap, under the 256 MB version cap.
    let big = "A".repeat(1_500_000);
    let ver = format!(
        r#"{{"semver":"0.9.0","payload":{{"editor":{{"squads":[{{"id":"s1","name":"{big}"}}]}}}}}}"#
    );
    let (st, _) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{id}/versions"),
        &t,
        Some(&ver),
    )
    .await;
    assert_ne!(
        st,
        StatusCode::PAYLOAD_TOO_LARGE,
        "version route must accept a >1 MB body (not 413)"
    );
    // A globally-capped route truncates the same body at 1 MB → invalid JSON → 400
    // (mirrors Go: only CreateVersion special-cases the length limit into a 413).
    let patch = format!(r#"{{"briefing":"{big}"}}"#);
    let (st, _) = call(
        &app,
        "PATCH",
        &format!("/api/v1/missions/{id}"),
        &t,
        Some(&patch),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::BAD_REQUEST,
        "over-cap body on a normal route → 400 invalid body"
    );
}

#[tokio::test]
async fn purge_removes_only_long_expired_tokens() {
    let Some((_, pool)) = boot().await else {
        return;
    };
    let fresh = format!("hash-fresh-{}", Uuid::new_v4());
    let stale = format!("hash-stale-{}", Uuid::new_v4());
    // Fresh (future expiry) + stale (expired > 7 days ago).
    for (h, days) in [(&fresh, 1i64), (&stale, -8i64)] {
        sqlx::query(
            "INSERT INTO refresh_tokens (discord_id, token_hash, expires_at, created_at) VALUES ('000000000000000007', $1, now() + ($2 || ' days')::interval, now())",
        )
        .bind(h)
        .bind(days.to_string())
        .execute(&pool)
        .await
        .unwrap();
    }
    let removed = purge_expired_refresh_tokens(&pool).await.unwrap();
    assert!(removed >= 1, "at least the stale token purged");
    let fresh_left: i64 =
        sqlx::query_scalar("SELECT count(*) FROM refresh_tokens WHERE token_hash = $1")
            .bind(&fresh)
            .fetch_one(&pool)
            .await
            .unwrap();
    let stale_left: i64 =
        sqlx::query_scalar("SELECT count(*) FROM refresh_tokens WHERE token_hash = $1")
            .bind(&stale)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(fresh_left, 1, "fresh token kept");
    assert_eq!(stale_left, 0, "stale token purged");
}
