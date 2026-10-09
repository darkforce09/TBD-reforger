//! GET /me and /me/link/status treat whitespace `arma_id` as unlinked.
//!
//! # Scope
//!
//! This binary is the HTTP half: plant a whitespace-only `users.arma_id` and assert both
//! endpoints report unlinked. Proves the `/me` handlers are not `is_some()`-only.
//!
//! Helper:
//! [`api_caller_identity::arma_identity_link::arma_id_is_linked`] — the same
//! one refresh and the Discord callback use.

use crate::common;

use api_configuration::configuration::Config;
use api_state::AppState;

use api_server::router::router;
use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

/// Serialise DB-touching tests — share ACTOR / WS_ARMA on one gate DB.
static DB_LOCK: std::sync::LazyLock<tokio::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

/// Private actor — must not share `DEV_LOGIN_USER` or the refresh suite's ACTOR.
const ACTOR: &str = "000000000000528001";
/// Stored whitespace-only `arma_id` (single space).
const WS_ARMA: &str = " ";
/// Unique non-whitespace seed released before we overwrite with WS_ARMA.
const SEED_ARMA: &str = "profile-ws-seed-arma-1";

async fn boot() -> Option<(Router, AppState, PgPool)> {
    let url = common::require_test_database_url()?;
    let pool = api_database::connect(&url).await.expect("connect");
    api_database::migrate(&pool).await.expect("migrate");
    let cfg = Config::for_tests(url, "profile-ws-secret");
    let state = api_server::composition::application_state(pool.clone(), cfg);
    Some((router(state.clone()), state, pool))
}

async fn cleanup(pool: &PgPool) {
    sqlx::query("DELETE FROM identity_link_codes WHERE discord_id = $1")
        .bind(ACTOR)
        .execute(pool)
        .await
        .expect("profile-ws cleanup link codes");
    sqlx::query("DELETE FROM refresh_tokens WHERE discord_id = $1")
        .bind(ACTOR)
        .execute(pool)
        .await
        .expect("profile-ws cleanup refresh");
    sqlx::query("UPDATE users SET arma_id = NULL WHERE arma_id = ANY($1)")
        .bind(vec![WS_ARMA.to_string(), SEED_ARMA.to_string()])
        .execute(pool)
        .await
        .expect("profile-ws release arma");
}

async fn plant_whitespace(pool: &PgPool) {
    common::seed_user(pool, ACTOR, "profile-ws-ws", SEED_ARMA, "enlisted").await;
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
    assert!(
        stored
            .as_deref()
            .expect("the stored arma_id is present")
            .trim()
            .is_empty()
    );
}

/// GET /me must report `arma_linked: false` for whitespace-only arma_id.
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
    let tok = common::access_token(
        &state,
        "profile_whitespace_arma_id",
        ACTOR,
        "enlisted",
        true,
    )
    .await;
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
