//! Corrections: a higher revision's present counters replace the stored counters (so they and the
//! leaderboard may go down), and a finalized match never returns to `pending`.
//!
//! Each case checks the HTTP answer, the persisted rows and the derived statistics.

use crate::{common, telemetry_support};

use axum::http::StatusCode;
use serde_json::Value;
use telemetry_support::admin_token;
use telemetry_support::match_reports::ReportingServer;
use telemetry_support::report_fixtures::{
    boot_with_state, counters, leaderboard_kills, line, match_state, player_rows, report,
    seed_player, served_statistics,
};

fn refusal(body: &Value) -> &str {
    body["details"]["code"].as_str().unwrap_or_default()
}

fn unique(prefix: &str) -> String {
    common::unique_arma(prefix)
}

async fn apply(server: &ReportingServer, app: &axum::Router, revision: i64, body: &Value) {
    let (status, answer) = server.post_results(app, revision, body).await;
    assert_eq!(status, StatusCode::OK, "revision {revision}: {answer}");
    assert_eq!(answer["applied"], true, "revision {revision}: {answer}");
    assert_eq!(answer["revision"], revision);
}

#[tokio::test]
async fn telemetry_corrections_higher_revision_lowers_counters_and_the_leaderboard() {
    let (app, pool, _state) = boot_with_state().await;
    let admin = admin_token(&app).await;
    let server = ReportingServer::open(&app, &pool, &unique("corrections-lower")).await;
    let (discord, username, arma) = seed_player(&pool, "lower").await;
    let src = unique("lower");
    let match_id = server.register_match(&app, &src).await;

    apply(
        &server,
        &app,
        1,
        &report(
            &src,
            "success",
            vec![line(&arma, "life-1", Some(counters(8, 2)))],
        ),
    )
    .await;
    assert_eq!(leaderboard_kills(&pool, &discord).await, (Some(8), 1));
    apply(
        &server,
        &app,
        2,
        &report(
            &src,
            "success",
            vec![line(&arma, "life-1", Some(counters(3, 2)))],
        ),
    )
    .await;

    let rows = player_rows(&pool, match_id).await;
    assert_eq!(rows.len(), 1);
    assert_eq!((rows[0].2, rows[0].3), (Some(3), Some(2)));
    assert_eq!(match_state(&pool, match_id).await.0, 2);
    assert_eq!(leaderboard_kills(&pool, &discord).await, (Some(3), 1));
    assert_eq!(
        served_statistics(&app, &admin, &discord, &username).await,
        (3, 1, Some(3))
    );
}

#[tokio::test]
async fn telemetry_corrections_finalized_match_cannot_return_to_pending() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &unique("corrections-pending")).await;
    let (_, _, arma) = seed_player(&pool, "pending").await;
    let src = unique("pending");
    let match_id = server.register_match(&app, &src).await;
    apply(
        &server,
        &app,
        1,
        &report(
            &src,
            "success",
            vec![line(&arma, "life-1", Some(counters(2, 0)))],
        ),
    )
    .await;
    let stored = match_state(&pool, match_id).await;
    assert!(stored.3.is_some(), "a terminal outcome finalizes the match");
    let rows = player_rows(&pool, match_id).await;

    let (status, body) = server
        .post_results(
            &app,
            2,
            &report(
                &src,
                "pending",
                vec![line(&arma, "life-1", Some(counters(9, 9)))],
            ),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(refusal(&body), "MATCH_FINALIZED", "{body}");

    assert_eq!(
        match_state(&pool, match_id).await,
        stored,
        "nothing of revision 2 is applied"
    );
    assert_eq!(player_rows(&pool, match_id).await, rows);
}
