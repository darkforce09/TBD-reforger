//! Report bodies, account sessions, identity links and read-backs shared by the telemetry
//! suites that assert derived statistics, event pages and live status.
//!
//! * [`boot_with_state`] — the router of [`super::boot`] plus the [`AppState`] behind it, so a
//!   suite can issue a persisted session for an account it seeded ([`account_token`]) or drive
//!   the realtime hub directly.
//! * [`counters`], [`line`], [`report`] — the `match-results` body pieces of
//!   `match-telemetry.schema.json`.
//! * [`link_identity`] — the production link flow: the account asks for a code and a game
//!   server confirms it with its machine credential.
//! * [`first_stream_status`] — the first `data:` frame of a server-status event stream.
//! * [`event`] — one detailed event of a `match-events` batch.
//! * [`match_state`], [`player_rows`], [`leaderboard_kills`], [`event_totals`],
//!   [`stored_events`], [`served_statistics`] — the persisted rows and served statistics every
//!   assertion compares with the response.

use std::time::Duration;

use api_configuration::configuration::Config;
use api_state::AppState;

use api_server::router::router;
use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use futures::StreamExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use super::call;
use super::match_reports::ReportingServer;
use crate::common;

/// Router, pool and the state behind the router, over this binary's private database.
///
/// Panics when `TEST_DATABASE_URL` is unset: a telemetry suite without its database fails.
pub(crate) async fn boot_with_state() -> (Router, PgPool, AppState) {
    let url = common::require_test_database_url()
        .expect("TEST_DATABASE_URL is required for the telemetry suites");
    let pool = api_database::connect(&url).await.expect("connect");
    api_database::migrate(&pool).await.expect("migrate");
    let state = api_server::composition::application_state(
        pool.clone(),
        Config::for_tests(url, "tele-secret"),
    );
    (router(state.clone()), pool, state)
}

/// Seed an account holding `arma_id` (or none) and issue a persisted session for it.
pub(crate) async fn account_token(
    state: &AppState,
    discord_id: &str,
    username: &str,
    arma_id: Option<&str>,
) -> String {
    match arma_id {
        Some(arma) => common::seed_user(&state.pool, discord_id, username, arma, "enlisted").await,
        None => {
            sqlx::query(
                "INSERT INTO users (discord_id, username, discord_handle, avatar_url, arma_id, \
                 arma_character, role, is_banned, ban_reason, created_at, updated_at) \
                 VALUES ($1, $2, $2, '', NULL, '', 'enlisted', false, '', now(), now()) \
                 ON CONFLICT (discord_id) DO UPDATE SET arma_id = NULL, username = EXCLUDED.username",
            )
            .bind(discord_id)
            .bind(username)
            .execute(&state.pool)
            .await
            .expect("seed unlinked account");
        }
    }
    common::access_token(state, "telemetry", discord_id, "enlisted", false).await
}

/// A fresh 18-digit Discord id no other test uses.
pub(crate) fn unique_discord_id() -> String {
    let digits: String = Uuid::new_v4()
        .as_u128()
        .to_string()
        .chars()
        .take(17)
        .collect();
    format!("9{digits}")
}

/// An account this test owns outright: `(discord_id, username, arma_id)`, every value unique.
pub(crate) async fn seed_player(pool: &PgPool, prefix: &str) -> (String, String, String) {
    let discord_id = unique_discord_id();
    let username = common::unique_arma(prefix);
    let arma_id = common::unique_arma(&format!("{prefix}-arma"));
    common::seed_user(pool, &discord_id, &username, &arma_id, "enlisted").await;
    (discord_id, username, arma_id)
}

/// A complete `counters` object with the given kills and deaths.
pub(crate) fn counters(kills: i64, deaths: i64) -> Value {
    json!({
        "kills": kills, "deaths": deaths, "team_kills": 0, "longest_kill_m": 0,
        "vehicles_destroyed": 0, "is_command": false,
    })
}

/// One player line; `counters` absent makes no counter claim.
pub(crate) fn line(arma_id: &str, source_event_id: &str, counters: Option<Value>) -> Value {
    let mut line = json!({
        "arma_id": arma_id, "role_played": "Rifleman", "source_event_id": source_event_id,
    });
    if let Some(counters) = counters {
        line["counters"] = counters;
    }
    line
}

/// A `match-results` body without its `revision`.
pub(crate) fn report(source: &str, outcome: &str, players: Vec<Value>) -> Value {
    json!({ "match": { "source_match_id": source, "outcome": outcome }, "players": players })
}

/// Link `arma_id` to the account behind `bearer` through `POST /api/v1/me/link` and the
/// machine-authenticated `POST /api/v1/ingest/link-confirm` of `server`.
pub(crate) async fn link_identity(
    app: &Router,
    server: &ReportingServer,
    bearer: &str,
    arma_id: &str,
) -> (StatusCode, Value) {
    let (status, issued) = call(app, "POST", "/api/v1/me/link", Some(bearer), None, None).await;
    assert_eq!(status, StatusCode::CREATED, "issue link code: {issued}");
    let code = issued["code"].as_str().expect("a link code").to_owned();
    server
        .post(
            app,
            "/api/v1/ingest/link-confirm",
            &json!({ "code": code, "arma_id": arma_id, "arma_character": "Linked Character" }),
        )
        .await
}

