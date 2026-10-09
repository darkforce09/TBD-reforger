//! Persisted session authorization, concurrent rotation/logout, and database failure recovery.

use crate::common;

use api_configuration::configuration::Config;
use api_identifiers::DiscordUserId;
use api_identity_and_access::services::{
    session_issuance::issue_session, session_rotation::rotate_session,
};
use api_server::router::router;
use api_state::AppState;
use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::Barrier;
use tower::ServiceExt;
use uuid::Uuid;

async fn fixture() -> (AppState, DiscordUserId, String, String) {
    let url = common::require_test_database_url().expect("database required");
    let pool = api_database::connect(&url)
        .await
        .expect("the test database accepts a connection");
    api_database::migrate(&pool)
        .await
        .expect("the migrations apply to the test database");
    let state = api_server::composition::application_state(
        pool,
        Config::for_tests(url, "session-transactions"),
    );
    let actor = DiscordUserId::new(format!("session-{}", Uuid::new_v4()));
    common::seed_user(
        &state.pool,
        actor.as_str(),
        "Session Actor",
        &common::unique_arma("session"),
        "admin",
    )
    .await;
    common::fixtures::seed_membership(
        &state.pool,
        actor.as_str(),
        state.cfg.discord_guild_id.as_str(),
        "admin",
    )
    .await;
    let (access, _, refresh) = issue_session(&state, &actor)
        .await
        .expect("issuing a session succeeds");
    (state, actor, access, refresh)
}

async fn me(app: Router, access: &str) -> StatusCode {
    app.oneshot(
        Request::builder()
            .uri("/api/v1/me")
            .header("Authorization", format!("Bearer {access}"))
            .body(Body::empty())
            .expect("the request builds"),
    )
    .await
    .unwrap()
    .status()
}

async fn active_tokens(pool: &PgPool, actor: &DiscordUserId) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM refresh_tokens WHERE discord_id = $1 AND revoked_at IS NULL",
    )
    .bind(actor)
    .fetch_one(pool)
    .await
    .expect("the read of refresh_tokens returns a row")
}

#[tokio::test]
async fn refresh_concurrency_replay_revokes_the_committed_successor_and_access() {
    let (state, actor, _, refresh) = fixture().await;
    let barrier = Arc::new(Barrier::new(3));
    let mut requests = Vec::new();
    for _ in 0..2 {
        let (state, raw, barrier) = (state.clone(), refresh.clone(), barrier.clone());
        requests.push(tokio::spawn(async move {
            barrier.wait().await;
            rotate_session(&state, &raw).await
        }));
    }
    barrier.wait().await;
    let results = futures::future::join_all(requests).await;
    let mut winners = Vec::new();
    let mut failures = Vec::new();
    for result in results {
        match result.unwrap() {
            Ok(pair) => winners.push(pair),
            Err(error) => failures.push(error),
        }
    }
    assert_eq!(winners.len(), 1);
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].status, StatusCode::UNAUTHORIZED);
    assert_eq!(active_tokens(&state.pool, &actor).await, 0);
    let (access, _, successor) = winners.pop().unwrap();
    assert_eq!(
        me(router(state.clone()), &access).await,
        StatusCode::UNAUTHORIZED
    );
    assert!(rotate_session(&state, &successor).await.is_err());
}

#[tokio::test]
async fn session_revocation_ban_deletion_and_identity_binding_apply_to_existing_access() {
    let (state, actor, access, _) = fixture().await;
    let app = router(state.clone());
    assert_eq!(me(app.clone(), &access).await, StatusCode::OK);
    let claims = state.jwt.parse(&access).unwrap();
    let forged = state
        .jwt
        .issue_access(
            &DiscordUserId::new("different-owner"),
            claims.sid,
            "admin",
            true,
        )
        .unwrap()
        .0;
    assert_eq!(me(app.clone(), &forged).await, StatusCode::UNAUTHORIZED);
    sqlx::query("UPDATE users SET is_banned = true WHERE discord_id = $1")
        .bind(&actor)
        .execute(&state.pool)
        .await
        .unwrap();
    assert_eq!(me(app.clone(), &access).await, StatusCode::UNAUTHORIZED);
    sqlx::query("UPDATE users SET is_banned = false WHERE discord_id = $1")
        .bind(&actor)
        .execute(&state.pool)
        .await
        .unwrap();
    assert_eq!(
        me(app.clone(), &access).await,
        StatusCode::UNAUTHORIZED,
        "unban never resurrects revoked sessions"
    );
    let (fresh, _, _) = issue_session(&state, &actor).await.unwrap();
    sqlx::query("UPDATE users SET deleted_at = now() WHERE discord_id = $1")
        .bind(&actor)
        .execute(&state.pool)
        .await
        .unwrap();
    assert_eq!(me(app, &fresh).await, StatusCode::UNAUTHORIZED);
}
