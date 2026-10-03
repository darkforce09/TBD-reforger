//! Detailed events: the seven kinds and their payloads, batch validation and conflicts under the
//! match row lock, late delivery after finalization, and the sequence-ordered, paged read of
//! `GET /api/v1/matches/{matchId}/events`. Request bodies, answers and pages are checked against
//! `match-telemetry.schema.json` and the types generated from it.
//!
//! Each case checks the HTTP answer and the persisted rows. Requires `TEST_DATABASE_URL`.

mod common;
mod contract_support;
mod telemetry_support;

use axum::Router;
use axum::http::StatusCode;
use contract_schema_types::match_telemetry::match_telemetry::{
    MatchEvent, MatchEventBatch, MatchEventBatchAnswer, MatchEventPage, TelemetryRefusal,
};
use serde_json::{Value, json};
use telemetry_support::match_reports::ReportingServer;
use telemetry_support::report_fixtures::{
    boot_with_state, event, event_totals, match_state, report, stored_events,
};
use telemetry_support::{admin_token, call};
use uuid::Uuid;

const SCHEMA: &str = "match-telemetry.schema.json";

fn refusal(body: &Value) -> &str {
    body["details"]["code"].as_str().unwrap_or_default()
}

fn unique(prefix: &str) -> String {
    common::unique_arma(prefix)
}

fn revived(id: &str, sequence: i64, subject: &str) -> Value {
    event(
        id,
        sequence,
        "medical.revived",
        json!({ "subject_arma_id": subject }),
    )
}

/// One event of each kind; `alpha` acts and `bravo` is acted upon.
fn every_kind(alpha: &str, bravo: &str) -> Vec<Value> {
    vec![
        event(
            "kill-1",
            1,
            "combat.kill",
            json!({
                "killer_arma_id": alpha, "victim_arma_id": bravo, "victim_is_player": true,
                "team_kill": false, "distance_m": 312.75, "weapon": "M21 SWS",
            }),
        ),
        event(
            "death-1",
            2,
            "combat.death",
            json!({ "victim_arma_id": bravo, "cause": "environment" }),
        ),
        event(
            "down-1",
            3,
            "medical.incapacitated",
            json!({ "subject_arma_id": bravo }),
        ),
        revived("up-1", 4, bravo),
        event(
            "wreck-1",
            5,
            "vehicle.destroyed",
            json!({
                "vehicle_prefab": "{5E74787B3B3E0E4C}Prefabs/Vehicles/Wheeled/UAZ469/UAZ469.et",
                "instigator_arma_id": alpha,
            }),
        ),
        event(
            "mount-1",
            6,
            "vehicle.entered",
            json!({
                "arma_id": alpha, "vehicle_prefab": "Prefabs/Vehicles/M151A2.et", "compartment": "pilot",
            }),
        ),
        event(
            "dismount-1",
            7,
            "vehicle.exited",
            json!({
                "arma_id": alpha, "vehicle_prefab": "Prefabs/Vehicles/M151A2.et", "compartment": "pilot",
            }),
        ),
    ]
}

