//! Game-server telemetry ingest — Rust port of `handlers/telemetry.go`. Service-token
//! authenticated. Feeds the SSE hub (server-status) and the leaderboard MV (match results).

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::response::Json;
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use crate::db::refresh_leaderboard;
use crate::error::ApiError;
use crate::middleware::ServiceAuth;
use crate::models::{AuditSeverity, MissionOutcome, ServerStatus, TerrainType};
use crate::services::write_audit;
use crate::state::AppState;

const LOW_FPS_THRESHOLD: f64 = 20.0;

fn valid_terrain(s: &str) -> Option<TerrainType> {
    match s {
        "everon" => Some(TerrainType::Everon),
        "arland" => Some(TerrainType::Arland),
        "custom" => Some(TerrainType::Custom),
        _ => None,
    }
}

fn parse_uuid_opt(s: &Option<String>) -> Option<Uuid> {
    s.as_deref()
        .filter(|v| !v.is_empty())
        .and_then(|v| Uuid::parse_str(v).ok())
}

// --- server status ---

#[derive(Debug, Deserialize)]
pub struct ServerStatusInput {
    #[serde(default)]
    server_id: String,
    #[serde(default)]
    is_online: bool,
    #[serde(default)]
    player_count: i64,
    #[serde(default)]
    max_players: i64,
    #[serde(default)]
    server_fps: f64,
    #[serde(default)]
    uptime_seconds: i64,
    current_match_id: Option<String>,
    #[serde(default)]
    ingame_time: String,
    #[serde(default)]
    ingame_weather: String,
}

/// `POST /api/v1/ingest/server-status` — upsert live status, append history, WARN on
/// low-FPS edge, fan out to SSE (service-token).
///
/// @route POST /api/v1/ingest/server-status
pub async fn ingest_server_status(
    State(state): State<AppState>,
    _svc: ServiceAuth,
    body: Result<Json<ServerStatusInput>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let Json(input) = body.map_err(|_| ApiError::bad_request("server_id required"))?;
    if input.server_id.is_empty() {
        return Err(ApiError::bad_request("server_id required"));
    }
    let Ok(server_id) = Uuid::parse_str(&input.server_id) else {
        return Err(ApiError::bad_request("invalid server_id"));
    };

    // Edge-trigger the low-FPS warning: only when crossing below the threshold.
    let prev_fps: Option<f64> =
        sqlx::query_scalar("SELECT server_fps::float8 FROM server_statuses WHERE server_id = $1")
            .bind(server_id)
            .fetch_optional(&state.pool)
            .await?;
    let prev_healthy = prev_fps.map(|f| f >= LOW_FPS_THRESHOLD).unwrap_or(true);

    let match_id = parse_uuid_opt(&input.current_match_id);
    let now = Utc::now();

    sqlx::query(
        "INSERT INTO server_statuses \
         (server_id, is_online, player_count, max_players, server_fps, uptime_seconds, \
          current_match_id, ingame_time, ingame_weather, updated_at) \
         VALUES ($1, $2, $3, $4, $5::float8::numeric, $6, $7, $8, $9, $10) \
         ON CONFLICT (server_id) DO UPDATE SET \
          is_online = EXCLUDED.is_online, player_count = EXCLUDED.player_count, \
          max_players = EXCLUDED.max_players, server_fps = EXCLUDED.server_fps, \
          uptime_seconds = EXCLUDED.uptime_seconds, current_match_id = EXCLUDED.current_match_id, \
          ingame_time = EXCLUDED.ingame_time, ingame_weather = EXCLUDED.ingame_weather, \
          updated_at = EXCLUDED.updated_at",
    )
    .bind(server_id)
    .bind(input.is_online)
    .bind(input.player_count)
    .bind(input.max_players)
    .bind(input.server_fps)
    .bind(input.uptime_seconds)
    .bind(match_id)
    .bind(&input.ingame_time)
    .bind(&input.ingame_weather)
    .bind(now)
    .execute(&state.pool)
    .await?;

    // Time-series sample.
    sqlx::query(
        "INSERT INTO server_status_histories (server_id, player_count, server_fps) \
         VALUES ($1, $2, $3::float8::numeric)",
    )
    .bind(server_id)
    .bind(input.player_count)
    .bind(input.server_fps)
    .execute(&state.pool)
    .await?;

    if prev_healthy && input.server_fps < LOW_FPS_THRESHOLD && input.is_online {
        write_audit(
            &state.pool,
            AuditSeverity::Warn,
            None,
            "system",
            "server.low_fps",
            &format!(
                "Active server FPS dropped below 20 (now {:.1})",
                input.server_fps
            ),
            "server",
            &server_id.to_string(),
        )
        .await;
    }

    // Fan out to SSE subscribers (the exact struct Go marshals).
    let status = ServerStatus {
        server_id,
        is_online: input.is_online,
        player_count: input.player_count,
        max_players: input.max_players,
        server_fps: input.server_fps,
        uptime_seconds: input.uptime_seconds,
        current_match_id: match_id,
        ingame_time: input.ingame_time,
        ingame_weather: input.ingame_weather,
        updated_at: now,
    };
    if let Ok(payload) = serde_json::to_vec(&status) {
        state.hub.publish(&format!("server:{server_id}"), payload);
    }

    Ok(Json(json!({ "ok": true })))
}

