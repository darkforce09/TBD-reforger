//! HTTP regressions for metadata-only telemetry corrections and whole-batch identity validation.

use axum::{Router, http::StatusCode};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

mod common;
mod telemetry_support;

async fn post(app: &Router, body: &Value) -> (StatusCode, Value) {
    telemetry_support::call(
        app,
        "POST",
        "/api/v1/ingest/match-results",
        None,
        Some(telemetry_support::SVC),
        Some(&body.to_string()),
    )
    .await
}

async fn registration_state(pool: &PgPool, event_mission: Uuid, actor: &str) -> String {
    sqlx::query_scalar(
        "SELECT state::text FROM event_registrations WHERE event_mission_id = $1 AND discord_id = $2",
    )
    .bind(event_mission)
    .bind(actor)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn aggregates(pool: &PgPool, actor: &str) -> (i64, f64, i64, i64) {
    sqlx::query_as(
        "SELECT total_deployments, attendance_rate::float8,
         (SELECT count(*) FROM event_registrations WHERE discord_id = $1 AND state = 'attended'),
         (SELECT count(*) FROM event_registrations r JOIN event_missions em ON em.id = r.event_mission_id
          WHERE r.discord_id = $1 AND em.start_time <= now() AND r.attendance_state IS NOT NULL)
         FROM users WHERE discord_id = $1",
    )
    .bind(actor)
    .fetch_one(pool)
    .await
    .unwrap()
}

#[tokio::test]
async fn telemetry_partial_correction_moves_attendance_using_preserved_player_facts() {
    let (app, pool) = telemetry_support::boot()
        .await
        .expect("partial correction tests require PostgreSQL");
    let actor = format!("partial-correction-{}", Uuid::new_v4());
    let arma = format!("partial-arma-{}", Uuid::new_v4());
    common::seed_user(
        &pool,
        &actor,
        "Partial correction player",
        &arma,
        "enlisted",
    )
    .await;
    let mission: Uuid = sqlx::query_scalar(
        "INSERT INTO missions(title, author_id, terrain, game_mode, max_players, status, created_at, updated_at)
         VALUES ('Partial correction mission', $1, 'everon', 'pve_coop', 32, 'live', now(), now())
         RETURNING id",
    )
    .bind(&actor)
    .fetch_one(&pool)
    .await
    .unwrap();
    let mut events = Vec::new();
    let mut event_missions = Vec::new();
    for name in ["Original operation", "Corrected operation"] {
        let event: Uuid = sqlx::query_scalar(
            "INSERT INTO events(name_override, start_time, status, created_by, created_at, updated_at)
             VALUES ($1, now() - interval '3 hours', 'open', $2, now(), now()) RETURNING id",
        )
        .bind(name)
        .bind(&actor)
        .fetch_one(&pool)
        .await
        .unwrap();
        let event_mission: Uuid = sqlx::query_scalar(
            "INSERT INTO event_missions(event_id, mission_id, start_time, created_at, updated_at)
             VALUES ($1, $2, now() - interval '2 hours', now(), now()) RETURNING id",
        )
        .bind(event)
        .bind(mission)
        .fetch_one(&pool)
        .await
        .unwrap();
        let mut fixture = pool.begin().await.unwrap();
        let allocation = common::participant_allocation(&mut fixture, event_mission, &actor).await;
        sqlx::query(
            "INSERT INTO event_registrations(event_mission_id, discord_id, reservation_state, allocation_id)
             VALUES ($1, $2, 'registered', $3)",
        )
        .bind(event_mission)
        .bind(&actor)
        .bind(allocation)
        .execute(&mut *fixture)
        .await
        .unwrap();
        fixture.commit().await.unwrap();
        events.push(event);
        event_missions.push(event_mission);
    }

    let source = format!("partial-match-{}", Uuid::new_v4());
    let source_event = format!("partial-result-{}", Uuid::new_v4());
    let initial = json!({
        "match": {
            "source_match_id": source,
            "event_id": events[0],
            "mission_id": mission,
            "outcome": "success"
        },
        "players": [{
            "arma_id": arma,
            "source_event_id": source_event,
            "role_played": "Squad Leader",
            "counters": {
                "kills": 7, "deaths": 2, "team_kills": 0, "longest_kill_m": 300,
                "vehicles_destroyed": 1, "is_command": true, "command_win": true
            }
        }]
    });
    let (status, first) = post(&app, &initial).await;
    assert_eq!(status, StatusCode::OK, "initial results: {first}");
    assert_eq!(first["players"], 1);
    let match_id: Uuid = first["match_id"].as_str().unwrap().parse().unwrap();
    assert_eq!(
        registration_state(&pool, event_missions[0], &actor).await,
        "attended"
    );
    assert_eq!(
        registration_state(&pool, event_missions[1], &actor).await,
        "registered"
    );
    assert_eq!(aggregates(&pool, &actor).await, (1, 100.0, 1, 1));
    let fact_before: (Uuid, String, String, String, i64, i64) = sqlx::query_as(
        "SELECT id, discord_id, arma_id, source_event_id, kills, deaths
         FROM match_player_stats WHERE match_id = $1",
    )
    .bind(match_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    let correction = json!({
        "match": {
            "source_match_id": source,
            "event_id": events[1],
            "mission_id": mission,
            "outcome": "success"
        },
        "players": []
    });
    for attempt in 0..2 {
        let (status, corrected) = post(&app, &correction).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "correction/retry {attempt}: {corrected}"
        );
        assert_eq!(corrected["match_id"], first["match_id"]);
        assert_eq!(
            corrected["players"], 0,
            "response reports submitted roster length"
        );
        assert_eq!(corrected["linked"], 0);
        assert_eq!(corrected["unlinked"], 0);
        assert_eq!(
            registration_state(&pool, event_missions[0], &actor).await,
            "registered"
        );
        assert_eq!(
            registration_state(&pool, event_missions[1], &actor).await,
            "attended"
        );
        assert_eq!(aggregates(&pool, &actor).await, (1, 100.0, 1, 1));
        let fact_after: (Uuid, String, String, String, i64, i64) = sqlx::query_as(
            "SELECT id, discord_id, arma_id, source_event_id, kills, deaths
             FROM match_player_stats WHERE match_id = $1",
        )
        .bind(match_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            fact_after, fact_before,
            "omitted player facts retain identity and counters"
        );
        let stored: (i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM matches WHERE source_match_id = $1),
             (SELECT count(*) FROM match_player_stats WHERE match_id = $2)",
        )
        .bind(&source)
        .bind(match_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            stored,
            (1, 1),
            "metadata correction cannot duplicate matches or player facts"
        );
        let stored_event: Uuid = sqlx::query_scalar("SELECT event_id FROM matches WHERE id = $1")
            .bind(match_id)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(stored_event, events[1]);
    }
}