async fn page(app: &Router, bearer: &str, match_id: Uuid, query: &str) -> Value {
    let (status, body) = call(
        app,
        "GET",
        &format!("/api/v1/matches/{match_id}/events{query}"),
        Some(bearer),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{query}: {body}");
    body
}

fn sequences(page: &Value) -> Vec<i64> {
    page["items"]
        .as_array()
        .expect("items")
        .iter()
        .map(|item| item["sequence"].as_i64().unwrap())
        .collect()
}

fn same_instant(left: &Value, right: &Value) -> bool {
    let parse =
        |value: &Value| chrono::DateTime::parse_from_rfc3339(value.as_str().unwrap()).unwrap();
    parse(left) == parse(right)
}

#[tokio::test]
async fn detailed_events_every_kind_round_trips() {
    let (app, pool, _state) = boot_with_state().await;
    let admin = admin_token(&app).await;
    let server = ReportingServer::open(&app, &pool, &unique("events-kinds")).await;
    let src = unique("kinds");
    let match_id = server.register_match(&app, &src).await;
    let (alpha, bravo) = (unique("kinds-alpha"), unique("kinds-bravo"));
    let sent = every_kind(&alpha, &bravo);

    let (status, answer) = server.post_events(&app, &src, json!(sent)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(
        (answer["accepted"].clone(), answer["event_count"].clone()),
        (json!(7), json!(7))
    );

    let read = page(&app, &admin, match_id, "").await;
    let items = read["items"].as_array().unwrap();
    assert_eq!(items.len(), 7);
    assert_eq!(read["next_after_sequence"], Value::Null);
    for (item, original) in items.iter().zip(&sent) {
        for key in ["event_id", "sequence", "kind", "mission_time_ms", "payload"] {
            assert_eq!(
                item[key], original[key],
                "{key} of {}",
                original["event_id"]
            );
        }
        assert!(
            same_instant(&item["occurred_at"], &original["occurred_at"]),
            "{item}"
        );
    }

    let kinds: Vec<String> = stored_events(&pool, match_id)
        .await
        .into_iter()
        .map(|e| e.2)
        .collect();
    assert_eq!(
        kinds,
        [
            "combat.kill",
            "combat.death",
            "medical.incapacitated",
            "medical.revived",
            "vehicle.destroyed",
            "vehicle.entered",
            "vehicle.exited"
        ]
    );
    let participants: Vec<(String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT kind, actor_arma_id, subject_arma_id FROM match_events WHERE match_id = $1 ORDER BY sequence",
    )
    .bind(match_id)
    .fetch_all(&pool)
    .await
    .unwrap();
    let (a, b) = (Some(alpha.clone()), Some(bravo.clone()));
    let expected: Vec<(Option<String>, Option<String>)> = vec![
        (a.clone(), b.clone()),
        (None, b.clone()),
        (None, b.clone()),
        (None, b.clone()),
        (a.clone(), None),
        (a.clone(), None),
        (a.clone(), None),
    ];
    let actual: Vec<(Option<String>, Option<String>)> = participants
        .into_iter()
        .map(|(_, actor, subject)| (actor, subject))
        .collect();
    assert_eq!(actual, expected, "actor and subject follow the kind table");

    let mut totals = vec![
        (
            alpha.clone(),
            "combat.kill".to_owned(),
            "actor".to_owned(),
            1,
        ),
        (
            alpha.clone(),
            "vehicle.destroyed".to_owned(),
            "actor".to_owned(),
            1,
        ),
        (
            alpha.clone(),
            "vehicle.entered".to_owned(),
            "actor".to_owned(),
            1,
        ),
        (
            alpha.clone(),
            "vehicle.exited".to_owned(),
            "actor".to_owned(),
            1,
        ),
        (
            bravo.clone(),
            "combat.death".to_owned(),
            "subject".to_owned(),
            1,
        ),
        (
            bravo.clone(),
            "combat.kill".to_owned(),
            "subject".to_owned(),
            1,
        ),
        (
            bravo.clone(),
            "medical.incapacitated".to_owned(),
            "subject".to_owned(),
            1,
        ),
        (
            bravo.clone(),
            "medical.revived".to_owned(),
            "subject".to_owned(),
            1,
        ),
    ];
    totals.sort();
    assert_eq!(event_totals(&pool, match_id).await, totals);
}

#[tokio::test]
async fn detailed_events_read_order_is_sequence_order_not_arrival_order() {
    let (app, pool, _state) = boot_with_state().await;
    let admin = admin_token(&app).await;
    let server = ReportingServer::open(&app, &pool, &unique("events-order")).await;
    let src = unique("order");
    let match_id = server.register_match(&app, &src).await;
    let subject = unique("order-subject");

    for batch in [vec![5, 3], vec![1, 4, 2]] {
        let events: Vec<Value> = batch
            .iter()
            .map(|&n| revived(&format!("r-{n}"), n, &subject))
            .collect();
        let (status, answer) = server.post_events(&app, &src, json!(events)).await;
        assert_eq!(status, StatusCode::OK, "{answer}");
    }
    let read = page(&app, &admin, match_id, "").await;
    assert_eq!(sequences(&read), [1, 2, 3, 4, 5]);
    let ids: Vec<&str> = read["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["event_id"].as_str().unwrap())
        .collect();
    assert_eq!(ids, ["r-1", "r-2", "r-3", "r-4", "r-5"]);
    let stored: Vec<i64> = stored_events(&pool, match_id)
        .await
        .into_iter()
        .map(|e| e.1)
        .collect();
    assert_eq!(stored, [1, 2, 3, 4, 5]);
    assert_eq!(match_state(&pool, match_id).await.4, 5);
}

#[tokio::test]
async fn detailed_events_pages_with_after_sequence_and_limit() {
    let (app, pool, _state) = boot_with_state().await;
    let admin = admin_token(&app).await;
    let server = ReportingServer::open(&app, &pool, &unique("events-paging")).await;
    let src = unique("paging");
    let match_id = server.register_match(&app, &src).await;
    let subject = unique("paging-subject");
    let events: Vec<Value> = (1..=7)
        .map(|n| revived(&format!("p-{n}"), n * 10, &subject))
        .collect();
    let (status, answer) = server.post_events(&app, &src, json!(events)).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer["last_sequence"], 70);

    let first = page(&app, &admin, match_id, "?limit=3").await;
    assert_eq!(
        (sequences(&first), first["next_after_sequence"].clone()),
        (vec![10, 20, 30], json!(30))
    );
    let second = page(&app, &admin, match_id, "?after_sequence=30&limit=3").await;
    assert_eq!(
        (sequences(&second), second["next_after_sequence"].clone()),
        (vec![40, 50, 60], json!(60))
    );
    let last = page(&app, &admin, match_id, "?after_sequence=60&limit=3").await;
    assert_eq!(
        (sequences(&last), last["next_after_sequence"].clone()),
        (vec![70], Value::Null)
    );
    let exact = page(&app, &admin, match_id, "?after_sequence=40&limit=3").await;
    assert_eq!(
        (sequences(&exact), exact["next_after_sequence"].clone()),
        (vec![50, 60, 70], Value::Null),
        "a page holding the last event is the last page"
    );
    let between = page(&app, &admin, match_id, "?after_sequence=25&limit=2").await;
    assert_eq!(sequences(&between), [30, 40]);
    assert_eq!(stored_events(&pool, match_id).await.len(), 7);

    let (status, _) = call(
        &app,
        "GET",
        &format!("/api/v1/matches/{match_id}/events"),
        None,
        None,
        None,
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "the read requires a signed-in user"
    );
}

#[tokio::test]
async fn detailed_events_same_event_id_with_another_payload_conflicts() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &unique("events-conflict")).await;
    let src = unique("conflict");
    let match_id = server.register_match(&app, &src).await;
    let (first, second) = (unique("conflict-a"), unique("conflict-b"));
    let (status, body) = server
        .post_events(&app, &src, json!([revived("e-1", 1, &first)]))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let totals = event_totals(&pool, match_id).await;

    let (status, body) = server
        .post_events(
            &app,
            &src,
            json!([revived("e-2", 2, &second), revived("e-1", 1, &second)]),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(refusal(&body), "EVENT_CONFLICT", "{body}");
    assert_eq!(body["details"]["index"], 1, "{body}");
    contract_support::assert_valid(SCHEMA, Some("TelemetryRefusal"), &body["details"]);
    contract_support::assert_decodes::<TelemetryRefusal>("TelemetryRefusal", &body["details"]);

    let ids: Vec<String> = stored_events(&pool, match_id)
        .await
        .into_iter()
        .map(|e| e.0)
        .collect();
    assert_eq!(ids, ["e-1"], "nothing of the refused batch is written");
    assert_eq!(event_totals(&pool, match_id).await, totals);
    assert_eq!(match_state(&pool, match_id).await.4, 1);
}

#[tokio::test]
async fn detailed_events_sequence_reuse_under_another_id_conflicts() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &unique("events-sequence")).await;
    let src = unique("sequence");
    let match_id = server.register_match(&app, &src).await;
    let subject = unique("sequence-subject");
    let (status, body) = server
        .post_events(&app, &src, json!([revived("e-1", 1, &subject)]))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let (status, body) = server
        .post_events(
            &app,
            &src,
            json!([revived("e-9", 9, &subject), revived("e-2", 1, &subject)]),
        )
        .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(refusal(&body), "EVENT_SEQUENCE_CONFLICT", "{body}");
    assert_eq!(body["details"]["index"], 1, "{body}");

    let ids: Vec<String> = stored_events(&pool, match_id)
        .await
        .into_iter()
        .map(|e| e.0)
        .collect();
    assert_eq!(ids, ["e-1"]);
    assert_eq!(event_totals(&pool, match_id).await.len(), 1);
    assert_eq!(match_state(&pool, match_id).await.4, 1);
}

