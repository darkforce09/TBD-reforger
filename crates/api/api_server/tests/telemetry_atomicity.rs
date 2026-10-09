//! Atomicity: a results revision and an event batch are validated whole before the
//! transaction opens, so one invalid entry rejects everything and names its index; an applied
//! batch counts only the events it actually inserted, so retries, overlaps and concurrent
//! duplicates never count an event twice.
//!
//! Each case checks the HTTP answer and the persisted rows. Requires `TEST_DATABASE_URL`.

mod common;
mod telemetry_support;

use axum::http::StatusCode;
use serde_json::{Value, json};
use telemetry_support::match_reports::ReportingServer;
use telemetry_support::report_fixtures::{
    boot_with_state, counters, event, event_totals, leaderboard_kills, line, match_state,
    player_rows, report, seed_player, stored_events,
};

fn refusal(body: &Value) -> &str {
    body["details"]["code"].as_str().unwrap_or_default()
}

fn unique(prefix: &str) -> String {
    common::unique_arma(prefix)
}

fn kill(id: &str, sequence: i64, killer: &str, victim: &str) -> Value {
    event(
        id,
        sequence,
        "combat.kill",
        json!({
            "killer_arma_id": killer, "victim_arma_id": victim, "victim_is_player": true,
            "team_kill": false, "distance_m": 120.5, "weapon": "M16A2",
        }),
    )
}

fn death(id: &str, sequence: i64, victim: &str) -> Value {
    event(
        id,
        sequence,
        "combat.death",
        json!({ "victim_arma_id": victim, "cause": "ai" }),
    )
}

fn totals(entries: &[(&str, &str, &str, i64)]) -> Vec<(String, String, String, i64)> {
    let mut rows: Vec<(String, String, String, i64)> = entries
        .iter()
        .map(|(arma, kind, role, count)| {
            (arma.to_string(), kind.to_string(), role.to_string(), *count)
        })
        .collect();
    rows.sort();
    rows
}

#[tokio::test]
async fn telemetry_atomicity_invalid_player_line_rejects_the_whole_revision() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &unique("atomic-lines")).await;
    let (discord, _, arma) = seed_player(&pool, "atomic-lines").await;
    let newcomer = unique("atomic-newcomer");
    let src = unique("atomic-lines");
    let match_id = server.register_match(&app, &src).await;
    let (status, body) = server
        .post_results(
            &app,
            1,
            &report(
                &src,
                "success",
                vec![line(&arma, "life-1", Some(counters(2, 0)))],
            ),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let stored = match_state(&pool, match_id).await;
    let rows = player_rows(&pool, match_id).await;

    let mut negative = counters(1, 0);
    negative["kills"] = json!(-1);
    let (status, body) = server
        .post_results(
            &app,
            2,
            &report(
                &src,
                "failure",
                vec![
                    line(&arma, "life-1", Some(counters(9, 9))),
                    line(&newcomer, "life-1", Some(counters(1, 1))),
                    line(&newcomer, "life-2", Some(negative)),
                ],
            ),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(refusal(&body), "INVALID_MATCH_RESULTS", "{body}");
    assert_eq!(
        body["details"]["index"], 2,
        "the first invalid line is named: {body}"
    );
    assert!(body["details"]["field"].is_string(), "{body}");

    let mut removal = report(
        &src,
        "failure",
        vec![line(&arma, "life-1", Some(counters(9, 9)))],
    );
    removal["removed_lines"] = json!([
        { "arma_id": arma, "source_event_id": "life-9" },
        { "arma_id": "", "source_event_id": "life-1" },
    ]);
    let (status, body) = server.post_results(&app, 2, &removal).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(refusal(&body), "INVALID_MATCH_RESULTS", "{body}");
    assert_eq!(
        body["details"]["index"], 1,
        "the index is the position in removed_lines: {body}"
    );

    assert_eq!(
        match_state(&pool, match_id).await,
        stored,
        "nothing of revision 2 is written"
    );
    assert_eq!(player_rows(&pool, match_id).await, rows);
    assert_eq!(leaderboard_kills(&pool, &discord).await, (Some(2), 1));
}

#[tokio::test]
async fn telemetry_atomicity_invalid_event_rejects_the_whole_batch() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &unique("atomic-events")).await;
    let src = unique("atomic-events");
    let match_id = server.register_match(&app, &src).await;
    let (killer, victim) = (unique("atomic-killer"), unique("atomic-victim"));

    let (status, body) = server
        .post_events(
            &app,
            &src,
            json!([
                kill("k-1", 1, &killer, &victim),
                death("d-1", 2, &victim),
                event(
                    "d-2",
                    3,
                    "combat.death",
                    json!({ "victim_arma_id": victim, "cause": "gremlins" })
                ),
            ]),
        )
        .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(refusal(&body), "INVALID_EVENT", "{body}");
    assert_eq!(body["details"]["index"], 2, "{body}");
    assert!(body["details"]["field"].is_string(), "{body}");

    assert!(
        stored_events(&pool, match_id).await.is_empty(),
        "nothing of the batch is persisted"
    );
    assert!(event_totals(&pool, match_id).await.is_empty());
    assert_eq!(match_state(&pool, match_id).await.4, 0);
}