#[tokio::test]
async fn telemetry_oversized_normalized_arma_identity_rejects_the_entire_batch() {
    let (app, pool) = telemetry_support::boot()
        .await
        .expect("identity validation tests require PostgreSQL");
    let actor = format!("identity-boundary-{}", Uuid::new_v4());
    let valid_arma = format!("valid-arma-{}", Uuid::new_v4());
    common::seed_user(&pool, &actor, "Validation player", &valid_arma, "enlisted").await;
    let invalid_identities = [
        "a".repeat(129),
        format!(" \t{}\n ", "b".repeat(129)),
        "é".repeat(65),
    ];
    for invalid in invalid_identities {
        assert!(
            invalid.trim().len() > 128,
            "limit counts normalized UTF-8 bytes"
        );
        let source = format!("rejected-match-{}", Uuid::new_v4());
        let valid_event = format!("valid-first-{}", Uuid::new_v4());
        let body = json!({
            "match": { "source_match_id": source, "outcome": "success" },
            "players": [
                {
                    "arma_id": valid_arma,
                    "role_played": "rifleman",
                    "source_event_id": valid_event,
                    "kills": 19,
                    "deaths": 1
                },
                {
                    "arma_id": invalid,
                    "role_played": "rifleman",
                    "source_event_id": format!("invalid-second-{}", Uuid::new_v4()),
                    "kills": 1
                }
            ]
        });
        let (status, response) = post(&app, &body).await;
        assert_eq!(
            status,
            StatusCode::BAD_REQUEST,
            "oversized normalized identity: {response}"
        );
        assert!(response["error"].as_str().unwrap().contains("128"));
        let writes: (i64, i64, i64) = sqlx::query_as(
            "SELECT
             (SELECT count(*) FROM matches WHERE source_match_id = $1),
             (SELECT count(*) FROM match_player_stats WHERE source_event_id = $2),
             (SELECT count(*) FROM audit_logs WHERE target_id IN ($1, $3))",
        )
        .bind(&source)
        .bind(&valid_event)
        .bind(&actor)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            writes,
            (0, 0, 0),
            "no match, earlier valid player, or audit may commit"
        );
        let stored: (String, i64) =
            sqlx::query_as("SELECT arma_id, total_deployments FROM users WHERE discord_id = $1")
                .bind(&actor)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(stored, (valid_arma.clone(), 0));
    }
}
