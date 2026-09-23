//! Identity ownership, single-use codes, and derived facts commit together under competing requests.

use axum::http::StatusCode;
use sqlx::PgPool;
use std::{future::Future, time::Duration};
use tokio::sync::Barrier;
use uuid::Uuid;
use website_api::core::{
    application_state::AppState, configuration::Config, database,
    error_handling::api_error::ApiError, middleware::AuthUser,
};
use website_api::identity_and_access::services::{
    identity_linking::{confirm_identity, unlink_identity},
    link_code_issuance::issue_link_code,
    session_authorization::authorize_session,
};

mod common;

async fn fixture() -> AppState {
    let url =
        common::require_test_database_url().expect("identity transaction tests require PostgreSQL");
    let pool = database::connect(&url)
        .await
        .expect("connect integration database");
    database::migrate(&pool)
        .await
        .expect("migrate integration database");
    AppState::new(pool, Config::for_tests(url, "identity-link-transactions"))
}

async fn actor(state: &AppState) -> AuthUser {
    let id = format!("identity-link-{}", Uuid::new_v4());
    let token =
        common::access_token(state, "identity_link_transactions", &id, "enlisted", false).await;
    authorize_session(&state.pool, &state.cfg, &state.jwt.parse(&token).unwrap())
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
    .unwrap()
}

