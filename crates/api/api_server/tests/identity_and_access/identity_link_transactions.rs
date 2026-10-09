//! Identity ownership, single-use codes, and derived facts commit together under competing requests.

use crate::common;

use api_caller_identity::session_authorization::authorize_session;
use api_configuration::configuration::Config;
use api_foundation::error_handling::api_error::ApiError;
use api_http_layer::middleware::AuthUser;
use api_identifiers::{ArmaPlayerId, ServerId};
use api_identity_and_access::services::{
    identity_linking::confirm_identity, link_code_issuance::issue_link_code,
};
use api_state::AppState;
use axum::http::StatusCode;
use sqlx::PgPool;
use std::{future::Future, time::Duration};
use tokio::sync::Barrier;
use uuid::Uuid;

async fn fixture() -> AppState {
    let url =
        common::require_test_database_url().expect("identity transaction tests require PostgreSQL");
    let pool = api_database::connect(&url)
        .await
        .expect("connect integration database");
    api_database::migrate(&pool)
        .await
        .expect("migrate integration database");
    api_server::composition::application_state(
        pool,
        Config::for_tests(url, "identity-link-transactions"),
    )
}

async fn actor(state: &AppState) -> AuthUser {
    let id = format!("identity-link-{}", Uuid::new_v4());
    let token =
        common::access_token(state, "identity_link_transactions", &id, "enlisted", false).await;
    authorize_session(
        &state.pool,
        &state.cfg,
        &state
            .jwt
            .parse(&token)
            .expect("the issued access token parses"),
    )
    .await
    .expect("authorize persisted actor session")
}

async fn race<A: Future, B: Future>(left: A, right: B) -> (A::Output, B::Output) {
    let barrier = Barrier::new(2);
    tokio::time::timeout(Duration::from_secs(30), async {
        tokio::join!(
            async {
                barrier.wait().await;
                left.await
            },
            async {
                barrier.wait().await;
                right.await
            },
        )
    })
    .await
    .expect("competing identity transactions must terminate without deadlock")
}

async fn pending_codes(pool: &PgPool, actor: &str) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM identity_link_codes
        WHERE discord_id = $1 AND consumed_at IS NULL AND cancelled_at IS NULL",
    )
    .bind(actor)
    .fetch_one(pool)
    .await
    .expect("the read of identity_link_codes returns a row")
}

async fn current_identity(pool: &PgPool, actor: &str) -> Option<String> {
    sqlx::query_scalar("SELECT arma_id FROM users WHERE discord_id = $1")
        .bind(actor)
        .fetch_one(pool)
        .await
        .expect("the read of users returns a row")
}

async fn audit_count(pool: &PgPool, actor: &str, action: &str) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM audit_logs WHERE actor_id = $1 AND action = $2")
        .bind(actor)
        .bind(action)
        .fetch_one(pool)
        .await
        .expect("the read of audit_logs returns a row")
}

async fn code_state(pool: &PgPool, code: &str) -> (bool, bool, Option<String>, Option<String>) {
    sqlx::query_as(
        "SELECT consumed_at IS NOT NULL, cancelled_at IS NOT NULL, arma_id,
        cancellation_reason FROM identity_link_codes WHERE code = $1",
    )
    .bind(code)
    .fetch_one(pool)
    .await
    .expect("the read of identity_link_codes returns a row")
}

fn one_winner<T: std::fmt::Debug>(
    first: Result<T, ApiError>,
    second: Result<T, ApiError>,
) -> usize {
    match (first, second) {
        (Ok(_), Err(error)) => {
            assert_eq!(error.status, StatusCode::CONFLICT);
            0
        }
        (Err(error), Ok(_)) => {
            assert_eq!(error.status, StatusCode::CONFLICT);
            1
        }
        (first, second) => {
            panic!("expected exactly one confirmation and one conflict: {first:?}, {second:?}")
        }
    }
}

/// A registered game server to confirm link codes from; confirmations name it in their audit.
async fn confirming_server(pool: &PgPool) -> ServerId {
    sqlx::query_scalar(
        "INSERT INTO servers (name, ip, port, is_active)
         VALUES ('Identity confirmation server', '127.0.0.1'::inet, 2001, true) RETURNING id",
    )
    .fetch_one(pool)
    .await
    .expect("the insert into servers returns its row")
}

