//! OAuth redirect paths — Rust port of `oauth_redirect_test.go`. These bail before
//! any DB access, so they run with a lazy (unconnected) pool and need no live DB.

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::response::Response;
use tower::ServiceExt;
use website_api::config::Config;
use website_api::state::AppState;
use website_api::{app, db};

fn app() -> Router {
    // for_tests() has a blank Discord client_id → the "oauth_unconfigured" path.
    let pool = db::connect_lazy("postgres://tbd:tbd@localhost:5434/unused").unwrap();
    app::router(AppState::new(
        pool,
        Config::for_tests("postgres://x/x", "oauth-secret"),
    ))
}

fn location(resp: &Response) -> String {
    resp.headers()[header::LOCATION]
        .to_str()
        .unwrap()
        .to_string()
}

#[tokio::test]
async fn discord_login_unconfigured_redirects_to_spa_error() {
    let resp = app()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/discord/login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FOUND);
    let loc = location(&resp);
    assert!(loc.contains("/auth/callback#"), "SPA callback: {loc}");
    assert!(loc.contains("error=oauth_unconfigured"), "{loc}");
}

#[tokio::test]
async fn callback_missing_code_redirects_error() {
    let resp = app()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/discord/callback")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FOUND);
    assert!(location(&resp).contains("error=missing_code"));
}

#[tokio::test]
async fn callback_invalid_state_redirects_error() {
    // code + state present, but no matching oauth_state cookie → invalid_state.
    let resp = app()
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/discord/callback?code=abc&state=xyz")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FOUND);
    assert!(location(&resp).contains("error=invalid_state"));
}

#[tokio::test]
async fn discord_login_sets_oauth_state_cookie_when_configured() {
    // A configured client_id → 307 to Discord + a state cookie.
    let mut cfg = Config::for_tests("postgres://x/x", "oauth-secret");
    cfg.discord_client_id = "test-client".into();
    cfg.discord_redirect_url = "http://localhost:8080/api/v1/auth/discord/callback".into();
    let pool = db::connect_lazy("postgres://tbd:tbd@localhost:5434/unused").unwrap();
    let resp = app::router(AppState::new(pool, cfg))
        .oneshot(
            Request::builder()
                .uri("/api/v1/auth/discord/login")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::TEMPORARY_REDIRECT); // 307
    let loc = location(&resp);
    assert!(
        loc.contains("/oauth2/authorize?"),
        "to Discord consent: {loc}"
    );
    assert!(loc.contains("client_id=test-client"));
    let cookie = resp.headers()[header::SET_COOKIE].to_str().unwrap();
    assert!(cookie.starts_with("oauth_state="));
    assert!(cookie.contains("HttpOnly"));
    assert!(cookie.contains("SameSite=Lax"));
}
