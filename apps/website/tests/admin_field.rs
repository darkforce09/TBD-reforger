//! Admin + approvals + CMS + field-tools. Skips without `TEST_DATABASE_URL`.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use reforger_backend::config::Config;
use reforger_backend::state::AppState;
use reforger_backend::{app, db};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

const TARGET: &str = "000000000000000009";

async fn boot() -> Option<(Router, PgPool)> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let app = app::router(AppState::new(
        pool.clone(),
        Config::for_tests(url, "af-secret"),
    ));
    Some((app, pool))
}

async fn admin_token(app: &Router) -> String {
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/dev-login?role=admin")
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
    tok: &str,
    body: Option<&str>,
) -> (StatusCode, Value) {
    let mut b = Request::builder()
        .method(method)
        .uri(uri)
        .header(header::AUTHORIZATION, format!("Bearer {tok}"));
    if body.is_some() {
        b = b.header(header::CONTENT_TYPE, "application/json");
    }
    let req = b
        .body(body.map_or(Body::empty(), |s| Body::from(s.to_string())))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn admin_approvals_cms_field() {
    let Some((app, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    let t = admin_token(&app).await;

    // A ban/warn target + a server for RCON.
    // arma_id NULL (not '') — a UNIQUE index forbids duplicate non-null arma_ids;
    // Go stores unlinked users as NULL (`*string`), so NULLs coexist.
    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, arma_character, role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'Target Z', 'targetz', '', NULL, '', 'enlisted', false, '', now(), now()) \
         ON CONFLICT (discord_id) DO UPDATE SET is_banned = false, role = 'enlisted'",
    )
    .bind(TARGET)
    .execute(&pool)
    .await
    .unwrap();
    let server_id: uuid::Uuid = sqlx::query_scalar(
        "INSERT INTO servers (name, ip, port, is_active) VALUES ('AF Srv', '127.0.0.1'::inet, 2010, true) RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    // --- admin ---
    let (st, w) = call(
        &app,
        "POST",
        &format!("/api/v1/admin/users/{TARGET}/warnings"),
        &t,
        Some(r#"{"reason":"late"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "warn: {w}");
    let (st, roster) = call(&app, "GET", "/api/v1/admin/users?q=Target%20Z", &t, None).await;
    assert_eq!(st, StatusCode::OK);
    let row = roster["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["discord_id"] == TARGET)
        .unwrap();
    assert!(row["warnings"].as_i64().unwrap() >= 1);
    assert_eq!(row["role"], "enlisted");

    let (st, r) = call(
        &app,
        "PATCH",
        &format!("/api/v1/admin/users/{TARGET}"),
        &t,
        Some(r#"{"role":"leader"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(r["role"], "leader");
    let (st, r) = call(
        &app,
        "PATCH",
        &format!("/api/v1/admin/users/{TARGET}"),
        &t,
        Some(r#"{"role":"wizard"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST, "invalid role: {r}");

    let (st, r) = call(
        &app,
        "POST",
        &format!("/api/v1/admin/users/{TARGET}/ban"),
        &t,
        Some(r#"{"reason":"grief"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(r["banned"], true);
    let (st, r) = call(
        &app,
        "DELETE",
        &format!("/api/v1/admin/users/{TARGET}/ban"),
        &t,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(r["banned"], false);

    let (st, _) = call(&app, "POST", "/api/v1/admin/roles/sync", &t, None).await;
    assert_eq!(st, StatusCode::OK);
    let (st, r) = call(
        &app,
        "POST",
        &format!("/api/v1/admin/servers/{server_id}/rcon"),
        &t,
        Some(r#"{"action":"restart"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::ACCEPTED);
    assert_eq!(r["action"], "restart");
    let (st, _) = call(
        &app,
        "POST",
        &format!("/api/v1/admin/servers/{server_id}/rcon"),
        &t,
        Some(r#"{"action":"nuke"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    // --- approvals + inject ---
    let (_, m) = call(
        &app,
        "POST",
        "/api/v1/missions",
        &t,
        Some(
            r#"{"title":"Approve Me","terrain":"everon","game_mode":"pve_coop","max_players":16}"#,
        ),
    )
    .await;
    let mid = m["id"].as_str().unwrap().to_string();
    let (st, _) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{mid}/submit"),
        &t,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let (st, appr) = call(&app, "GET", "/api/v1/approvals", &t, None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(
        appr["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["mission_id"] == mid.as_str())
    );
    let (st, r) = call(
        &app,
        "POST",
        &format!("/api/v1/approvals/{mid}/approve"),
        &t,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK, "approve: {r}");
    assert_eq!(r["status"], "live");
    // Now live → injectable.
    let (st, inj) = call(
        &app,
        "POST",
        &format!("/api/v1/missions/{mid}/inject"),
        &t,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::ACCEPTED, "inject: {inj}");
    assert!(
        inj["staged_path"]
            .as_str()
            .unwrap()
            .ends_with(".mission.json")
    );

    // --- CMS ---
    let (st, a) = call(
        &app,
        "POST",
        "/api/v1/cms/announcements",
        &t,
        Some(r#"{"title":"News","body":"<b>hi</b><script>x</script>","status":"published"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED, "announce: {a}");
    let aid = a["id"].as_str().unwrap().to_string();
    assert!(
        !a["body"].as_str().unwrap().contains("<script>"),
        "body sanitized"
    );
    // Visible on the public feed while published.
    let (st, _) = call(
        &app,
        "GET",
        &format!("/api/v1/announcements/{aid}"),
        &t,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    // Webhook not configured in tests → push-discord 400.
    let (st, _) = call(
        &app,
        "POST",
        &format!("/api/v1/cms/announcements/{aid}/push-discord"),
        &t,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    // Archive → gone from the public feed.
    let (st, _) = call(
        &app,
        "DELETE",
        &format!("/api/v1/cms/announcements/{aid}"),
        &t,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NO_CONTENT);
    let (st, _) = call(
        &app,
        "GET",
        &format!("/api/v1/announcements/{aid}"),
        &t,
        None,
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // --- field tools (mortar) ---
    let (st, sol) = call(
        &app,
        "POST",
        "/api/v1/fire-missions/solve",
        &t,
        Some(r#"{"weapon_system":"M252 81mm","fp_x":0,"fp_y":0,"tgt_x":0,"tgt_y":1000}"#),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "solve: {sol}");
    assert_eq!(sol["distance_m"], 1000);
    let (st, _) = call(
        &app,
        "POST",
        "/api/v1/fire-missions/solve",
        &t,
        Some(r#"{"weapon_system":"M252 81mm","fp_x":0,"fp_y":0,"tgt_x":0,"tgt_y":100000}"#),
    )
    .await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY, "out of range → 422");
    let (st, saved) = call(&app, "POST", "/api/v1/fire-missions", &t, Some(r#"{"weapon_system":"M252 81mm","fp_x":0,"fp_y":0,"tgt_x":0,"tgt_y":1000,"fp_grid":"012345","target_grid":"012845"}"#)).await;
    assert_eq!(st, StatusCode::CREATED, "save fire: {saved}");
    assert_eq!(saved["fire_mission"]["distance_m"], 1000);
}
