//! Real PostgreSQL barriers verify identity transactions coexist with foreign-key row locks.

use axum::{
    body::{Body, to_bytes},
    http::{Request, StatusCode},
};
use std::time::Duration;
use tower::ServiceExt;
use uuid::Uuid;
use website_api::{
    core::{
        application_state::AppState, configuration::Config, database, http_router,
        middleware::AuthUser,
    },
    identity_and_access::services::{
        identity_linking::confirm_identity, link_code_issuance::issue_link_code,
        session_authorization::authorize_session,
    },
};

mod common;

const LOCK_DEADLINE: Duration = Duration::from_secs(5);

async fn fixture() -> (AppState, AuthUser, String) {
    let url = common::require_test_database_url().expect("scratch database required");
    let pool = database::connect(&url).await.unwrap();
    database::migrate(&pool).await.unwrap();
    let state = AppState::new(pool, Config::for_tests(url, "identity-foreign-key-locks"));
    let actor = format!("identity-lock-{}", Uuid::new_v4());
    let access = common::access_token(
        &state,
        "identity_foreign_key_locks",
        &actor,
        "enlisted",
        false,
    )
    .await;
    let user = authorize_session(&state.pool, &state.cfg, &state.jwt.parse(&access).unwrap())
        .await
        .unwrap();
    (state, user, common::unique_arma("identity-lock"))
}

#[tokio::test]
async fn ingest_waiting_on_event_allows_registration_foreign_key_account_lock() {
    let (state, user, arma) = fixture().await;
    let code = issue_link_code(&state, &user).await.unwrap().0;
    confirm_identity(&state, &code, &arma, "Lock Barrier Player")
        .await
        .unwrap();
    let event: Uuid = sqlx::query_scalar(
        "INSERT INTO events (name_override, start_time, created_by, created_at, updated_at)
         VALUES ('Identity lock barrier', now(), $1, now(), now()) RETURNING id",
    )
    .bind(&user.discord_id)
    .fetch_one(&state.pool)
    .await
    .unwrap();

    // Registration holds this event lock before its assigned_to foreign key checks the account.
    let mut registration = state.pool.begin().await.unwrap();
    sqlx::query("SELECT id FROM events WHERE id = $1 FOR UPDATE")
        .bind(event)
        .fetch_one(&mut *registration)
        .await
        .unwrap();
    let blocker_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *registration)
        .await
        .unwrap();
    let source = Uuid::new_v4().to_string();
    let body = serde_json::json!({
        "match": {"source_match_id": source, "event_id": event, "outcome": "success"},
        "players": [{"arma_id": arma, "source_event_id": "result", "role_played": "rifleman"}]
    });
    let app = http_router::router(state.clone());
    let ingest = tokio::spawn(async move {
        app.oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/ingest/match-results")
                .header("X-Service-Token", "test-service-token")
                .header("Content-Type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
    });

    tokio::time::timeout(LOCK_DEADLINE, async {
        loop {
            let blocked: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity
                 WHERE datname = current_database() AND $1 = ANY(pg_blocking_pids(pid))
                 AND query LIKE '%INSERT INTO matches%')",
            )
            .bind(blocker_pid)
            .fetch_one(&state.pool)
            .await
            .unwrap();
            if blocked {
                break;
            }
            assert!(
                !ingest.is_finished(),
                "ingest must reach the event FK barrier"
            );
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("ingest must block on the held event, after locking its linked account");

    // NOWAIT makes an incompatible account lock a deterministic failure instead of a deadlock.
    let account_lock = sqlx::query_scalar::<_, String>(
        "SELECT discord_id FROM users WHERE discord_id = $1 FOR KEY SHARE NOWAIT",
    )
    .bind(&user.discord_id)
    .fetch_one(&mut *registration)
    .await;
    registration.rollback().await.unwrap();
    let response = tokio::time::timeout(LOCK_DEADLINE, ingest)
        .await
        .expect("ingest must finish after the event lock is released")
        .unwrap();
    assert_eq!(
        account_lock.expect("ingest account locks must admit the registration FK check"),
        user.discord_id
    );
    let status = response.status();
    let response_body = to_bytes(response.into_body(), 1024 * 1024).await.unwrap();
    assert_eq!(status, StatusCode::OK, "{response_body:?}");
    let attributed: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM match_player_stats s JOIN matches m ON m.id = s.match_id
         WHERE m.source_match_id = $1 AND m.event_id = $2 AND s.discord_id = $3",
    )
    .bind(&source)
    .bind(event)
    .bind(&user.discord_id)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(
        attributed, 1,
        "the released HTTP request commits its attribution"
    );
}

#[tokio::test]
async fn identity_confirmation_does_not_upgrade_past_a_foreign_key_account_lock() {
    let (state, user, arma) = fixture().await;
    let code = issue_link_code(&state, &user).await.unwrap().0;
    let mut foreign_key_check = state.pool.begin().await.unwrap();
    sqlx::query("SELECT discord_id FROM users WHERE discord_id = $1 FOR KEY SHARE")
        .bind(&user.discord_id)
        .fetch_one(&mut *foreign_key_check)
        .await
        .unwrap();

    // This covers the actual arma_id UPDATE and commit, including any implicit row-lock upgrade.
    let confirmation = tokio::time::timeout(
        LOCK_DEADLINE,
        confirm_identity(&state, &code, &arma, "Foreign Key Player"),
    )
    .await;
    foreign_key_check.rollback().await.unwrap();
    let confirmed = confirmation
        .expect("identity confirmation must commit while the account KEY SHARE lock remains held")
        .expect("identity confirmation must succeed");
    assert_eq!(confirmed.discord_id, user.discord_id);
    let linked: (Option<String>, Option<String>, bool) = sqlx::query_as(
        "SELECT u.arma_id, c.arma_id, c.consumed_at IS NOT NULL
         FROM users u JOIN identity_link_codes c ON c.discord_id = u.discord_id
         WHERE u.discord_id = $1 AND c.code = $2",
    )
    .bind(&user.discord_id)
    .bind(&code)
    .fetch_one(&state.pool)
    .await
    .unwrap();
    assert_eq!(linked, (Some(arma.clone()), Some(arma), true));
}