// --- match results ---

#[derive(Debug, Deserialize, Default)]
pub struct MatchInput {
    source_match_id: Option<String>,
    event_id: Option<String>,
    mission_id: Option<String>,
    #[serde(default)]
    terrain: String,
    started_at: Option<DateTime<Utc>>,
    ended_at: Option<DateTime<Utc>>,
    #[serde(default)]
    outcome: String,
    #[serde(default)]
    winning_faction: String,
    #[serde(default)]
    aar_replay_url: String,
}

#[derive(Debug, Deserialize)]
pub struct PlayerStatInput {
    #[serde(default)]
    arma_id: String,
    #[serde(default)]
    role_played: String,
    #[serde(default)]
    kills: i64,
    #[serde(default)]
    deaths: i64,
    #[serde(default)]
    team_kills: i64,
    #[serde(default)]
    longest_kill_m: i64,
    #[serde(default)]
    vehicles_destroyed: i64,
    #[serde(default)]
    is_command: bool,
    command_win: Option<bool>,
    #[serde(default)]
    source_event_id: String,
}

#[derive(Debug, Deserialize)]
pub struct MatchResultsInput {
    #[serde(rename = "match", default)]
    match_data: MatchInput,
    #[serde(default)]
    players: Vec<PlayerStatInput>,
}