async fn current_identity(pool: &PgPool, actor: &str) -> Option<String> {
    sqlx::query_scalar("SELECT arma_id FROM users WHERE discord_id = $1")
        .bind(actor)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn audit_count(pool: &PgPool, actor: &str, action: &str) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM audit_logs WHERE actor_id = $1 AND action = $2")
        .bind(actor)
        .bind(action)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn code_state(pool: &PgPool, code: &str) -> (bool, bool, Option<String>, Option<String>) {
    sqlx::query_as(
        "SELECT consumed_at IS NOT NULL, cancelled_at IS NOT NULL, arma_id,
        cancellation_reason FROM identity_link_codes WHERE code = $1",
    )
    .bind(code)
    .fetch_one(pool)
    .await
    .unwrap()
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

async fn historical_fact(pool: &PgPool, arma_id: &str) -> Uuid {
    let id: Uuid = sqlx::query_scalar(
        "INSERT INTO matches(source_match_id, started_at, outcome, created_at)
        VALUES ($1, now() - interval '1 day', 'success', now()) RETURNING id",
    )
    .bind(format!("identity-match-{}", Uuid::new_v4()))
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO match_player_stats(match_id, discord_id, arma_id, source_event_id, kills, created_at)
        VALUES ($1, NULL, $2, $3, 7, now())")
        .bind(id).bind(arma_id).bind(format!("identity-result-{}", Uuid::new_v4()))
        .execute(pool).await.unwrap();
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
    .unwrap()
}

#[tokio::test]
async fn link_code_atomic_issuance_keeps_one_pending_code_for_competing_requests() {
    let state = fixture().await;
    let user = actor(&state).await;
    let (first, second) = race(
        issue_link_code(&state, &user),
        issue_link_code(&state, &user),
    )
    .await;
    let (first, first_expiry) = first.unwrap();
    let (second, second_expiry) = second.unwrap();
    assert_ne!(
        first, second,
        "superseding issuance does not recycle the previous code"
    );
    assert!(first_expiry > chrono::Utc::now());
    assert!(second_expiry > chrono::Utc::now());
    assert_eq!(pending_codes(&state.pool, &user.discord_id).await, 1);
    let first_state = code_state(&state.pool, &first).await;
    let second_state = code_state(&state.pool, &second).await;
    assert!(
        !first_state.0 && !second_state.0,
        "issuance cannot consume a code"
    );
    assert_ne!(
        first_state.1, second_state.1,
        "exactly one issuance supersedes the other"
    );
    let cancelled = if first_state.1 {
        first_state
    } else {
        second_state
    };
    assert_eq!(cancelled.3.as_deref(), Some("superseded"));
    assert_eq!(
        audit_count(&state.pool, &user.discord_id, "identity.link_code_issued").await,
        2
    );
}

#[tokio::test]
async fn linking_single_code_cannot_assign_two_different_arma_identities() {
    let state = fixture().await;
    let user = actor(&state).await;
    let (code, _) = issue_link_code(&state, &user).await.unwrap();
    let identities = [
        format!("arma-{}", Uuid::new_v4()),
        format!("arma-{}", Uuid::new_v4()),
    ];
    let (first, second) = race(
        confirm_identity(&state, &code, &identities[0], "First player"),
        confirm_identity(&state, &code, &identities[1], "Second player"),
    )
    .await;
    let winner = one_winner(first, second);
    assert_eq!(
        current_identity(&state.pool, &user.discord_id)
            .await
            .as_deref(),
        Some(identities[winner].as_str())
    );
    let spent = code_state(&state.pool, &code).await;
    assert!(spent.0 && !spent.1);
    assert_eq!(spent.2.as_deref(), Some(identities[winner].as_str()));
    assert_eq!(pending_codes(&state.pool, &user.discord_id).await, 0);
    assert_eq!(
        audit_count(&state.pool, &user.discord_id, "identity.link").await,
        1
    );
}

#[tokio::test]
async fn linking_identity_ownership_has_one_winner_across_two_accounts() {
    let state = fixture().await;
    let first = actor(&state).await;
    let second = actor(&state).await;
    let (first_code, _) = issue_link_code(&state, &first).await.unwrap();
    let (second_code, _) = issue_link_code(&state, &second).await.unwrap();
    let identity = format!("arma-{}", Uuid::new_v4());
    let history = historical_fact(&state.pool, &identity).await;
    let (left, right) = race(
        confirm_identity(&state, &first_code, &identity, "Shared player"),
        confirm_identity(&state, &second_code, &identity, "Shared player"),
    )
    .await;
    let winner = one_winner(left, right);
    let users = [&first, &second];
    let codes = [&first_code, &second_code];
    assert_eq!(
        current_identity(&state.pool, &users[winner].discord_id)
            .await
            .as_deref(),
        Some(identity.as_str())
    );
    assert!(
        current_identity(&state.pool, &users[1 - winner].discord_id)
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
        pending_codes(&state.pool, &users[1 - winner].discord_id).await,
        1
    );
    assert_eq!(
        audit_count(&state.pool, &users[winner].discord_id, "identity.link").await,
        1
    );
    assert_eq!(
        audit_count(&state.pool, &users[1 - winner].discord_id, "identity.link").await,
        0
    );
    let owners: i64 = sqlx::query_scalar("SELECT count(*) FROM users WHERE arma_id = $1")
        .bind(&identity)
        .fetch_one(&state.pool)
        .await
        .unwrap();
    assert_eq!(owners, 1);
}

#[tokio::test]
async fn linking_confirmation_racing_unlink_never_restores_released_ownership() {
    let state = fixture().await;
    let user = actor(&state).await;
    let (code, _) = issue_link_code(&state, &user).await.unwrap();
    let identity = format!("arma-{}", Uuid::new_v4());
    let history = historical_fact(&state.pool, &identity).await;
    let (confirmation, unlink) = race(
        confirm_identity(&state, &code, &identity, "Concurrent player"),
        unlink_identity(&state, &user),
    )
    .await;
    unlink.expect("unlink must commit");
    if let Err(error) = &confirmation {
        assert_eq!(
            error.status,
            StatusCode::NOT_FOUND,
            "unlink-first cancels the unconsumed code"
        );
    }
    assert!(
        current_identity(&state.pool, &user.discord_id)
            .await
            .is_none()
    );
    assert!(
        historical_owner(&state.pool, history, &identity)
            .await
            .is_none()
    );
    assert_eq!(pending_codes(&state.pool, &user.discord_id).await, 0);
    let terminal = code_state(&state.pool, &code).await;
    assert_ne!(
        terminal.0, terminal.1,
        "code is consumed first or cancelled first, never both"
    );
    assert_eq!(terminal.0, confirmation.is_ok());
    if terminal.1 {
        assert_eq!(terminal.3.as_deref(), Some("unlinked"));
    }
    assert_eq!(
        audit_count(&state.pool, &user.discord_id, "identity.unlink").await,
        1
    );
    assert_eq!(
        audit_count(&state.pool, &user.discord_id, "identity.link").await,
        i64::from(confirmation.is_ok())
    );
    assert!(
        confirm_identity(&state, &code, &identity, "Late retry")
            .await
            .is_err()
    );
    assert!(
        current_identity(&state.pool, &user.discord_id)
            .await
            .is_none()
    );
}

#[tokio::test]
async fn linking_unlink_cancels_pending_codes_even_when_consumed_confirmation_retries() {
    let state = fixture().await;
    let user = actor(&state).await;
    let identity = format!("arma-{}", Uuid::new_v4());
    let (consumed, _) = issue_link_code(&state, &user).await.unwrap();
    confirm_identity(&state, &consumed, &identity, "Linked player")
        .await
        .unwrap();
    let (pending, _) = issue_link_code(&state, &user).await.unwrap();
    let (retry, unlink) = race(
        confirm_identity(&state, &consumed, &identity, "Duplicate player"),
        unlink_identity(&state, &user),
    )
    .await;
    unlink.unwrap();
    if let Err(error) = retry {
        assert_eq!(error.status, StatusCode::CONFLICT);
    }
    assert!(
        current_identity(&state.pool, &user.discord_id)
            .await
            .is_none()
    );
    assert_eq!(pending_codes(&state.pool, &user.discord_id).await, 0);
    assert_eq!(
        code_state(&state.pool, &pending).await,
        (false, true, None, Some("unlinked".into()))
    );
    assert!(
        code_state(&state.pool, &consumed).await.0,
        "consumption history remains factual"
    );
    assert_eq!(
        audit_count(&state.pool, &user.discord_id, "identity.link").await,
        1
    );
}

async fn install_stats_failure(pool: &PgPool, actor: &str) -> String {
    let name = format!("reject_link_stats_{}", Uuid::new_v4().simple());
    assert!(
        actor
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    );
    let sql = format!(
        "CREATE FUNCTION {name}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
        RAISE EXCEPTION 'injected identity statistics failure'; RETURN NEW; END $$;
        CREATE TRIGGER {name} BEFORE UPDATE OF total_deployments ON users
        FOR EACH ROW WHEN (NEW.discord_id = '{actor}') EXECUTE FUNCTION {name}();"
    );
    sqlx::raw_sql(sqlx::AssertSqlSafe(sql.as_str()))
        .execute(pool)
        .await
        .unwrap();
    name
}

async fn remove_stats_failure(pool: &PgPool, name: &str) {
    assert!(
        name.bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    );
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!(
        "DROP TRIGGER {name} ON users; DROP FUNCTION {name}();"
    )))
    .execute(pool)
    .await
    .unwrap();
}