async fn historical_fact(pool: &PgPool, arma_id: &str) -> Uuid {
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO matches(source_match_id, started_at, outcome, created_at)
        VALUES ($1, now() - interval '1 day', 'success', now()) RETURNING id",
    )
    .bind(format!("identity-match-{}", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .expect("the insert into matches returns its row");
    sqlx::query("INSERT INTO match_player_stats(match_id, discord_id, arma_id, source_event_id, kills, created_at)
        VALUES ($1, NULL, $2, $3, 7, now())")
        .bind(id).bind(arma_id).bind(format!("identity-result-{}", Uuid::new_v4()))
        .execute(pool).await.expect("the insert into match_player_stats succeeds");
    id
}

async fn historical_owner(pool: &PgPool, match_id: Uuid, arma_id: &str) -> Option<String> {
    sqlx::query_scalar(
        "SELECT discord_id FROM match_player_stats WHERE match_id = $1 AND arma_id = $2",
    )
    .bind(match_id)
    .bind(arma_id)
    .fetch_one(pool)
    .await
    .expect("the read of match_player_stats returns a row")
}

#[tokio::test]
async fn linking_single_code_cannot_assign_two_different_arma_identities() {
    let state = fixture().await;
    let server = confirming_server(&state.pool).await;
    let user = actor(&state).await;
    let (code, _) = issue_link_code(&state, &user).await.unwrap();
    let identities = [
        format!("arma-{}", Uuid::new_v4()),
        format!("arma-{}", Uuid::new_v4()),
    ];
    let (first, second) = race(
        confirm_identity(
            &state,
            server,
            &code,
            &ArmaPlayerId::new(identities[0].as_str()),
            "First player",
        ),
        confirm_identity(
            &state,
            server,
            &code,
            &ArmaPlayerId::new(identities[1].as_str()),
            "Second player",
        ),
    )
    .await;
    let winner = one_winner(first, second);
    assert_eq!(
        current_identity(&state.pool, user.discord_id.as_str())
            .await
            .as_deref(),
        Some(identities[winner].as_str())
    );
    let spent = code_state(&state.pool, &code).await;
    assert!(spent.0 && !spent.1);
    assert_eq!(spent.2.as_deref(), Some(identities[winner].as_str()));
    assert_eq!(
        pending_codes(&state.pool, user.discord_id.as_str()).await,
        0
    );
    assert_eq!(
        audit_count(&state.pool, user.discord_id.as_str(), "identity.link").await,
        1
    );
}

#[tokio::test]
async fn linking_identity_ownership_has_one_winner_across_two_accounts() {
    let state = fixture().await;
    let server = confirming_server(&state.pool).await;
    let first = actor(&state).await;
    let second = actor(&state).await;
    let (first_code, _) = issue_link_code(&state, &first).await.unwrap();
    let (second_code, _) = issue_link_code(&state, &second).await.unwrap();
    let identity = format!("arma-{}", Uuid::new_v4());
    let history = historical_fact(&state.pool, &identity).await;
    let (left, right) = race(
        confirm_identity(
            &state,
            server,
            &first_code,
            &ArmaPlayerId::new(identity.as_str()),
            "Shared player",
        ),
        confirm_identity(
            &state,
            server,
            &second_code,
            &ArmaPlayerId::new(identity.as_str()),
            "Shared player",
        ),
    )
    .await;
    let winner = one_winner(left, right);
    let users = [&first, &second];
    let codes = [&first_code, &second_code];
    assert_eq!(
        current_identity(&state.pool, users[winner].discord_id.as_str())
            .await
            .as_deref(),
        Some(identity.as_str())
    );
    assert!(
        current_identity(&state.pool, users[1 - winner].discord_id.as_str())
            .await
            .is_none()
    );
    assert_eq!(
        historical_owner(&state.pool, history, &identity)
            .await
            .as_deref(),
        Some(users[winner].discord_id.as_str())
    );
    assert!(code_state(&state.pool, codes[winner]).await.0);
    assert!(!code_state(&state.pool, codes[1 - winner]).await.0);
    assert_eq!(
        pending_codes(&state.pool, users[1 - winner].discord_id.as_str()).await,
        1
    );
    assert_eq!(
        audit_count(
            &state.pool,
            users[winner].discord_id.as_str(),
            "identity.link"
        )
        .await,
        1
    );
    assert_eq!(
        audit_count(
            &state.pool,
            users[1 - winner].discord_id.as_str(),
            "identity.link"
        )
        .await,
        0
    );
    let owners: i64 = sqlx::query_scalar("SELECT count(*) FROM users WHERE arma_id = $1")
        .bind(&identity)
        .fetch_one(&state.pool)
        .await
        .unwrap();
    assert_eq!(owners, 1);
}
