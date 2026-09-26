//! The game runtime's outbound telemetry queue reading: an optional heartbeat block that is
//! complete and bounded when present, kept when absent, stored on `server_statuses` with its
//! `reported_at`, and carried by `ServerStatus.telemetry_queue` through the status read, the
//! status stream, the scheduled publisher, Server Intel and the dashboard fleet.
//!
//! Every case holds [`FLEET`] because the dashboard totals sum the whole fleet of this binary's
//! database. Each case checks the HTTP answer and the persisted status row. Requires
//! `TEST_DATABASE_URL`.

mod common;
mod contract_support;
mod telemetry_support;

use axum::Router;
use axum::http::StatusCode;
use serde_json::{Value, json};
use sqlx::PgPool;
use telemetry_support::match_reports::ReportingServer;
use telemetry_support::report_fixtures::{boot_with_state, first_stream_status};
use telemetry_support::{admin_token, call, heartbeat};
use tokio::sync::Mutex;
use uuid::Uuid;
use website_api::match_telemetry::models::generated::match_telemetry::{
    TelemetryQueueReading, TelemetryQueueStatus,
};
use website_api::server_infrastructure::services::status_broadcast::publish_all_server_statuses;

static FLEET: Mutex<()> = Mutex::const_new(());

type StoredQueue = (
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<i64>,
    Option<chrono::DateTime<chrono::Utc>>,
);

fn queue(backlog: i64, capacity: i64, dropped_total: i64, oldest_age_seconds: i64) -> Value {
    json!({
        "backlog": backlog, "capacity": capacity, "dropped_total": dropped_total,
        "oldest_age_seconds": oldest_age_seconds,
    })
}

fn unique(prefix: &str) -> String {
    common::unique_arma(prefix)
}

async fn stored_queue(pool: &PgPool, server: Uuid) -> StoredQueue {
    sqlx::query_as(
        "SELECT telemetry_queue_backlog, telemetry_queue_capacity, telemetry_queue_dropped_total,
                telemetry_queue_oldest_age_seconds, telemetry_queue_reported_at
         FROM server_statuses WHERE server_id = $1",
    )
    .bind(server)
    .fetch_one(pool)
    .await
    .expect("status row")
}