#[tokio::test]
async fn detailed_events_unknown_kind_or_malformed_event_is_invalid() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &unique("events-invalid")).await;
    let src = unique("invalid");
    let match_id = server.register_match(&app, &src).await;
    let subject = unique("invalid-subject");
    let valid = revived("ok-1", 1, &subject);

    let mut negative_distance = event(
        "k-1",
        2,
        "combat.kill",
        json!({
            "killer_arma_id": subject, "victim_is_player": false, "team_kill": false, "distance_m": -1,
        }),
    );
    let long_arma = "a".repeat(129);
    let mut bad_id = revived("ok-2", 2, &subject);
    bad_id["event_id"] = json!("has spaces");
    let mut zero_sequence = revived("ok-2", 2, &subject);
    zero_sequence["sequence"] = json!(0);
    let mut unknown_key = revived("ok-2", 2, &subject);
    unknown_key["payload"]["extra"] = json!(true);
    negative_distance["mission_time_ms"] = json!(5);
    for (label, invalid) in [
        (
            "unknown kind",
            event(
                "x-1",
                2,
                "combat.teabag",
                json!({ "subject_arma_id": subject }),
            ),
        ),
        (
            "missing payload field",
            event(
                "x-1",
                2,
                "combat.death",
                json!({ "victim_arma_id": subject }),
            ),
        ),
        ("negative distance", negative_distance),
        ("arma id above 128 bytes", revived("x-1", 2, &long_arma)),
        ("event id pattern", bad_id),
        ("sequence below 1", zero_sequence),
        ("unknown payload key", unknown_key),
    ] {
        let (status, body) = server
            .post_events(&app, &src, json!([valid.clone(), invalid]))
            .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{label}: {body}");
        assert_eq!(refusal(&body), "INVALID_EVENT", "{label}: {body}");
        assert_eq!(body["details"]["index"], 1, "{label}: {body}");
    }
    let (status, body) = server.post_events(&app, &src, json!([])).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "an empty batch: {body}");

    assert!(stored_events(&pool, match_id).await.is_empty());
    assert!(event_totals(&pool, match_id).await.is_empty());
    assert_eq!(match_state(&pool, match_id).await.4, 0);
}

