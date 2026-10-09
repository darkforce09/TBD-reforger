//! Corrections: a higher revision replaces what it names and keeps what it omits. Present
//! counters replace stored counters (so they may go down), a line without counters keeps them,
//! `removed_lines` deletes rows, a terminal outcome may be re-adjudicated but never returns to
//! `pending`, and `finalized_at` is set once.
//!
//! Each case checks the HTTP answer, the persisted rows and the derived statistics. Requires
//! `TEST_DATABASE_URL`.

mod common;
mod telemetry_support;

use axum::http::StatusCode;
use serde_json::{Value, json};
use sqlx::PgPool;
use telemetry_support::admin_token;
use telemetry_support::match_reports::ReportingServer;
use telemetry_support::report_fixtures::{
    boot_with_state, counters, leaderboard_kills, line, match_state, player_rows, report,
    seed_player, served_statistics,
};
use uuid::Uuid;

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

async fn role_played(pool: &PgPool, match_id: Uuid, arma: &str) -> Option<String> {
    sqlx::query_scalar(
        "SELECT role_played FROM match_player_stats WHERE match_id = $1 AND arma_id = $2",
    )
    .bind(match_id)
    .bind(arma)
    .fetch_one(pool)
    .await
    .expect("the read of match_player_stats returns a row")
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

#[tokio::test]
async fn telemetry_corrections_terminal_outcome_is_readjudicated_and_finalized_at_is_kept() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &unique("corrections-outcome")).await;
    let src = unique("outcome");
    let match_id = server.register_match(&app, &src).await;

    apply(&server, &app, 1, &report(&src, "pending", vec![])).await;
    let (_, _, outcome, finalized, _) = match_state(&pool, match_id).await;
    assert_eq!((outcome.as_str(), finalized), ("pending", None));

    apply(&server, &app, 2, &report(&src, "success", vec![])).await;
    let (_, _, outcome, finalized, _) = match_state(&pool, match_id).await;
    assert_eq!(outcome, "success");
    let finalized = finalized.expect("the first terminal outcome sets finalized_at");

    for (revision, outcome) in [(3, "failure"), (4, "aborted")] {
        apply(&server, &app, revision, &report(&src, outcome, vec![])).await;
        let (stored_revision, _, stored, again, _) = match_state(&pool, match_id).await;
        assert_eq!((stored_revision, stored.as_str()), (revision, outcome));
        assert_eq!(
            again,
            Some(finalized),
            "finalized_at is set once and never moves"
        );
    }
}

#[tokio::test]
async fn telemetry_corrections_omitted_lines_are_kept() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &unique("corrections-omitted")).await;
    let (alpha_discord, _, alpha) = seed_player(&pool, "omitted-a").await;
    let (bravo_discord, _, bravo) = seed_player(&pool, "omitted-b").await;
    let src = unique("omitted");
    let match_id = server.register_match(&app, &src).await;

    apply(
        &server,
        &app,
        1,
        &report(
            &src,
            "success",
            vec![
                line(&alpha, "life-1", Some(counters(2, 1))),
                line(&bravo, "life-1", Some(counters(5, 0))),
            ],
        ),
    )
    .await;
    let (status, answer) = server
        .post_results(
            &app,
            2,
            &report(
                &src,
                "success",
                vec![line(&alpha, "life-1", Some(counters(4, 1)))],
            ),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(
        answer["players"], 1,
        "the answer counts the submitted lines"
    );

    let rows = player_rows(&pool, match_id).await;
    let kills: Vec<(String, Option<i64>)> = rows.iter().map(|row| (row.0.clone(), row.2)).collect();
    let mut expected = vec![(alpha.clone(), Some(4)), (bravo.clone(), Some(5))];
    expected.sort();
    assert_eq!(
        kills, expected,
        "the omitted line keeps its row and counters"
    );
    assert_eq!(leaderboard_kills(&pool, &alpha_discord).await, (Some(4), 1));
    assert_eq!(leaderboard_kills(&pool, &bravo_discord).await, (Some(5), 1));
}

