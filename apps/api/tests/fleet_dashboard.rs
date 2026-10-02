//! The configured fleet is the set of active servers: the dashboard lists them by name with
//! their statuses and totals, the scheduled publisher republishes only them, and the server
//! list, status read and status stream show inactive servers to administrators alone.
//!
//! Every case holds [`FLEET`] because each asserts fleet-wide state of this binary's database,
//! and compares the served fleet with the `servers` and `server_statuses` rows read right after.
//! Requires `TEST_DATABASE_URL`.

mod common;
mod telemetry_support;

use api::core::realtime_hub::Hub;
use api::server_infrastructure::services::status_broadcast::publish_all_server_statuses;
use axum::Router;
use axum::http::StatusCode;
use serde_json::{Value, json};
use sqlx::PgPool;
use telemetry_support::match_reports::ReportingServer;
use telemetry_support::report_fixtures::{boot_with_state, first_stream_status};
use telemetry_support::{admin_token, call, heartbeat};
use tokio::sync::Mutex;
use tokio::sync::broadcast::error::TryRecvError;
use uuid::Uuid;

static FLEET: Mutex<()> = Mutex::const_new(());

fn unique(prefix: &str) -> String {
    common::unique_arma(prefix)
}

async fn member_token(app: &Router) -> String {
    common::dev_login_token(app, "fleet_dashboard", "enlisted").await
}

/// Register an active server with no runtime session and no status row.
async fn silent_server(pool: &PgPool, name: &str) -> Uuid {
    sqlx::query_scalar(
        "INSERT INTO servers (name, ip, port, is_active) VALUES ($1, '127.0.0.1', 2001, true) RETURNING id",
    )
    .bind(name)
    .fetch_one(pool)
    .await
    .unwrap()
}

/// An online server with a status row, deactivated when `active` is false.
async fn reporting_server(
    app: &Router,
    pool: &PgPool,
    name: &str,
    players: i64,
    backlog: i64,
    active: bool,
) -> Uuid {
    let server = ReportingServer::open(app, pool, name).await;
    let (status, body) = heartbeat(
        app,
        &server.session,
        1,
        json!({
            "is_online": true, "player_count": players, "max_players": 40,
            "telemetry_queue": { "backlog": backlog, "capacity": 512, "dropped_total": 1, "oldest_age_seconds": 5 },
        }),
    )
    .await;
    assert!(status.is_success(), "{status} {body}");
    if !active {
        sqlx::query("UPDATE servers SET is_active = false WHERE id = $1")
            .bind(server.server_id)
            .execute(pool)
            .await
            .unwrap();
    }
    server.server_id
}