/// Open `uri` as an event stream and answer its status with the first `data:` frame, if one
/// arrives within five seconds.
pub(crate) async fn first_stream_status(
    app: &Router,
    uri: &str,
    bearer: &str,
) -> (StatusCode, Option<Value>) {
    let request = Request::builder()
        .uri(uri)
        .header(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {bearer}"),
        )
        .body(Body::empty())
        .expect("the request builds");
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    if status != StatusCode::OK {
        return (status, None);
    }
    let mut stream = response.into_body().into_data_stream();
    let mut buffered = String::new();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while let Ok(Some(Ok(chunk))) = tokio::time::timeout_at(deadline, stream.next()).await {
        buffered.push_str(&String::from_utf8_lossy(&chunk));
        if let Some(frame) = buffered
            .lines()
            .find_map(|text| text.strip_prefix("data:"))
            .and_then(|data| serde_json::from_str(data.trim()).ok())
        {
            return (status, Some(frame));
        }
    }
    (status, None)
}

/// The stored match row: `(revision, report_sha256, outcome, finalized_at, event_count)`.
pub(crate) type MatchState = (
    i64,
    Option<String>,
    String,
    Option<chrono::DateTime<chrono::Utc>>,
    i64,
);

/// Read the stored state of one match.
pub(crate) async fn match_state(pool: &PgPool, match_id: Uuid) -> MatchState {
    sqlx::query_as(
        "SELECT revision, report_sha256, outcome::text, finalized_at, event_count
         FROM matches WHERE id = $1",
    )
    .bind(match_id)
    .fetch_one(pool)
    .await
    .expect("read match row")
}

/// The stored player lines of one match: `(arma_id, source_event_id, kills, deaths,
/// discord_id)` ordered by `arma_id` then `source_event_id`.
pub(crate) async fn player_rows(
    pool: &PgPool,
    match_id: Uuid,
) -> Vec<(String, String, Option<i64>, Option<i64>, Option<String>)> {
    sqlx::query_as(
        "SELECT arma_id, source_event_id, kills, deaths, discord_id FROM match_player_stats
         WHERE match_id = $1 ORDER BY arma_id, source_event_id",
    )
    .bind(match_id)
    .fetch_all(pool)
    .await
    .expect("read player rows")
}

/// `(leaderboard_totals.kills, users.total_deployments)` of one account; kills is `None` when
/// the account has no leaderboard row.
pub(crate) async fn leaderboard_kills(pool: &PgPool, discord_id: &str) -> (Option<i64>, i64) {
    let kills: Option<i64> =
        sqlx::query_scalar("SELECT kills::int8 FROM leaderboard_totals WHERE discord_id = $1")
            .bind(discord_id)
            .fetch_optional(pool)
            .await
            .expect("read leaderboard row")
            .flatten();
    let deployments: i64 =
        sqlx::query_scalar("SELECT total_deployments::int8 FROM users WHERE discord_id = $1")
            .bind(discord_id)
            .fetch_one(pool)
            .await
            .expect("read account");
    (kills, deployments)
}

/// `(stats.kills, total_operations)` from `GET /api/v1/users/{discord_id}/stats` and the kills
/// of the account's row in `GET /api/v1/leaderboards?category=missions&q=<username>` (`None`
/// when the board does not list it).
pub(crate) async fn served_statistics(
    app: &Router,
    bearer: &str,
    discord_id: &str,
    username: &str,
) -> (i64, i64, Option<i64>) {
    let (status, stats) = call(
        app,
        "GET",
        &format!("/api/v1/users/{discord_id}/stats"),
        Some(bearer),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "user stats: {stats}");
    let (status, board) = call(
        app,
        "GET",
        &format!("/api/v1/leaderboards?category=missions&q={username}&limit=50"),
        Some(bearer),
        None,
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "leaderboard: {board}");
    let listed = board["data"]
        .as_array()
        .expect("leaderboard data")
        .iter()
        .find(|row| row["discord_id"] == discord_id)
        .map(|row| row["kills"].as_i64().expect("kills"));
    (
        stats["stats"]["kills"].as_i64().expect("stats kills"),
        stats["total_operations"]
            .as_i64()
            .expect("total_operations"),
        listed,
    )
}

/// One detailed event; `occurred_at` and `mission_time_ms` follow `sequence`.
pub(crate) fn event(event_id: &str, sequence: i64, kind: &str, payload: Value) -> Value {
    json!({
        "event_id": event_id, "sequence": sequence, "kind": kind,
        "mission_time_ms": sequence * 1000,
        "occurred_at": format!("2026-01-01T00:{:02}:{:02}Z", (sequence / 60) % 60, sequence % 60),
        "payload": payload,
    })
}

/// The stored `match_event_totals` of one match: `(arma_id, kind, participant_role,
/// event_count)` in key order.
pub(crate) async fn event_totals(
    pool: &PgPool,
    match_id: Uuid,
) -> Vec<(String, String, String, i64)> {
    sqlx::query_as(
        "SELECT arma_id, kind, participant_role, event_count FROM match_event_totals
         WHERE match_id = $1 ORDER BY arma_id, kind, participant_role",
    )
    .bind(match_id)
    .fetch_all(pool)
    .await
    .expect("read event totals")
}

/// The stored events of one match: `(event_id, sequence, kind)` in sequence order.
pub(crate) async fn stored_events(pool: &PgPool, match_id: Uuid) -> Vec<(String, i64, String)> {
    sqlx::query_as(
        "SELECT event_id, sequence, kind FROM match_events WHERE match_id = $1 ORDER BY sequence",
    )
    .bind(match_id)
    .fetch_all(pool)
    .await
    .expect("read match events")
}
