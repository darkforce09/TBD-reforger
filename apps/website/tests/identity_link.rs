//! Arma identity-link flow — port of the link half of `identity_integration_test.go`.
//! Skips unless `TEST_DATABASE_URL` points at a migrated DB.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use reforger_backend::config::Config;
use reforger_backend::state::AppState;
use reforger_backend::{app, db};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

const DEV_ID: &str = "000000000000000001";
const USER2: &str = "identity-link-user2";
const SVC: &str = "test-service-token";

async fn setup() -> Option<(Router, PgPool)> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    // Clean prior state for both users.
    for q in [
        "DELETE FROM identity_link_codes WHERE discord_id = ANY($1)",
        "DELETE FROM users WHERE discord_id = $2",
        "UPDATE users SET arma_id = NULL WHERE discord_id = $3",
    ] {
        let _ = sqlx::query(q)
            .bind(vec![DEV_ID.to_string(), USER2.to_string()])
            .bind(USER2)
            .bind(DEV_ID)
            .execute(&pool)
            .await;
    }
    let cfg = Config::for_tests(url, "identity-secret");
    Some((app::router(AppState::new(pool.clone(), cfg)), pool))
}

fn frag(location: &str, key: &str) -> String {
    location
        .split_once('#')
        .map(|(_, f)| f)
        .unwrap_or("")
        .split('&')
        .find_map(|p| {
            p.split_once('=')
                .filter(|(k, _)| *k == key)
                .map(|(_, v)| v.to_string())
        })
        .unwrap_or_default()
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    headers: &[(&str, &str)],
    body: Option<&str>,
) -> (StatusCode, Value) {
    let mut b = Request::builder().method(method).uri(uri);
    for (k, v) in headers {
        b = b.header(*k, *v);
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
async fn arma_link_flow() {
    let Some((app, pool)) = setup().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    // dev-login → access token.
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
    let access = frag(
        resp.headers()[header::LOCATION].to_str().unwrap(),
        "access_token",
    );
    let bearer = format!("Bearer {access}");
    let auth = [(header::AUTHORIZATION.as_str(), bearer.as_str())];
    let json_svc = [
        (header::CONTENT_TYPE.as_str(), "application/json"),
        ("x-service-token", SVC),
    ];

    // Start unlinked.
    let (st, _) = call(&app, "DELETE", "/api/v1/me/link", &auth, None).await;
    assert_eq!(st, StatusCode::OK);
    let (st, body) = call(&app, "GET", "/api/v1/me/link/status", &auth, None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(body["linked"], false);
    assert_eq!(body["pending_code"], false);

    // Create a code → 201, pending.
    let (st, body) = call(&app, "POST", "/api/v1/me/link", &auth, None).await;
    assert_eq!(st, StatusCode::CREATED);
    let code = body["code"].as_str().unwrap().to_string();
    assert_eq!(code.len(), 6);
    let (_, body) = call(&app, "GET", "/api/v1/me/link/status", &auth, None).await;
    assert_eq!(body["pending_code"], true);

    // Confirm (service-token) → linked.
    let confirm =
        format!(r#"{{"code":"{code}","arma_id":"steam-xyz","arma_character":"Test Char"}}"#);
    let (st, body) = call(
        &app,
        "POST",
        "/api/v1/ingest/link-confirm",
        &json_svc,
        Some(&confirm),
    )
    .await;
    assert_eq!(st, StatusCode::OK, "confirm body={body}");
    assert_eq!(body["linked"], true);
    assert_eq!(body["arma_id"], "steam-xyz");

    let (_, body) = call(&app, "GET", "/api/v1/me/link/status", &auth, None).await;
    assert_eq!(body["linked"], true);
    assert_eq!(body["arma_id"], "steam-xyz");
    assert_eq!(body["arma_character"], "Test Char");

    // Re-confirm the consumed code → 404.
    let (st, _) = call(
        &app,
        "POST",
        "/api/v1/ingest/link-confirm",
        &json_svc,
        Some(&confirm),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);

    // No/invalid service token → 401.
    let (st, _) = call(
        &app,
        "POST",
        "/api/v1/ingest/link-confirm",
        &[(header::CONTENT_TYPE.as_str(), "application/json")],
        Some(&confirm),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);

    // Clash: a second user's code confirming with dev's arma_id → 409.
    sqlx::query(
        "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_character, \
         role, is_banned, ban_reason, created_at, updated_at) \
         VALUES ($1, 'User Two', '', '', '', 'enlisted', false, '', now(), now()) \
         ON CONFLICT (discord_id) DO NOTHING",
    )
    .bind(USER2)
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO identity_link_codes (code, discord_id, expires_at, created_at) \
         VALUES ('424242', $1, now() + interval '10 minutes', now())",
    )
    .bind(USER2)
    .execute(&pool)
    .await
    .unwrap();
    let clash = r#"{"code":"424242","arma_id":"steam-xyz","arma_character":"Dupe"}"#;
    let (st, body) = call(
        &app,
        "POST",
        "/api/v1/ingest/link-confirm",
        &json_svc,
        Some(clash),
    )
    .await;
    assert_eq!(st, StatusCode::CONFLICT);
    assert_eq!(body["error"], "arma id already linked to another account");
}
