//! T-528 — GET /me and /me/link/status treat whitespace `arma_id` as unlinked.
//!
//! # Owns expansion (called out)
//!
//! Wave owns list is only `handlers/me.rs`. This IT binary is the Class-R / IT half
//! the ticket requires: plant a whitespace-only `users.arma_id` and assert both
//! endpoints report unlinked. Proves me.rs is not `is_some()`-only (T-350 finish).
//!
//! Helper: [`website_api::handlers::auth::arma_id_is_linked`] — same as refresh/oauth.

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use website_api::config::Config;
use website_api::state::AppState;
use website_api::{app, db};

mod common;

/// Serialise DB-touching tests — share ACTOR / WS_ARMA on one gate DB.
static DB_LOCK: std::sync::LazyLock<tokio::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

/// Private actor — must not share `DEV_LOGIN_USER` or T-350's ACTOR.
const ACTOR: &str = "000000000000528001";
/// Stored whitespace-only `arma_id` (single space — ticket pin).
const WS_ARMA: &str = " ";
/// Unique non-whitespace seed released before we overwrite with WS_ARMA.
const SEED_ARMA: &str = "t528-seed-arma-528001";

async fn boot() -> Option<(Router, AppState, PgPool)> {
    let url = common::require_test_database_url()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let cfg = Config::for_tests(url, "t528-secret");
    let state = AppState::new(pool.clone(), cfg);
    Some((app::router(state.clone()), state, pool))
}

async fn cleanup(pool: &PgPool) {
    sqlx::query("DELETE FROM identity_link_codes WHERE discord_id = $1")
        .bind(ACTOR)
        .execute(pool)
        .await
        .expect("t528 cleanup link codes");
    sqlx::query("DELETE FROM refresh_tokens WHERE discord_id = $1")
        .bind(ACTOR)
        .execute(pool)
        .await
        .expect("t528 cleanup refresh");
    sqlx::query("UPDATE users SET arma_id = NULL WHERE arma_id = ANY($1)")
        .bind(vec![WS_ARMA.to_string(), SEED_ARMA.to_string()])
        .execute(pool)
        .await
        .expect("t528 release arma");
}

async fn plant_whitespace(pool: &PgPool) {
    common::seed_user(pool, ACTOR, "t528-ws", SEED_ARMA, "enlisted").await;
    sqlx::query("UPDATE users SET arma_id = $1, updated_at = now() WHERE discord_id = $2")
        .bind(WS_ARMA)
        .bind(ACTOR)
        .execute(pool)
        .await
        .expect("plant whitespace arma_id");

    let stored: Option<String> =
        sqlx::query_scalar("SELECT arma_id FROM users WHERE discord_id = $1")
            .bind(ACTOR)
            .fetch_one(pool)
            .await
            .expect("read arma_id");
    assert_eq!(stored.as_deref(), Some(WS_ARMA));
    assert!(stored.is_some());
    assert!(stored.as_deref().unwrap().trim().is_empty());
}

/// Class-R: GET /me must report `arma_linked: false` for whitespace-only arma_id.
///
/// Perturbation: revert `get_me` to `u.arma_id.is_some()` → this asserts false.
#[tokio::test]
async fn get_me_whitespace_arma_id_reports_arma_linked_false() {
    let _guard = DB_LOCK.lock().await;
    let Some((app, state, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    cleanup(&pool).await;
    plant_whitespace(&pool).await;

    // JWT claim may still say linked=true from mint; /me recomputes from the DB row.
    let tok = common::access_token(&state, "t528_me_whitespace_linked", ACTOR, "enlisted", true);
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/me")
                .header(header::AUTHORIZATION, format!("Bearer {tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "GET /me must succeed");
    let json: Value =
        serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(
        json["arma_linked"], false,
        "whitespace arma_id must yield arma_linked=false on GET /me; got {json}"
    );

    cleanup(&pool).await;
}

/// Class-R: GET /me/link/status must report `linked: false` for whitespace-only arma_id.
///
/// Perturbation: revert `link_status` to `u.arma_id.is_some()` → this asserts false.
#[tokio::test]
async fn link_status_whitespace_arma_id_reports_linked_false() {
    let _guard = DB_LOCK.lock().await;
    let Some((app, state, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    cleanup(&pool).await;
    plant_whitespace(&pool).await;

    let tok = common::access_token(&state, "t528_me_whitespace_linked", ACTOR, "enlisted", true);
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/v1/me/link/status")
                .header(header::AUTHORIZATION, format!("Bearer {tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "GET /me/link/status must succeed"
    );
    let json: Value =
        serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(
        json["linked"], false,
        "whitespace arma_id must yield linked=false on /me/link/status; got {json}"
    );

    cleanup(&pool).await;
}

/// Real content still links — guards against "always false" vacuity on both sites.
#[tokio::test]
async fn get_me_and_link_status_real_arma_id_reports_linked_true() {
    let _guard = DB_LOCK.lock().await;
    let Some((app, state, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    cleanup(&pool).await;
    common::seed_user(&pool, ACTOR, "t528-real", SEED_ARMA, "enlisted").await;

    let tok = common::access_token(
        &state,
        "t528_me_whitespace_linked",
        ACTOR,
        "enlisted",
        false,
    );
    for (uri, key) in [
        ("/api/v1/me", "arma_linked"),
        ("/api/v1/me/link/status", "linked"),
    ] {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(uri)
                    .header(header::AUTHORIZATION, format!("Bearer {tok}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK, "{uri}");
        let json: Value =
            serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
        assert_eq!(
            json[key], true,
            "{uri}: non-whitespace arma_id must still report linked; got {json}"
        );
    }

    cleanup(&pool).await;
}