/// `POST /api/v1/ingest/match-results` — idempotent match + per-player stats,
/// attendance marking, user-stat recompute, leaderboard refresh (service-token).
///
/// @route POST /api/v1/ingest/match-results
pub async fn ingest_match_results(
    State(state): State<AppState>,
    _svc: ServiceAuth,
    body: Result<Json<MatchResultsInput>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let Json(input) = body.map_err(|_| ApiError::bad_request("match and players are required"))?;
    let m = input.match_data;

    let outcome = match m.outcome.as_str() {
        "" => MissionOutcome::Pending,
        "success" => MissionOutcome::Success,
        "failure" => MissionOutcome::Failure,
        "aborted" => MissionOutcome::Aborted,
        "pending" => MissionOutcome::Pending,
        _ => return Err(ApiError::bad_request("invalid outcome")),
    };

    let mut tx = state.pool.begin().await?;
    let (match_id, event_id) = upsert_match(&mut tx, &m, outcome).await?;

    let mut resolved: Vec<String> = Vec::new();
    for p in &input.players {
        let discord_id: Option<String> = sqlx::query_scalar(
            "SELECT discord_id FROM users WHERE arma_id = $1 AND deleted_at IS NULL",
        )
        .bind(&p.arma_id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(did) = &discord_id
            && !resolved.contains(did)
        {
            resolved.push(did.clone());
        }
        sqlx::query(
            "INSERT INTO match_player_stats \
             (match_id, arma_id, discord_id, role_played, kills, deaths, team_kills, \
              longest_kill_m, vehicles_destroyed, is_command, command_win, source_event_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) \
             ON CONFLICT (match_id, arma_id, source_event_id) DO UPDATE SET \
              discord_id = EXCLUDED.discord_id, role_played = EXCLUDED.role_played, \
              kills = EXCLUDED.kills, deaths = EXCLUDED.deaths, team_kills = EXCLUDED.team_kills, \
              longest_kill_m = EXCLUDED.longest_kill_m, vehicles_destroyed = EXCLUDED.vehicles_destroyed, \
              is_command = EXCLUDED.is_command, command_win = EXCLUDED.command_win",
        )
        .bind(match_id)
        .bind(&p.arma_id)
        .bind(&discord_id)
        .bind(&p.role_played)
        .bind(p.kills)
        .bind(p.deaths)
        .bind(p.team_kills)
        .bind(p.longest_kill_m)
        .bind(p.vehicles_destroyed)
        .bind(p.is_command)
        .bind(p.command_win)
        .bind(&p.source_event_id)
        .execute(&mut *tx)
        .await?;
    }

    // Mark attendance for scheduled operations (resolve via the event's missions).
    if let Some(eid) = event_id
        && !resolved.is_empty()
    {
        sqlx::query(
            "UPDATE event_registrations SET state = 'attended' \
             WHERE event_mission_id IN (SELECT id FROM event_missions WHERE event_id = $1) \
               AND discord_id = ANY($2)",
        )
        .bind(eid)
        .bind(&resolved)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    // Recompute denormalized user stats + refresh the leaderboard view.
    for did in &resolved {
        recompute_user_stats(&state.pool, did).await?;
    }
    if refresh_leaderboard(&state.pool).await.is_err() {
        write_audit(
            &state.pool,
            AuditSeverity::Warn,
            None,
            "system",
            "leaderboard.refresh_failed",
            "Leaderboard refresh failed after match ingest",
            "match",
            &match_id.to_string(),
        )
        .await;
    }

    Ok(Json(
        json!({ "match_id": match_id, "players": input.players.len() }),
    ))
}

/// Find a match by source_match_id (updating mutable fields) or create one. Returns
/// `(id, event_id)`.
async fn upsert_match(
    tx: &mut sqlx::PgConnection,
    m: &MatchInput,
    outcome: MissionOutcome,
) -> Result<(Uuid, Option<Uuid>), ApiError> {
    let started = m.started_at.unwrap_or_else(Utc::now);
    let event_id = parse_uuid_opt(&m.event_id);
    let mission_id = parse_uuid_opt(&m.mission_id);
    let terrain = valid_terrain(&m.terrain);

    if let Some(src) = m.source_match_id.as_deref().filter(|s| !s.is_empty()) {
        let existing: Option<(Uuid, Option<Uuid>)> =
            sqlx::query_as("SELECT id, event_id FROM matches WHERE source_match_id = $1")
                .bind(src)
                .fetch_optional(&mut *tx)
                .await?;
        if let Some((id, ev)) = existing {
            sqlx::query(
                "UPDATE matches SET ended_at = $1, outcome = $2, winning_faction = $3, aar_replay_url = $4 WHERE id = $5",
            )
            .bind(m.ended_at)
            .bind(outcome)
            .bind(&m.winning_faction)
            .bind(&m.aar_replay_url)
            .bind(id)
            .execute(&mut *tx)
            .await?;
            return Ok((id, ev));
        }
    }

    let row: (Uuid, Option<Uuid>) = sqlx::query_as(
        "INSERT INTO matches \
         (source_match_id, event_id, mission_id, terrain, started_at, ended_at, outcome, \
          winning_faction, aar_replay_url, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, now()) RETURNING id, event_id",
    )
    .bind(&m.source_match_id)
    .bind(event_id)
    .bind(mission_id)
    .bind(terrain)
    .bind(started)
    .bind(m.ended_at)
    .bind(outcome)
    .bind(&m.winning_faction)
    .bind(&m.aar_replay_url)
    .fetch_one(&mut *tx)
    .await?;
    Ok(row)
}

/// Refresh a user's denormalized deployment + attendance metrics.
async fn recompute_user_stats(pool: &PgPool, discord_id: &str) -> Result<(), ApiError> {
    let deployments: i64 = sqlx::query_scalar(
        "SELECT count(DISTINCT match_id) FROM match_player_stats WHERE discord_id = $1",
    )
    .bind(discord_id)
    .fetch_one(pool)
    .await?;
    let attended: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM event_registrations WHERE discord_id = $1 AND state::text = 'attended'",
    )
    .bind(discord_id)
    .fetch_one(pool)
    .await?;
    let past_registered: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM event_registrations \
         JOIN event_missions ON event_missions.id = event_registrations.event_mission_id \
         WHERE event_registrations.discord_id = $1 AND event_missions.start_time <= now()",
    )
    .bind(discord_id)
    .fetch_one(pool)
    .await?;
    let rate = if past_registered > 0 {
        attended as f64 / past_registered as f64 * 100.0
    } else {
        0.0
    };
    sqlx::query(
        "UPDATE users SET total_deployments = $1, attendance_rate = $2::float8::numeric WHERE discord_id = $3",
    )
    .bind(deployments)
    .bind(rate)
    .bind(discord_id)
    .execute(pool)
    .await?;
    Ok(())
}
