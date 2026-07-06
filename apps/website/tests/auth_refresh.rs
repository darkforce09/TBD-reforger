//! Gate G7a — the single-use rotating refresh token invariant (port of
//! `auth_refresh_integration_test.go`). Drives the real router via `tower::oneshot`.
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

async fn setup() -> Option<(Router, PgPool)> {
    let url = std::env::var("TEST_DATABASE_URL").ok()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    // Isolate: clear the dev user's tokens from prior runs.
    sqlx::query("DELETE FROM refresh_tokens WHERE discord_id = $1")
        .bind(DEV_ID)
        .execute(&pool)
        .await
        .expect("cleanup");
    let cfg = Config::for_tests(url, "g7a-secret");
    Some((app::router(AppState::new(pool.clone(), cfg)), pool))
}

/// Extract a fragment param from a redirect Location. Token values are hex/JWT, so
/// no percent-decoding is needed for `refresh_token`.
fn fragment_value(location: &str, key: &str) -> String {
    let frag = location.split_once('#').map(|(_, f)| f).unwrap_or("");
    for pair in frag.split('&') {
        if let Some((k, v)) = pair.split_once('=')
            && k == key
        {
            return v.to_string();
        }
    }
    String::new()
}

async fn post_refresh(app: &Router, token: &str) -> (StatusCode, Value) {
    let body = format!(r#"{{"refresh_token":"{token}"}}"#);
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/refresh")
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let json = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, json)
}

#[tokio::test]
async fn refresh_rotation_reuse_revokes_family() {
    let Some((app, _pool)) = setup().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };

    // dev-login → 302 with the token fragment.
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
    assert_eq!(resp.status(), StatusCode::FOUND, "dev-login redirects");
    let loc = resp.headers()[header::LOCATION].to_str().unwrap();
    let refresh1 = fragment_value(loc, "refresh_token");
    assert!(!refresh1.is_empty(), "fragment carries refresh_token");

    // The access token authorizes GET /me (exercises the AuthUser extractor E2E).
    let access1 = fragment_value(loc, "access_token");
    let me = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/me")
                .header(header::AUTHORIZATION, format!("Bearer {access1}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(me.status(), StatusCode::OK, "GET /me authorized");
    let me_body: Value =
        serde_json::from_slice(&to_bytes(me.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(me_body["user"]["discord_id"], DEV_ID);
    assert_eq!(me_body["arma_linked"], true);

    // No bearer → 401 (extractor rejection).
    let noauth = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/me")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        noauth.status(),
        StatusCode::UNAUTHORIZED,
        "GET /me needs auth"
    );

    // Rotate: token1 → 200 + a fresh token2.
    let (status, body) = post_refresh(&app, &refresh1).await;
    assert_eq!(status, StatusCode::OK, "first rotation succeeds");
    assert_eq!(body["token_type"], "Bearer");
    let refresh2 = body["refresh_token"].as_str().unwrap().to_string();
    assert!(
        !refresh2.is_empty() && refresh2 != refresh1,
        "token rotated"
    );

    // Reuse of the now-revoked token1 is detected and revokes the whole family.
    let (status, body) = post_refresh(&app, &refresh1).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "refresh token reuse detected");

    // token2 was revoked by the family sweep → also rejected.
    let (status, _) = post_refresh(&app, &refresh2).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED, "family revoked token2");

    // An unknown token → invalid (not reuse).
    let (status, body) = post_refresh(&app, "deadbeef").await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(body["error"], "invalid refresh token");
}
