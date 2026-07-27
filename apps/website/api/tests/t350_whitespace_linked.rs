//! T-350 — whitespace `arma_id` is not linked; leave `reason` rejects blank/whitespace.
//!
//! # Owns expansion (called out)
//!
//! Wave owns list only the three handlers. This IT binary is the Class-R / IT half the
//! ticket requires: refresh must mint `arma_linked=false` when the row holds
//! whitespace-only `arma_id` (proves auth.rs is not `is_some()`-only), and
//! `POST /me/leave-requests` must 400 on whitespace `reason` (deployments.rs).
//!
//! oauth.rs shares [`website_api::handlers::auth::arma_id_is_linked`] — covered by the
//! unit pin in `handlers/auth.rs` plus the refresh IT below (same helper, same claim).

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;
use website_api::config::Config;
use website_api::handlers::auth::issue_refresh;
use website_api::state::AppState;
use website_api::{app, db};

mod common;

/// Serialise DB-touching tests — all three share ACTOR / WS_ARMA on one gate DB.
static DB_LOCK: std::sync::LazyLock<tokio::sync::Mutex<()>> =
    std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));

/// Private actor — must not share `DEV_LOGIN_USER` (auth_refresh / identity_link).
const ACTOR: &str = "000000000000350001";
/// Stored whitespace-only `arma_id`. Distinct from empty-string fixtures other suites use.
const WS_ARMA: &str = "   ";
/// Unique non-whitespace seed released before we overwrite with WS_ARMA.
const SEED_ARMA: &str = "t350-seed-arma-350001";

async fn boot() -> Option<(Router, AppState, PgPool)> {
    let url = common::require_test_database_url()?;
    let pool = db::connect(&url).await.expect("connect");
    db::migrate(&pool).await.expect("migrate");
    let cfg = Config::for_tests(url, "t350-secret");
    let state = AppState::new(pool.clone(), cfg);
    Some((app::router(state.clone()), state, pool))
}

async fn cleanup(pool: &PgPool) {
    sqlx::query("DELETE FROM refresh_tokens WHERE discord_id = $1")
        .bind(ACTOR)
        .execute(pool)
        .await
        .expect("t350 cleanup refresh");
    sqlx::query("DELETE FROM leave_requests WHERE discord_id = $1")
        .bind(ACTOR)
        .execute(pool)
        .await
        .expect("t350 cleanup leave");
    // Release UNIQUE arma values before reseed.
    sqlx::query("UPDATE users SET arma_id = NULL WHERE arma_id = ANY($1)")
        .bind(vec![WS_ARMA.to_string(), SEED_ARMA.to_string()])
        .execute(pool)
        .await
        .expect("t350 release arma");
}

/// Refresh must mint `arma_linked=false` for a legacy whitespace `users.arma_id` row.
///
/// Class-R: if `refresh` still used `user.arma_id.is_some()`, this asserts false.
#[tokio::test]
async fn refresh_whitespace_arma_id_mints_arma_linked_false() {
    let _guard = DB_LOCK.lock().await;
    let Some((app, state, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    cleanup(&pool).await;
    common::seed_user(&pool, ACTOR, "t350-ws", SEED_ARMA, "enlisted").await;
    sqlx::query("UPDATE users SET arma_id = $1, updated_at = now() WHERE discord_id = $2")
        .bind(WS_ARMA)
        .bind(ACTOR)
        .execute(&pool)
        .await
        .expect("plant whitespace arma_id");

    // Prove the row really is Some(whitespace) — the is_some()-only bug's precondition.
    let stored: Option<String> =
        sqlx::query_scalar("SELECT arma_id FROM users WHERE discord_id = $1")
            .bind(ACTOR)
            .fetch_one(&pool)
            .await
            .expect("read arma_id");
    assert_eq!(stored.as_deref(), Some(WS_ARMA));
    assert!(stored.is_some());
    assert!(stored.as_deref().unwrap().trim().is_empty());

    let refresh = issue_refresh(&pool, ACTOR).await.expect("issue_refresh");
    let body = format!(r#"{{"refresh_token":"{refresh}"}}"#);
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK, "refresh must succeed");
    let json: Value =
        serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    let access = json["access_token"].as_str().expect("access_token");
    let claims = state.jwt.parse(access).expect("parse access JWT");
    assert!(
        !claims.arma_linked,
        "whitespace arma_id must mint arma_linked=false; got true (is_some()-only regression)"
    );

    cleanup(&pool).await;
}

/// Real content still links — guards against "always false" vacuity.
#[tokio::test]
async fn refresh_real_arma_id_mints_arma_linked_true() {
    let _guard = DB_LOCK.lock().await;
    let Some((app, state, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    cleanup(&pool).await;
    common::seed_user(&pool, ACTOR, "t350-real", SEED_ARMA, "enlisted").await;

    let refresh = issue_refresh(&pool, ACTOR).await.expect("issue_refresh");
    let body = format!(r#"{{"refresh_token":"{refresh}"}}"#);
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/auth/refresh")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let json: Value =
        serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    let claims = state
        .jwt
        .parse(json["access_token"].as_str().unwrap())
        .unwrap();
    assert!(
        claims.arma_linked,
        "non-whitespace arma_id must still mint arma_linked=true"
    );

    cleanup(&pool).await;
}

/// Leave reason blank/whitespace → 400 (T-218/317/343 family; T-350 closes the LOA site).
#[tokio::test]
async fn submit_leave_rejects_blank_and_whitespace_reason() {
    let _guard = DB_LOCK.lock().await;
    let Some((app, state, pool)) = boot().await else {
        eprintln!("skip: TEST_DATABASE_URL unset");
        return;
    };
    cleanup(&pool).await;
    common::seed_user(&pool, ACTOR, "t350-leave", SEED_ARMA, "enlisted").await;
    let tok = common::access_token(&state, "t350_whitespace_linked", ACTOR, "enlisted", false);

    let rejected = [
        (
            "missing reason key",
            r#"{"starts_on":"2026-08-01","ends_on":"2026-08-05"}"#,
        ),
        (
            "empty reason",
            r#"{"starts_on":"2026-08-01","ends_on":"2026-08-05","reason":""}"#,
        ),
        (
            "whitespace-only reason",
            r#"{"starts_on":"2026-08-01","ends_on":"2026-08-05","reason":"   "}"#,
        ),
    ];
    for (label, body) in rejected {
        let resp = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/me/leave-requests")
                    .header(header::AUTHORIZATION, format!("Bearer {tok}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::BAD_REQUEST,
            "{label}: expected 400"
        );
        let json: Value =
            serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
        // Missing key fails decode → "starts_on, ends_on and reason are required";
        // empty/whitespace hits the trim guard → "reason is required".
        let err = json["error"].as_str().unwrap_or("");
        assert!(
            err.contains("reason"),
            "{label}: error must mention reason, got {json}"
        );
    }

    // Positive: trimmed reason stores cleanly.
    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/me/leave-requests")
                .header(header::AUTHORIZATION, format!("Bearer {tok}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"starts_on":"2026-08-01","ends_on":"2026-08-05","reason":"  holiday  "}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let json: Value =
        serde_json::from_slice(&to_bytes(resp.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(json["reason"], "holiday");

    cleanup(&pool).await;
}
