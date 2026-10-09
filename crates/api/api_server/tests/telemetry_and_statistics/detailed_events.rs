//! Detailed events: the seven kinds and their payloads, batch validation and conflicts under the
//! match row lock, late delivery after finalization, and the sequence-ordered, paged read of
//! `GET /api/v1/matches/{matchId}/events`. Request bodies, answers and pages are checked against
//! `match-telemetry.schema.json` and the types generated from it.
//!
//! Each case checks the HTTP answer and the persisted rows. Requires `TEST_DATABASE_URL`.

use crate::{common, contract_support, telemetry_support};

use axum::Router;
use axum::http::StatusCode;
use contract_schema_types::match_telemetry::match_telemetry::TelemetryRefusal;
use serde_json::{Value, json};
use telemetry_support::match_reports::ReportingServer;
use telemetry_support::report_fixtures::{
    boot_with_state, event, event_totals, match_state, stored_events,
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

fn same_instant(left: &Value, right: &Value) -> bool {
    let parse = |value: &Value| {
        chrono::DateTime::parse_from_rfc3339(
            value.as_str().expect("the timestamp value is a string"),
        )
        .expect("the timestamp is RFC 3339")
    };
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