async fn dashboard_fleet(app: &Router, bearer: &str) -> Value {
    let (status, body) = call(app, "GET", "/api/v1/dashboard", Some(bearer), None, None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["fleet"].clone()
}

fn listed(fleet: &Value) -> Vec<(Uuid, String)> {
    fleet["servers"]
        .as_array()
        .expect("fleet servers")
        .iter()
        .map(|entry| {
            (
                entry["server_id"].as_str().unwrap().parse().unwrap(),
                entry["name"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

async fn active_servers(pool: &PgPool) -> Vec<(Uuid, String)> {
    sqlx::query_as("SELECT id, name FROM servers WHERE is_active ORDER BY name, id")
        .fetch_all(pool)
        .await
        .unwrap()
}

/// The fleet totals recomputed from the rows: a server without a status row counts offline.
async fn stored_totals(pool: &PgPool) -> Value {
    let row: (i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT count(*)::int8,
                count(*) FILTER (WHERE s.is_online)::int8,
                COALESCE(sum(s.player_count) FILTER (WHERE s.is_online), 0)::int8,
                COALESCE(sum(s.max_players) FILTER (WHERE s.is_online), 0)::int8,
                COALESCE(sum(s.telemetry_queue_backlog), 0)::int8,
                COALESCE(sum(s.telemetry_queue_dropped_total), 0)::int8
         FROM servers LEFT JOIN server_statuses s ON s.server_id = servers.id
         WHERE servers.is_active",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    json!({
        "configured": row.0, "online": row.1, "players": row.2, "max_players": row.3,
        "telemetry_backlog": row.4, "telemetry_dropped_total": row.5,
    })
}

#[tokio::test]
async fn fleet_dashboard_lists_every_active_server_ordered_by_name() {
    let _fleet = FLEET.lock().await;
    let (app, pool, _state) = boot_with_state().await;
    let admin = admin_token(&app).await;
    let stem = unique("fleet-order");
    let charlie = reporting_server(&app, &pool, &format!("{stem}-c"), 2, 0, true).await;
    let alpha = silent_server(&pool, &format!("{stem}-a")).await;
    let bravo = reporting_server(&app, &pool, &format!("{stem}-b"), 4, 3, true).await;
    let twin_one = silent_server(&pool, &format!("{stem}-d")).await;
    let twin_two = silent_server(&pool, &format!("{stem}-d")).await;

    let fleet = dashboard_fleet(&app, &admin).await;
    let served = listed(&fleet);
    assert_eq!(
        served,
        active_servers(&pool).await,
        "every active server, by name then id"
    );
    let mine: Vec<Uuid> = served
        .iter()
        .map(|entry| entry.0)
        .filter(|id| [alpha, bravo, charlie, twin_one, twin_two].contains(id))
        .collect();
    let mut twins = [twin_one, twin_two];
    twins.sort();
    assert_eq!(mine, [alpha, bravo, charlie, twins[0], twins[1]]);
    assert_eq!(fleet["totals"], stored_totals(&pool).await);
    assert!(fleet.get("server_status").is_none());
}

#[tokio::test]
async fn fleet_dashboard_inactive_servers_are_excluded_from_the_list_and_totals() {
    let _fleet = FLEET.lock().await;
    let (app, pool, _state) = boot_with_state().await;
    let admin = admin_token(&app).await;
    let active = reporting_server(&app, &pool, &unique("fleet-active"), 6, 2, true).await;
    let before = dashboard_fleet(&app, &admin).await;
    let inactive = reporting_server(&app, &pool, &unique("fleet-inactive"), 30, 9, false).await;

    let fleet = dashboard_fleet(&app, &admin).await;
    let ids: Vec<Uuid> = listed(&fleet).into_iter().map(|entry| entry.0).collect();
    assert!(ids.contains(&active));
    assert!(
        !ids.contains(&inactive),
        "an inactive server is not in the fleet"
    );
    assert_eq!(
        fleet["totals"], before["totals"],
        "its players and backlog are not counted"
    );
    assert_eq!(fleet["totals"], stored_totals(&pool).await);
    let stored: (bool, i64) = sqlx::query_as(
        "SELECT servers.is_active, s.player_count FROM servers JOIN server_statuses s ON s.server_id = servers.id WHERE servers.id = $1",
    )
    .bind(inactive)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        stored,
        (false, 30),
        "the inactive server keeps its status row"
    );
}

#[tokio::test]
async fn fleet_dashboard_server_without_a_status_is_listed_without_status_and_counted_offline() {
    let _fleet = FLEET.lock().await;
    let (app, pool, _state) = boot_with_state().await;
    let admin = admin_token(&app).await;
    let before = dashboard_fleet(&app, &admin).await;
    let silent = silent_server(&pool, &unique("fleet-silent")).await;

    let fleet = dashboard_fleet(&app, &admin).await;
    let entry = fleet["servers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["server_id"] == json!(silent))
        .expect("a server without status is listed")
        .clone();
    assert!(entry.get("status").is_none(), "no status key: {entry}");
    let rows: i64 = sqlx::query_scalar("SELECT count(*) FROM server_statuses WHERE server_id = $1")
        .bind(silent)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(rows, 0);
    let totals = &fleet["totals"];
    assert_eq!(
        totals["configured"],
        before["totals"]["configured"].as_i64().unwrap() + 1
    );
    assert_eq!(
        totals["online"], before["totals"]["online"],
        "it counts offline"
    );
    assert_eq!(*totals, stored_totals(&pool).await);
}

#[tokio::test]
async fn fleet_dashboard_scheduled_publisher_skips_inactive_servers() {
    let _fleet = FLEET.lock().await;
    let (app, pool, _state) = boot_with_state().await;
    let active = reporting_server(&app, &pool, &unique("fleet-publish-active"), 1, 0, true).await;
    let inactive =
        reporting_server(&app, &pool, &unique("fleet-publish-inactive"), 1, 0, false).await;
    let hub = Hub::new();
    let mut active_topic = hub.subscribe(&format!("server:{active}"));
    let mut inactive_topic = hub.subscribe(&format!("server:{inactive}"));

    let published = publish_all_server_statuses(&pool, &hub)
        .await
        .expect("publish");
    let expected: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM server_statuses s JOIN servers ON servers.id = s.server_id WHERE servers.is_active",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        published as i64, expected,
        "one publish per active status row"
    );
    let payload: Value = serde_json::from_slice(
        &active_topic
            .try_recv()
            .expect("the active server is published"),
    )
    .unwrap();
    assert_eq!(payload["server_id"], json!(active));
    assert!(
        matches!(inactive_topic.try_recv(), Err(TryRecvError::Empty)),
        "the inactive server is not republished"
    );
}

#[tokio::test]
async fn fleet_dashboard_server_list_hides_inactive_servers_from_members() {
    let _fleet = FLEET.lock().await;
    let (app, pool, _state) = boot_with_state().await;
    let (admin, member) = (admin_token(&app).await, member_token(&app).await);
    let active = reporting_server(&app, &pool, &unique("fleet-list-active"), 2, 0, true).await;
    let inactive = reporting_server(&app, &pool, &unique("fleet-list-inactive"), 2, 0, false).await;
    let ids_and_flags = |body: &Value| -> Vec<(Uuid, bool)> {
        body["data"]
            .as_array()
            .expect("server list")
            .iter()
            .map(|card| {
                (
                    card["id"].as_str().unwrap().parse().unwrap(),
                    card["is_active"].as_bool().expect("is_active"),
                )
            })
            .collect()
    };

    let (status, body) = call(&app, "GET", "/api/v1/servers", Some(&member), None, None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let expected: Vec<(Uuid, bool)> =
        sqlx::query_as("SELECT id, is_active FROM servers WHERE is_active ORDER BY name, id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        ids_and_flags(&body),
        expected,
        "a member sees the active servers only"
    );
    assert!(expected.contains(&(active, true)));

    let (status, body) = call(&app, "GET", "/api/v1/servers", Some(&admin), None, None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let expected: Vec<(Uuid, bool)> =
        sqlx::query_as("SELECT id, is_active FROM servers ORDER BY name, id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(
        ids_and_flags(&body),
        expected,
        "an administrator sees every server"
    );
    assert!(expected.contains(&(inactive, false)));
}

#[tokio::test]
async fn fleet_dashboard_inactive_status_read_and_stream_are_for_administrators_only() {
    let _fleet = FLEET.lock().await;
    let (app, pool, _state) = boot_with_state().await;
    let (admin, member) = (admin_token(&app).await, member_token(&app).await);
    let active = reporting_server(&app, &pool, &unique("fleet-read-active"), 3, 0, true).await;
    let inactive = reporting_server(&app, &pool, &unique("fleet-read-inactive"), 3, 0, false).await;

    let read = |server: Uuid, bearer: String| {
        let app = app.clone();
        async move {
            call(
                &app,
                "GET",
                &format!("/api/v1/servers/{server}/status"),
                Some(&bearer),
                None,
                None,
            )
            .await
        }
    };
    let stream = |server: Uuid| format!("/api/v1/servers/{server}/status/stream");

    let (status, body) = read(inactive, member.clone()).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    let (status, _) = first_stream_status(&app, &stream(inactive), &member).await;
    assert_eq!(status, StatusCode::NOT_FOUND);

    let (status, body) = read(inactive, admin.clone()).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        (body["id"].clone(), body["is_active"].clone()),
        (json!(inactive), json!(false))
    );
    assert_eq!(body["status"]["player_count"], 3);
    let (status, frame) = first_stream_status(&app, &stream(inactive), &admin).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        frame.expect("the stream opens with the status")["server_id"],
        json!(inactive)
    );

    let (status, body) = read(active, member.clone()).await;
    assert_eq!(
        status,
        StatusCode::OK,
        "an active server is readable by a member: {body}"
    );
    let (status, frame) = first_stream_status(&app, &stream(active), &member).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(frame.expect("status frame")["server_id"], json!(active));
    let stored: bool = sqlx::query_scalar("SELECT is_active FROM servers WHERE id = $1")
        .bind(inactive)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(!stored);
}