async fn audit_snapshot(pool: &PgPool, actor: &str) -> (i64, i64) {
    sqlx::query_as("SELECT
        (SELECT count(*) FROM audit_logs WHERE actor_id = $1),
        (SELECT count(*) FROM audit_publication_pending p JOIN audit_logs a ON a.id = p.audit_id WHERE a.actor_id = $1)")
        .bind(actor).fetch_one(pool).await.unwrap()
}

#[tokio::test]
async fn linking_statistics_failure_rolls_back_code_ownership_facts_and_required_audit() {
    let state = fixture().await;
    let user = actor(&state).await;
    let identity = format!("arma-{}", Uuid::new_v4());
    let history = historical_fact(&state.pool, &identity).await;
    let (code, _) = issue_link_code(&state, &user).await.unwrap();
    let audits = audit_snapshot(&state.pool, &user.discord_id).await;
    let code_before = code_state(&state.pool, &code).await;
    let trigger = install_stats_failure(&state.pool, &user.discord_id).await;
    let failed = confirm_identity(&state, &code, &identity, "Recoverable player").await;
    remove_stats_failure(&state.pool, &trigger).await;
    assert_eq!(
        failed
            .expect_err("derived statistics failure must abort linking")
            .status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert!(
        current_identity(&state.pool, &user.discord_id)
            .await
            .is_none()
    );
    assert_eq!(code_state(&state.pool, &code).await, code_before);
    assert!(
        historical_owner(&state.pool, history, &identity)
            .await
            .is_none()
    );
    assert_eq!(audit_snapshot(&state.pool, &user.discord_id).await, audits);
    let deployments: i64 =
        sqlx::query_scalar("SELECT total_deployments FROM users WHERE discord_id = $1")
            .bind(&user.discord_id)
            .fetch_one(&state.pool)
            .await
            .unwrap();
    assert_eq!(deployments, 0);

    assert_eq!(
        confirm_identity(&state, &code, &identity, "Recoverable player")
            .await
            .unwrap()
            .discord_id,
        user.discord_id
    );
    assert_eq!(
        current_identity(&state.pool, &user.discord_id)
            .await
            .as_deref(),
        Some(identity.as_str())
    );
    assert_eq!(
        historical_owner(&state.pool, history, &identity)
            .await
            .as_deref(),
        Some(user.discord_id.as_str())
    );
    assert!(code_state(&state.pool, &code).await.0);
    let deployments: i64 =
        sqlx::query_scalar("SELECT total_deployments FROM users WHERE discord_id = $1")
            .bind(&user.discord_id)
            .fetch_one(&state.pool)
            .await
            .unwrap();
    assert_eq!(deployments, 1);
    assert_eq!(
        audit_count(&state.pool, &user.discord_id, "identity.link").await,
        1
    );
    assert_eq!(
        audit_snapshot(&state.pool, &user.discord_id).await,
        (audits.0 + 1, audits.1 + 1)
    );
}

#[tokio::test]
async fn linking_duplicate_same_identity_confirmation_is_audited_once() {
    let state = fixture().await;
    let user = actor(&state).await;
    let identity = format!("arma-{}", Uuid::new_v4());
    let (code, _) = issue_link_code(&state, &user).await.unwrap();
    let (first, second) = race(
        confirm_identity(&state, &code, &identity, "Original character"),
        confirm_identity(&state, &code, &identity, "Original character"),
    )
    .await;
    assert_eq!(first.unwrap().discord_id, user.discord_id);
    assert_eq!(second.unwrap().discord_id, user.discord_id);
    let before = code_state(&state.pool, &code).await;
    assert_eq!(
        confirm_identity(&state, &code, &identity, "Changed duplicate character")
            .await
            .unwrap()
            .discord_id,
        user.discord_id
    );
    assert_eq!(code_state(&state.pool, &code).await, before);
    assert_eq!(
        audit_count(&state.pool, &user.discord_id, "identity.link").await,
        1
    );
    let character: String =
        sqlx::query_scalar("SELECT arma_character FROM users WHERE discord_id = $1")
            .bind(&user.discord_id)
            .fetch_one(&state.pool)
            .await
            .unwrap();
    assert_eq!(
        character, "Original character",
        "retry cannot mutate committed character details"
    );
}