#[tokio::test]
async fn telemetry_atomicity_event_retry_does_not_double_count() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &unique("atomic-retry")).await;
    let src = unique("atomic-retry");
    let match_id = server.register_match(&app, &src).await;
    let (killer, victim) = (unique("retry-killer"), unique("retry-victim"));
    let batch = json!([kill("k-1", 1, &killer, &victim), death("d-1", 2, &victim)]);
    let expected = totals(&[
        (&killer, "combat.kill", "actor", 1),
        (&victim, "combat.kill", "subject", 1),
        (&victim, "combat.death", "subject", 1),
    ]);

    let (status, first) = server.post_events(&app, &src, batch.clone()).await;
    assert_eq!(status, StatusCode::OK, "{first}");
    assert_eq!(
        (
            first["accepted"].clone(),
            first["duplicates"].clone(),
            first["event_count"].clone()
        ),
        (json!(2), json!(0), json!(2))
    );
    assert_eq!(first["last_sequence"], 2);
    assert_eq!(event_totals(&pool, match_id).await, expected);

    let (status, retry) = server.post_events(&app, &src, batch).await;
    assert_eq!(status, StatusCode::OK, "{retry}");
    assert_eq!(
        (
            retry["accepted"].clone(),
            retry["duplicates"].clone(),
            retry["event_count"].clone()
        ),
        (json!(0), json!(2), json!(2))
    );
    assert_eq!(
        event_totals(&pool, match_id).await,
        expected,
        "a retry counts nothing"
    );
    assert_eq!(match_state(&pool, match_id).await.4, 2);
    assert_eq!(stored_events(&pool, match_id).await.len(), 2);
}

#[tokio::test]
async fn telemetry_atomicity_overlapping_batch_counts_only_new_events() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &unique("atomic-overlap")).await;
    let src = unique("atomic-overlap");
    let match_id = server.register_match(&app, &src).await;
    let (killer, victim) = (unique("overlap-killer"), unique("overlap-victim"));

    let (status, first) = server
        .post_events(
            &app,
            &src,
            json!([kill("k-1", 1, &killer, &victim), death("d-1", 2, &victim)]),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{first}");
    let (status, overlap) = server
        .post_events(
            &app,
            &src,
            json!([death("d-1", 2, &victim), kill("k-2", 3, &killer, &victim)]),
        )
        .await;
    assert_eq!(status, StatusCode::OK, "{overlap}");
    assert_eq!(
        (overlap["accepted"].clone(), overlap["duplicates"].clone()),
        (json!(1), json!(1))
    );
    assert_eq!(
        (
            overlap["event_count"].clone(),
            overlap["last_sequence"].clone()
        ),
        (json!(3), json!(3))
    );

    assert_eq!(
        event_totals(&pool, match_id).await,
        totals(&[
            (&killer, "combat.kill", "actor", 2),
            (&victim, "combat.kill", "subject", 2),
            (&victim, "combat.death", "subject", 1),
        ])
    );
    assert_eq!(match_state(&pool, match_id).await.4, 3);
    let ids: Vec<String> = stored_events(&pool, match_id)
        .await
        .into_iter()
        .map(|e| e.0)
        .collect();
    assert_eq!(ids, ["k-1", "d-1", "k-2"]);
}

#[tokio::test]
async fn telemetry_atomicity_concurrent_identical_batches_count_once() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &unique("atomic-race")).await;
    let src = unique("atomic-race");
    let match_id = server.register_match(&app, &src).await;
    let (killer, victim) = (unique("race-killer"), unique("race-victim"));
    let batch: Vec<Value> = (1..=20)
        .map(|sequence| kill(&format!("k-{sequence}"), sequence, &killer, &victim))
        .collect();

    let (left, right) = tokio::join!(
        server.post_events(&app, &src, json!(batch.clone())),
        server.post_events(&app, &src, json!(batch.clone()))
    );
    assert_eq!(
        (left.0, right.0),
        (StatusCode::OK, StatusCode::OK),
        "{} {}",
        left.1,
        right.1
    );
    let accepted = left.1["accepted"].as_i64().unwrap() + right.1["accepted"].as_i64().unwrap();
    let duplicates =
        left.1["duplicates"].as_i64().unwrap() + right.1["duplicates"].as_i64().unwrap();
    assert_eq!(
        (accepted, duplicates),
        (20, 20),
        "every event is inserted exactly once"
    );

    assert_eq!(match_state(&pool, match_id).await.4, 20);
    assert_eq!(stored_events(&pool, match_id).await.len(), 20);
    assert_eq!(
        event_totals(&pool, match_id).await,
        totals(&[
            (&killer, "combat.kill", "actor", 20),
            (&victim, "combat.kill", "subject", 20)
        ])
    );
}

#[tokio::test]
async fn telemetry_atomicity_duplicate_event_id_or_sequence_inside_one_batch_is_refused() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &unique("atomic-dupes")).await;
    let src = unique("atomic-dupes");
    let match_id = server.register_match(&app, &src).await;
    let victim = unique("dupes-victim");

    let (status, body) = server
        .post_events(
            &app,
            &src,
            json!([death("d-1", 1, &victim), death("d-1", 2, &victim)]),
        )
        .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "duplicate event_id: {body}"
    );
    assert_eq!(refusal(&body), "INVALID_EVENT", "{body}");
    assert_eq!(body["details"]["index"], 1, "{body}");

    let (status, body) = server
        .post_events(
            &app,
            &src,
            json!([death("d-1", 1, &victim), death("d-2", 1, &victim)]),
        )
        .await;
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "duplicate sequence: {body}"
    );
    assert_eq!(refusal(&body), "INVALID_EVENT", "{body}");
    assert_eq!(body["details"]["index"], 1, "{body}");

    assert!(stored_events(&pool, match_id).await.is_empty());
    assert!(event_totals(&pool, match_id).await.is_empty());
    assert_eq!(match_state(&pool, match_id).await.4, 0);
}
