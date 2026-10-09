//! Derived statistics (`users.total_deployments`, `leaderboard_totals`) are recomputed inside the
//! transaction of every change that affects them: link confirmation attributes the history and
//! refreshes the leaderboard, and a relink to another account moves the totals.
//!
//! Each case checks the HTTP answer, the persisted ownership of the player rows, the stored
//! statistics and the served `users/{id}/stats` and leaderboard.

use crate::{common, telemetry_support};

use axum::Router;
use axum::http::StatusCode;
use serde_json::{Value, json};
use sqlx::PgPool;
use telemetry_support::match_reports::ReportingServer;
use telemetry_support::report_fixtures::{
    account_token, boot_with_state, counters, event, event_totals, leaderboard_kills, line,
    link_identity, player_rows, report, served_statistics, unique_discord_id,
};
use telemetry_support::{admin_token, call};
use uuid::Uuid;

fn unique(prefix: &str) -> String {
    common::unique_arma(prefix)
}

/// A fresh account: `(discord_id, username)`.
fn account(prefix: &str) -> (String, String) {
    (unique_discord_id(), unique(prefix))
}

/// Report `arma` with `kills` in a new match of `server`; answers the match id and the answer.
async fn reported_match(
    app: &Router,
    server: &ReportingServer,
    arma: &str,
    kills: i64,
) -> (Uuid, Value) {
    let src = unique("recompute");
    let match_id = server.register_match(app, &src).await;
    let (status, answer) = server
        .post_results(
            app,
            1,
            &report(
                &src,
                "success",
                vec![line(arma, "life-1", Some(counters(kills, 1)))],
            ),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer["applied"], true);
    (match_id, answer)
}

async fn owners(pool: &PgPool, match_id: Uuid) -> Vec<Option<String>> {
    player_rows(pool, match_id)
        .await
        .into_iter()
        .map(|row| row.4)
        .collect()
}

async fn stored_arma(pool: &PgPool, discord_id: &str) -> Option<String> {
    sqlx::query_scalar("SELECT arma_id FROM users WHERE discord_id = $1")
        .bind(discord_id)
        .fetch_one(pool)
        .await
        .expect("the read of users returns a row")
}

async fn unlink(app: &Router, bearer: &str) {
    let (status, body) = call(app, "DELETE", "/api/v1/me/link", Some(bearer), None, None).await;
    assert_eq!(status, StatusCode::OK, "unlink: {body}");
    assert_eq!(body["linked"], false);
}

#[tokio::test]
async fn statistics_recomputation_link_attributes_history_and_refreshes_the_leaderboard() {
    let (app, pool, state) = boot_with_state().await;
    let admin = admin_token(&app).await;
    let server = ReportingServer::open(&app, &pool, &unique("recompute-link")).await;
    let (discord, username) = account("recompute-link");
    let bearer = account_token(&state, &discord, &username, None).await;
    let arma = unique("recompute-link-arma");

    let (match_id, answer) = reported_match(&app, &server, &arma, 6).await;
    assert_eq!(
        (answer["linked"].as_i64(), answer["unlinked"].as_i64()),
        (Some(0), Some(1))
    );
    assert_eq!(owners(&pool, match_id).await, [None]);
    assert_eq!(leaderboard_kills(&pool, &discord).await, (None, 0));
    let source: String = sqlx::query_scalar("SELECT source_match_id FROM matches WHERE id = $1")
        .bind(match_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    let revived = event(
        "revived-1",
        1,
        "medical.revived",
        json!({ "subject_arma_id": arma }),
    );
    let (status, events) = server.post_events(&app, &source, json!([revived])).await;
    assert_eq!(status, StatusCode::OK, "{events}");
    let totals = event_totals(&pool, match_id).await;
    assert_eq!(totals.len(), 1);

    let (status, body) = link_identity(&app, &server, &bearer, &arma).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["linked"], true);
    assert_eq!(
        event_totals(&pool, match_id).await,
        totals,
        "event totals are keyed by arma_id"
    );
    let audited: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM audit_logs WHERE action = 'identity.link' AND target_id = $1
         AND message LIKE '%' || $2 || '%'",
    )
    .bind(&discord)
    .bind(server.server_id.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audited, 1, "the link audit names the confirming server");
    assert_eq!(
        owners(&pool, match_id).await,
        [Some(discord.clone())],
        "history is attributed"
    );
    assert_eq!(
        stored_arma(&pool, &discord).await.as_deref(),
        Some(arma.as_str())
    );
    assert_eq!(leaderboard_kills(&pool, &discord).await, (Some(6), 1));
    assert_eq!(
        served_statistics(&app, &admin, &discord, &username).await,
        (6, 1, Some(6))
    );
}

#[tokio::test]
async fn statistics_recomputation_relink_to_another_account_moves_the_totals() {
    let (app, pool, state) = boot_with_state().await;
    let admin = admin_token(&app).await;
    let server = ReportingServer::open(&app, &pool, &unique("recompute-relink")).await;
    let arma = unique("recompute-relink-arma");
    let (first, first_name) = account("recompute-first");
    let first_bearer = account_token(&state, &first, &first_name, Some(&arma)).await;
    let (second, second_name) = account("recompute-second");
    let second_bearer = account_token(&state, &second, &second_name, None).await;

    let (match_id, _) = reported_match(&app, &server, &arma, 5).await;
    assert_eq!(leaderboard_kills(&pool, &first).await, (Some(5), 1));
    unlink(&app, &first_bearer).await;
    let (status, body) = link_identity(&app, &server, &second_bearer, &arma).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    assert_eq!(owners(&pool, match_id).await, [Some(second.clone())]);
    assert_eq!(leaderboard_kills(&pool, &second).await, (Some(5), 1));
    assert_eq!(leaderboard_kills(&pool, &first).await, (None, 0));
    assert_eq!(
        served_statistics(&app, &admin, &second, &second_name).await,
        (5, 1, Some(5))
    );
    assert_eq!(
        served_statistics(&app, &admin, &first, &first_name).await,
        (0, 0, None)
    );
}