#[tokio::test]
async fn detailed_events_batch_above_500_is_too_large() {
    let (app, pool, _state) = boot_with_state().await;
    let admin = admin_token(&app).await;
    let server = ReportingServer::open(&app, &pool, &unique("events-size")).await;
    let src = unique("size");
    let match_id = server.register_match(&app, &src).await;
    let subject = unique("size-subject");
    let batch = |count: i64| -> Value {
        json!(
            (1..=count)
                .map(|n| revived(&format!("s-{n}"), n, &subject))
                .collect::<Vec<_>>()
        )
    };

    let (status, body) = server.post_events(&app, &src, batch(501)).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(refusal(&body), "EVENT_BATCH_TOO_LARGE", "{body}");
    assert!(stored_events(&pool, match_id).await.is_empty());
    assert_eq!(match_state(&pool, match_id).await.4, 0);

    let (status, body) = server.post_events(&app, &src, batch(500)).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "500 events is the ceiling, not above it: {body}"
    );
    assert_eq!(
        (body["accepted"].clone(), body["last_sequence"].clone()),
        (json!(500), json!(500))
    );
    assert_eq!(match_state(&pool, match_id).await.4, 500);

    let default = page(&app, &admin, match_id, "").await;
    assert_eq!(
        default["items"].as_array().unwrap().len(),
        100,
        "the default limit is 100"
    );
    assert_eq!(default["next_after_sequence"], 100);
    let full = page(&app, &admin, match_id, "?limit=500").await;
    assert_eq!(full["items"].as_array().unwrap().len(), 500);
    assert_eq!(full["next_after_sequence"], Value::Null);
}

#[tokio::test]
async fn detailed_events_are_accepted_after_finalization() {
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &unique("events-late")).await;
    let src = unique("late");
    let match_id = server.register_match(&app, &src).await;
    let (status, body) = server
        .post_results(&app, 1, &report(&src, "success", vec![]))
        .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let finalized = match_state(&pool, match_id).await;
    assert!(finalized.3.is_some());

    let subject = unique("late-subject");
    let (status, body) = server
        .post_events(&app, &src, json!([revived("late-1", 1, &subject)]))
        .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "late delivery after finalization: {body}"
    );
    assert_eq!(body["accepted"], 1);
    let after = match_state(&pool, match_id).await;
    assert_eq!(
        (after.0, after.2.as_str(), after.3, after.4),
        (1, "success", finalized.3, 1)
    );
    assert_eq!(stored_events(&pool, match_id).await.len(), 1);
}

#[tokio::test]
async fn detailed_events_bodies_and_pages_follow_the_contract() {
    let (app, pool, _state) = boot_with_state().await;
    let admin = admin_token(&app).await;
    let server = ReportingServer::open(&app, &pool, &unique("events-contract")).await;
    let src = unique("contract");
    let match_id = server.register_match(&app, &src).await;
    let events = every_kind(&unique("contract-alpha"), &unique("contract-bravo"));
    let body = json!({ "source_match_id": src, "events": events });
    contract_support::assert_valid(SCHEMA, Some("MatchEventBatch"), &body);
    contract_support::assert_decodes::<MatchEventBatch>("MatchEventBatch", &body);
    for event in &events {
        contract_support::assert_decodes::<MatchEvent>("MatchEvent", event);
    }

    let (status, answer) = server
        .post(&app, "/api/v1/ingest/match-events", &body)
        .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    contract_support::assert_valid(SCHEMA, Some("MatchEventBatchAnswer"), &answer);
    contract_support::assert_decodes::<MatchEventBatchAnswer>("MatchEventBatchAnswer", &answer);
    assert_eq!(answer["match_id"], json!(match_id));

    for query in ["", "?limit=4", "?after_sequence=4&limit=4"] {
        let read = page(&app, &admin, match_id, query).await;
        contract_support::assert_valid(SCHEMA, None, &read);
        contract_support::assert_decodes::<MatchEventPage>("MatchEventPage", &read);
    }
    assert_eq!(stored_events(&pool, match_id).await.len(), 7);
}