async fn status_read(app: &Router, bearer: &str, server: Uuid) -> Value {
    let (status, body) = call(
        app,
        "GET",
        &format!("/api/v1/servers/{server}/status"),
        Some(bearer),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

/// The served block equals the stored columns, and `reported_at` is the stored instant.
fn assert_block_matches(block: &Value, stored: &StoredQueue) {
    assert_eq!(block["backlog"].as_i64(), stored.0, "{block}");
    assert_eq!(block["capacity"].as_i64(), stored.1, "{block}");
    assert_eq!(block["dropped_total"].as_i64(), stored.2, "{block}");
    assert_eq!(block["oldest_age_seconds"].as_i64(), stored.3, "{block}");
    let served =
        chrono::DateTime::parse_from_rfc3339(block["reported_at"].as_str().expect("reported_at"))
            .expect("RFC 3339 reported_at");
    assert_eq!(
        Some(served.with_timezone(&chrono::Utc)),
        stored.4,
        "{block}"
    );
    contract_support::assert_valid(
        "match-telemetry.schema.json",
        Some("TelemetryQueueStatus"),
        block,
    );
    contract_support::assert_decodes::<TelemetryQueueStatus>("TelemetryQueueStatus", block);
}

#[tokio::test]
async fn telemetry_queue_reading_lands_in_the_status_read_and_the_status_stream() {
    let _fleet = FLEET.lock().await;
    let (app, pool, _state) = boot_with_state().await;
    let admin = admin_token(&app).await;
    let server = ReportingServer::open(&app, &pool, &unique("queue-read")).await;
    let silent = ReportingServer::open(&app, &pool, &unique("queue-silent")).await;

    let (status, body) = heartbeat(
        &app,
        &server.session,
        1,
        json!({ "is_online": true, "player_count": 5, "telemetry_queue": queue(7, 512, 2, 30) }),
    )
    .await;
    assert!(status.is_success(), "{status} {body}");
    let stored = stored_queue(&pool, server.server_id).await;
    assert_eq!(
        (stored.0, stored.1, stored.2, stored.3),
        (Some(7), Some(512), Some(2), Some(30))
    );
    assert!(
        stored.4.is_some(),
        "the reading is stored with its reported_at"
    );

    let read = status_read(&app, &admin, server.server_id).await;
    assert_block_matches(&read["status"]["telemetry_queue"], &stored);
    let (status, frame) = first_stream_status(
        &app,
        &format!("/api/v1/servers/{}/status/stream", server.server_id),
        &admin,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let frame = frame.expect("the stream opens with the current status");
    assert_block_matches(&frame["telemetry_queue"], &stored);

    let (status, body) = heartbeat(&app, &silent.session, 1, json!({ "is_online": true })).await;
    assert!(status.is_success(), "{status} {body}");
    assert_eq!(
        stored_queue(&pool, silent.server_id).await,
        (None, None, None, None, None)
    );
    let read = status_read(&app, &admin, silent.server_id).await;
    assert!(read["status"].is_object(), "{read}");
    assert!(
        read["status"].get("telemetry_queue").is_none(),
        "never reported, no key: {read}"
    );
}

#[tokio::test]
async fn telemetry_queue_partial_or_malformed_block_is_refused() {
    let _fleet = FLEET.lock().await;
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &unique("queue-partial")).await;
    let (status, body) = heartbeat(
        &app,
        &server.session,
        1,
        json!({ "telemetry_queue": queue(1, 512, 0, 4) }),
    )
    .await;
    assert!(status.is_success(), "{status} {body}");
    let stored = stored_queue(&pool, server.server_id).await;

    let mut unknown = queue(1, 512, 0, 4);
    unknown["oldest_sequence"] = json!(3);
    for (sequence, (label, block)) in [
        (
            "missing oldest_age_seconds",
            json!({ "backlog": 3, "capacity": 512, "dropped_total": 0 }),
        ),
        (
            "missing capacity",
            json!({ "backlog": 3, "dropped_total": 0, "oldest_age_seconds": 1 }),
        ),
        ("negative dropped_total", queue(3, 512, -1, 1)),
        ("negative oldest_age_seconds", queue(3, 512, 0, -5)),
        ("unknown key", unknown),
    ]
    .into_iter()
    .enumerate()
    {
        let (status, body) = heartbeat(
            &app,
            &server.session,
            sequence as i64 + 2,
            json!({ "player_count": 9, "telemetry_queue": block }),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{label}: {body}");
    }
    assert_eq!(
        stored_queue(&pool, server.server_id).await,
        stored,
        "refused readings change nothing"
    );
    let players: i64 =
        sqlx::query_scalar("SELECT player_count FROM server_statuses WHERE server_id = $1")
            .bind(server.server_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_ne!(players, 9, "a refused heartbeat writes none of its fields");
}

#[tokio::test]
async fn telemetry_queue_absent_block_keeps_the_previous_reading() {
    let _fleet = FLEET.lock().await;
    let (app, pool, _state) = boot_with_state().await;
    let admin = admin_token(&app).await;
    let server = ReportingServer::open(&app, &pool, &unique("queue-keep")).await;
    let (status, body) = heartbeat(
        &app,
        &server.session,
        1,
        json!({ "telemetry_queue": queue(12, 512, 3, 90) }),
    )
    .await;
    assert!(status.is_success(), "{status} {body}");
    let stored = stored_queue(&pool, server.server_id).await;

    let (status, body) = heartbeat(
        &app,
        &server.session,
        2,
        json!({ "is_online": true, "player_count": 11 }),
    )
    .await;
    assert!(status.is_success(), "{status} {body}");
    assert_eq!(
        stored_queue(&pool, server.server_id).await,
        stored,
        "reading and reported_at are kept"
    );
    let read = status_read(&app, &admin, server.server_id).await;
    assert_eq!(read["status"]["player_count"], 11);
    assert_block_matches(&read["status"]["telemetry_queue"], &stored);

    let (status, body) = heartbeat(
        &app,
        &server.session,
        3,
        json!({ "telemetry_queue": queue(0, 512, 3, 0) }),
    )
    .await;
    assert!(status.is_success(), "{status} {body}");
    let replaced = stored_queue(&pool, server.server_id).await;
    assert_eq!(
        (replaced.0, replaced.3),
        (Some(0), Some(0)),
        "a present block replaces the reading"
    );
    assert!(replaced.4 >= stored.4);
}

#[tokio::test]
async fn telemetry_queue_backlog_above_capacity_is_refused() {
    let _fleet = FLEET.lock().await;
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &unique("queue-bound")).await;
    let (status, body) = heartbeat(
        &app,
        &server.session,
        1,
        json!({ "telemetry_queue": queue(512, 512, 0, 60) }),
    )
    .await;
    assert!(
        status.is_success(),
        "a full queue is within bounds: {status} {body}"
    );
    let stored = stored_queue(&pool, server.server_id).await;
    assert_eq!((stored.0, stored.1), (Some(512), Some(512)));

    let (status, body) = heartbeat(
        &app,
        &server.session,
        2,
        json!({ "telemetry_queue": queue(513, 512, 0, 60) }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(stored_queue(&pool, server.server_id).await, stored);
}

#[tokio::test]
async fn telemetry_queue_reading_appears_in_the_dashboard_fleet_and_server_intel() {
    let _fleet = FLEET.lock().await;
    let (app, pool, state) = boot_with_state().await;
    let admin = admin_token(&app).await;
    let server = ReportingServer::open(&app, &pool, &unique("queue-fleet")).await;
    let (status, body) = heartbeat(
        &app,
        &server.session,
        1,
        json!({ "is_online": true, "player_count": 3, "max_players": 40, "telemetry_queue": queue(21, 512, 6, 45) }),
    )
    .await;
    assert!(status.is_success(), "{status} {body}");
    let stored = stored_queue(&pool, server.server_id).await;

    let (status, dashboard) =
        call(&app, "GET", "/api/v1/dashboard", Some(&admin), None, None).await;
    assert_eq!(status, StatusCode::OK, "{dashboard}");
    let entry = dashboard["fleet"]["servers"]
        .as_array()
        .expect("fleet servers")
        .iter()
        .find(|entry| entry["server_id"] == json!(server.server_id))
        .expect("the reporting server is in the fleet")
        .clone();
    assert_block_matches(&entry["status"]["telemetry_queue"], &stored);
    let (backlog, dropped): (i64, i64) = sqlx::query_as(
        "SELECT COALESCE(sum(s.telemetry_queue_backlog), 0)::int8,
                COALESCE(sum(s.telemetry_queue_dropped_total), 0)::int8
         FROM server_statuses s JOIN servers ON servers.id = s.server_id WHERE servers.is_active",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(backlog >= 21 && dropped >= 6);
    assert_eq!(dashboard["fleet"]["totals"]["telemetry_backlog"], backlog);
    assert_eq!(
        dashboard["fleet"]["totals"]["telemetry_dropped_total"],
        dropped
    );

    let (status, intel) = call(&app, "GET", "/api/v1/servers", Some(&admin), None, None).await;
    assert_eq!(status, StatusCode::OK, "{intel}");
    let card = intel["data"]
        .as_array()
        .expect("server intel list")
        .iter()
        .find(|card| card["id"] == json!(server.server_id))
        .expect("the reporting server is listed")
        .clone();
    assert_block_matches(&card["status"]["telemetry_queue"], &stored);

    let mut published = state.hub.subscribe(&format!("server:{}", server.server_id));
    publish_all_server_statuses(&pool, &state.hub)
        .await
        .expect("publish the fleet");
    let payload: Value = serde_json::from_slice(&published.try_recv().expect("a published status"))
        .expect("status JSON");
    assert_block_matches(&payload["telemetry_queue"], &stored);
}

#[tokio::test]
async fn telemetry_queue_heartbeat_with_the_block_follows_the_contract() {
    let _fleet = FLEET.lock().await;
    let (app, pool, _state) = boot_with_state().await;
    let server = ReportingServer::open(&app, &pool, &unique("queue-contract")).await;
    let block = queue(4, 512, 1, 12);
    let body = json!({
        "generation": server.session.generation, "sequence": 1, "is_online": true,
        "player_count": 2, "max_players": 64, "server_fps": 59.5, "uptime_seconds": 600,
        "ingame_time": "12:00", "ingame_weather": "clear", "telemetry_queue": block,
    });
    let schema = "game-runtime-session.schema.json";
    contract_support::assert_valid(schema, Some("RuntimeHeartbeat"), &body);
    contract_support::assert_valid(
        "match-telemetry.schema.json",
        Some("TelemetryQueueReading"),
        &block,
    );
    contract_support::assert_decodes::<TelemetryQueueReading>("TelemetryQueueReading", &block);
    let mut partial = body.clone();
    partial["telemetry_queue"] = json!({ "backlog": 4, "capacity": 512 });
    contract_support::assert_invalid(schema, Some("RuntimeHeartbeat"), &partial);

    let (status, answer) = call(
        &app,
        "POST",
        &format!(
            "/api/v1/game-runtime/sessions/{}/heartbeats",
            server.session.id
        ),
        Some(&server.session.secret),
        None,
        Some(&body.to_string()),
    )
    .await;
    assert!(status.is_success(), "{status} {answer}");
    let stored = stored_queue(&pool, server.server_id).await;
    assert_eq!(
        (stored.0, stored.1, stored.2, stored.3),
        (Some(4), Some(512), Some(1), Some(12))
    );
}
