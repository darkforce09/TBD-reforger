//! Generated operation traces and concurrent HTTP ingestion verify current identity attribution.
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use proptest::collection::vec;
use std::sync::Arc;
use tokio::sync::Barrier;
use tower::ServiceExt;
use website_api::{
    core::{
        application_state::AppState, configuration::Config, database, http_router,
        middleware::AuthUser,
    },
    identity_and_access::services::{
        identity_linking::{confirm_identity, unlink_identity},
        link_code_issuance::issue_link_code,
        session_authorization::authorize_session,
    },
};
mod common;

async fn fixture() -> (AppState, AuthUser, String, String) {
    let url = common::require_test_database_url().unwrap();
    let pool = database::connect(&url).await.unwrap();
    database::migrate(&pool).await.unwrap();
    let state = AppState::new(
        pool,
        Config::for_tests(url, "identity-attribution-sequences"),
    );
    let actor = format!("attribution-{}", uuid::Uuid::new_v4());
    let access = common::access_token(
        &state,
        "identity_attribution_sequences",
        &actor,
        "enlisted",
        false,
    )
    .await;
    let user = authorize_session(&state.pool, &state.cfg, &state.jwt.parse(&access).unwrap())
        .await
        .unwrap();
    (
        state,
        user,
        common::unique_arma("attribution"),
        uuid::Uuid::new_v4().to_string(),
    )
}

async fn ingest(state: AppState, arma: &str, source: &str) {
    let body = serde_json::json!({"match":{"source_match_id":source,"outcome":"success","winning_faction":"USA"},
        "players":[{"arma_id":arma,"role_played":"rifleman","source_event_id":"result",
        "counters":{"kills":7,"deaths":1,"team_kills":0,"longest_kill_m":100,"vehicles_destroyed":0,"is_command":false}}]});
    let response = http_router::router(state)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/ingest/match-results")
                .header("X-Service-Token", "test-service-token")
                .header("Content-Type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

async fn assert_consistent(state: &AppState, user: &AuthUser, arma: &str) {
    let owner: Option<String> = sqlx::query_scalar(
        "SELECT discord_id FROM users WHERE arma_id = $1 AND deleted_at IS NULL",
    )
    .bind(arma)
    .fetch_optional(&state.pool)
    .await
    .unwrap();
    let mismatches: i64 = sqlx::query_scalar("SELECT count(*) FROM match_player_stats WHERE arma_id = $1 AND discord_id IS DISTINCT FROM $2")
        .bind(arma).bind(&owner).fetch_one(&state.pool).await.unwrap();
    assert_eq!(
        mismatches, 0,
        "gameplay rows follow the currently verified identity owner"
    );
    let counts: (i64, i64) = sqlx::query_as("SELECT total_deployments, (SELECT count(DISTINCT match_id) FROM match_player_stats WHERE discord_id = $1)
        FROM users WHERE discord_id = $1").bind(&user.discord_id).fetch_one(&state.pool).await.unwrap();
    assert_eq!(
        counts.0, counts.1,
        "acknowledged operations include aggregate recomputation"
    );
    let pending: i64 = sqlx::query_scalar("SELECT count(*) FROM identity_link_codes WHERE discord_id=$1 AND consumed_at IS NULL AND cancelled_at IS NULL")
        .bind(&user.discord_id).fetch_one(&state.pool).await.unwrap();
    assert!(pending <= 1);
    let leaderboard: i64 = sqlx::query_scalar("SELECT COALESCE((SELECT missions_played FROM leaderboard_totals WHERE discord_id = $1),0)::bigint")
        .bind(&user.discord_id).fetch_one(&state.pool).await.unwrap();
    assert_eq!(leaderboard, counts.1);
}

#[tokio::test]
async fn ingestion_racing_unlink_cannot_restore_previous_attribution() {
    let (state, user, arma, source) = fixture().await;
    let code = issue_link_code(&state, &user).await.unwrap().0;
    confirm_identity(&state, &code, &arma, "Player")
        .await
        .unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let (s, a, src, b) = (state.clone(), arma.clone(), source.clone(), barrier.clone());
    let upload = tokio::spawn(async move {
        b.wait().await;
        ingest(s, &a, &src).await;
    });
    let (s, u, b) = (state.clone(), user.clone(), barrier.clone());
    let unlink = tokio::spawn(async move {
        b.wait().await;
        unlink_identity(&s, &u).await.unwrap();
    });
    barrier.wait().await;
    upload.await.unwrap();
    unlink.await.unwrap();
    assert_consistent(&state, &user, &arma).await;
    let identity: Option<String> =
        sqlx::query_scalar("SELECT arma_id FROM users WHERE discord_id=$1")
            .bind(&user.discord_id)
            .fetch_one(&state.pool)
            .await
            .unwrap();
    assert!(identity.is_none());
}

#[tokio::test]
async fn ingestion_racing_link_cannot_leave_new_history_unattributed() {
    let (state, user, arma, source) = fixture().await;
    let code = issue_link_code(&state, &user).await.unwrap().0;
    let barrier = Arc::new(Barrier::new(3));
    let (s, a, src, b) = (state.clone(), arma.clone(), source.clone(), barrier.clone());
    let upload = tokio::spawn(async move {
        b.wait().await;
        ingest(s, &a, &src).await;
    });
    let (s, a, b) = (state.clone(), arma.clone(), barrier.clone());
    let link = tokio::spawn(async move {
        b.wait().await;
        confirm_identity(&s, &code, &a, "Player").await.unwrap();
    });
    barrier.wait().await;
    upload.await.unwrap();
    link.await.unwrap();
    assert_consistent(&state, &user, &arma).await;
}

#[test]
fn generated_identity_operation_sequences_preserve_ownership_codes_and_aggregates() {
    let runtime = tokio::runtime::Runtime::new().unwrap();
    common::property_evidence::run_property(
        "generated_identity_operation_sequences_preserve_ownership_codes_and_aggregates",
        16,
        &vec(0u8..6, 1..25),
        |operations| {
            runtime.block_on(async {
                let (state, user, arma, source) = fixture().await;
                let mut code = None;
                let mut consumed = std::collections::BTreeMap::<String, String>::new();
                for operation in operations {
                    match operation {
                        0 => code = Some(issue_link_code(&state, &user).await.unwrap().0),
                        1 | 4 => {
                            if let Some(code) = &code {
                                let _ = confirm_identity(&state, code, &arma, "Player").await;
                            }
                        }
                        2 => unlink_identity(&state, &user).await.unwrap(),
                        3 => {
                            sqlx::query(
                                "UPDATE identity_link_codes SET expires_at = clock_timestamp() - interval '1 second'
                                 WHERE discord_id=$1 AND consumed_at IS NULL AND cancelled_at IS NULL",
                            )
                            .bind(&user.discord_id)
                            .execute(&state.pool)
                            .await
                            .unwrap();
                        }
                        _ => ingest(state.clone(), &arma, &source).await,
                    }
                    let spent: Vec<(String, String)> = sqlx::query_as(
                        "SELECT code, arma_id FROM identity_link_codes WHERE discord_id=$1 AND consumed_at IS NOT NULL",
                    )
                    .bind(&user.discord_id)
                    .fetch_all(&state.pool)
                    .await
                    .unwrap();
                    for (key, identity) in spent {
                        if let Some(prior) = consumed.insert(key, identity.clone()) {
                            assert_eq!(prior, identity);
                        }
                    }
                    assert_consistent(&state, &user, &arma).await;
                }
            });
            Ok(())
        },
    );
}