#[tokio::test]
async fn telemetry_corrections_line_without_counters_keeps_stored_counters() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &unique("corrections-claimless")).await;
    let (discord, _, arma) = seed_player(&pool, "claimless").await;
    let src = unique("claimless");
    let match_id = server.register_match(&app, &src).await;

    apply(
        &server,
        &app,
        1,
        &report(
            &src,
            "success",
            vec![line(&arma, "life-1", Some(counters(5, 2)))],
        ),
    )
    .await;
    let mut medic = line(&arma, "life-1", None);
    medic["role_played"] = json!("Medic");
    apply(&server, &app, 2, &report(&src, "success", vec![medic])).await;

    let rows = player_rows(&pool, match_id).await;
    assert_eq!(rows.len(), 1);
    assert_eq!(
        (rows[0].2, rows[0].3),
        (Some(5), Some(2)),
        "no counter claim keeps the counters"
    );
    assert_eq!(rows[0].4.as_deref(), Some(discord.as_str()));
    assert_eq!(
        role_played(&pool, match_id, &arma).await.as_deref(),
        Some("Medic"),
        "role always replaces"
    );
    assert_eq!(leaderboard_kills(&pool, &discord).await, (Some(5), 1));
}

#[tokio::test]
async fn telemetry_corrections_removed_lines_delete_rows_and_recompute_statistics() {
    let (app, pool, _state) = boot_with_state().await;
    let admin = admin_token(&app).await;
    let server = ReportingServer::open(&app, &pool, &unique("corrections-removed")).await;
    let (discord, username, arma) = seed_player(&pool, "removed").await;
    let src = unique("removed");
    let match_id = server.register_match(&app, &src).await;

    apply(
        &server,
        &app,
        1,
        &report(
            &src,
            "success",
            vec![
                line(&arma, "life-1", Some(counters(3, 1))),
                line(&arma, "life-2", Some(counters(4, 1))),
            ],
        ),
    )
    .await;
    assert_eq!(leaderboard_kills(&pool, &discord).await, (Some(7), 1));

    let mut removal = report(&src, "success", vec![]);
    removal["removed_lines"] = json!([
        { "arma_id": arma, "source_event_id": "life-2" },
        { "arma_id": arma, "source_event_id": "never-reported" },
    ]);
    apply(&server, &app, 2, &removal).await;
    let rows = player_rows(&pool, match_id).await;
    assert_eq!(rows.len(), 1);
    assert_eq!((rows[0].1.as_str(), rows[0].2), ("life-1", Some(3)));
    assert_eq!(leaderboard_kills(&pool, &discord).await, (Some(3), 1));
    assert_eq!(
        served_statistics(&app, &admin, &discord, &username).await,
        (3, 1, Some(3))
    );

    let mut removal = report(&src, "success", vec![]);
    removal["removed_lines"] = json!([{ "arma_id": arma, "source_event_id": "life-1" }]);
    apply(&server, &app, 3, &removal).await;
    assert!(player_rows(&pool, match_id).await.is_empty());
    assert_eq!(
        leaderboard_kills(&pool, &discord).await,
        (None, 0),
        "no line left, no deployment"
    );
    assert_eq!(
        served_statistics(&app, &admin, &discord, &username).await,
        (0, 0, None)
    );
}

#[tokio::test]
async fn telemetry_corrections_absent_match_fields_keep_stored_values() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &unique("corrections-fields")).await;
    let src = unique("fields");
    let match_id = server.register_match(&app, &src).await;
    let fields = |pool: PgPool| async move {
        sqlx::query_as::<_, (Option<String>, Option<chrono::DateTime<chrono::Utc>>)>(
            "SELECT winning_faction, ended_at FROM matches WHERE id = $1",
        )
        .bind(match_id)
        .fetch_one(&pool)
        .await
        .unwrap()
    };

    let mut first = report(&src, "success", vec![]);
    first["match"]["winning_faction"] = json!("US");
    first["match"]["ended_at"] = json!("2026-01-01T02:00:00Z");
    apply(&server, &app, 1, &first).await;
    let stored = fields(pool.clone()).await;
    assert_eq!(stored.0.as_deref(), Some("US"));
    assert!(stored.1.is_some());

    apply(&server, &app, 2, &report(&src, "success", vec![])).await;
    assert_eq!(
        fields(pool.clone()).await,
        stored,
        "absent fields keep the stored values"
    );

    let mut third = report(&src, "failure", vec![]);
    third["match"]["winning_faction"] = json!("USSR");
    apply(&server, &app, 3, &third).await;
    let (faction, ended) = fields(pool.clone()).await;
    assert_eq!(
        (faction.as_deref(), ended),
        (Some("USSR"), stored.1),
        "a present field replaces"
    );
}
