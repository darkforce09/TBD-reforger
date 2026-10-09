//! The game runtime's outbound telemetry queue reading: an optional heartbeat block that is stored
//! on `server_statuses` and carried by `ServerStatus.telemetry_queue` through the status read and
//! the status stream, and refused when its backlog exceeds its capacity.
//!
//! Each case checks the HTTP answer and the persisted status row.

use crate::{common, contract_support, telemetry_support};

use axum::Router;
use axum::http::StatusCode;
use contract_schema_types::match_telemetry::match_telemetry::TelemetryQueueStatus;
use serde_json::{Value, json};
use sqlx::PgPool;
use telemetry_support::match_reports::ReportingServer;
use telemetry_support::report_fixtures::{boot_with_state, first_stream_status};
use telemetry_support::{admin_token, call, heartbeat};
use tokio::sync::Mutex;
use uuid::Uuid;

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
