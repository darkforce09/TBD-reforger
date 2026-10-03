//! `POST /api/v1/game-runtime/sessions/:sessionId/heartbeats` — the game runtime's live-status
//! heartbeat: the session fence, the partial upsert, the time-series sample, the low-FPS edge
//! warning, and the SSE fan-out.
//!
//! The server is the one the `mod_runtime` machine credential belongs to, never a value in the
//! body. Each heartbeat names its runtime session, that session's generation and a sequence that
//! strictly increases within the session; the fence and the status write commit together, so a
//! delayed or stale-runtime heartbeat can never overwrite the state a newer one reported.
//!
//! @contract runtime-heartbeat-receipt.schema.json#/definitions/HeartbeatReceipt

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::response::Json;
use chrono::Utc;
use fleet_wire_contract::ExecutorKind;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::administration::models::audit_log::AuditSeverity;
use crate::administration::services::audit_writer::write_audit;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::http::path_parameters::PathParams;
use crate::server_infrastructure::services::machine_credentials::MachineCaller;
use crate::server_infrastructure::services::runtime_sessions::admit_heartbeat;
use crate::server_infrastructure::services::status_broadcast::publish_server_status_by_id;

use super::server_heartbeat_contract::ServerStatusInput;
use crate::match_telemetry::services::ingest_parsing::{
    coalesce_str, foreign_key_error, parse_uuid_opt,
};

const LOW_FPS_THRESHOLD: f64 = 20.0;

/// The server's live status after a heartbeat is folded in — what actually landed in the
/// row, which is what the SSE subscribers and the history sample have to reflect (a partial
/// heartbeat must not publish its own zeros).
#[derive(Debug, sqlx::FromRow)]
struct EffectiveStatus {
    is_online: bool,
    player_count: i64,
    server_fps: f64,
}

/// Fence the runtime session, upsert live status, append history, WARN on low-FPS edge, fan out
/// to SSE (`mod_runtime` machine credential).
///
/// @route POST /api/v1/game-runtime/sessions/:sessionId/heartbeats
pub async fn ingest_server_status(
    State(state): State<AppState>,
    caller: MachineCaller,
    PathParams(session): PathParams<String>,
    body: Result<Json<ServerStatusInput>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    caller.require_executor(ExecutorKind::ModRuntime)?;
    let session = Uuid::parse_str(&session)
        .map_err(|_| ApiError::bad_request("invalid runtime session id"))?;
    let Json(input) = body.map_err(ApiError::from_json_rejection)?;
    if input.server_id.is_some() {
        return Err(ApiError::bad_request(
            "server_id is not accepted: the machine credential identifies the server",
        ));
    }
    let fence = input.fence(session)?;
    let telemetry_queue = input.telemetry_queue()?;
    // A heartbeat that reports nothing is not a "server is at zero" reading, it is a
    // malformed request — writing eight defaults for it turns it into one.
    if input.is_empty_reading() {
        return Err(ApiError::bad_request(
            "heartbeat must carry at least one reading besides its session fence",
        ));
    }
    let server_id = caller.server_id;
    let mut transaction = state.pool.begin().await?;
    admit_heartbeat(&mut transaction, &caller, fence).await?;

    // Edge-trigger the low-FPS warning: only when crossing below the threshold. Read the
    // pre-update value before the upsert; the session lock orders heartbeats of one server.
    let prev_fps: Option<f64> =
        sqlx::query_scalar("SELECT server_fps::float8 FROM server_statuses WHERE server_id = $1")
            .bind(server_id)
            .fetch_optional(&mut *transaction)
            .await?;
    let prev_healthy = prev_fps.map(|f| f >= LOW_FPS_THRESHOLD).unwrap_or(true);

    // Three-state (see [`ServerStatusInput`]): absent keeps, present sets, `""` clears.
    let set_match_id = input.current_match_id.is_some();
    let match_id = parse_uuid_opt(&input.current_match_id);
    // A server reports only its own registered matches (or ones recorded before registration
    // existed); a match another server registered is refused rather than displayed as this one's.
    if let Some(match_id) = match_id {
        let match_server: Option<Option<Uuid>> =
            sqlx::query_scalar("SELECT server_id FROM matches WHERE id = $1")
                .bind(match_id)
                .fetch_optional(&mut *transaction)
                .await?;
        if let Some(Some(match_server)) = match_server
            && match_server != server_id
        {
            return Err(ApiError::bad_request(
                "current_match_id names a match another server registered",
            ));
        }
    }
    let now = Utc::now();

    // `COALESCE($n, <stored>)` in the DO UPDATE — deliberately reading the bind parameters
    // and not `EXCLUDED`, because `EXCLUDED` holds the row the VALUES clause already
    // defaulted, so it is never NULL and would defeat the whole point. `RETURNING` gives us
    // the merged row so the history sample and the SSE payload report what the server's
    // state actually is, not just the slice of it this heartbeat happened to mention.
    let eff: EffectiveStatus = sqlx::query_as(
        "INSERT INTO server_statuses \
         (server_id, is_online, player_count, max_players, server_fps, uptime_seconds, \
          current_match_id, ingame_time, ingame_weather, updated_at, \
          telemetry_queue_backlog, telemetry_queue_capacity, telemetry_queue_dropped_total, \
          telemetry_queue_oldest_age_seconds, telemetry_queue_reported_at) \
         VALUES ($1, COALESCE($2, false), COALESCE($3, 0), COALESCE($4, 64), \
                 COALESCE($5::float8, 0)::numeric, COALESCE($6, 0), \
                 $7, COALESCE($8, ''), COALESCE($9, ''), $11, \
                 $12, $13, $14, $15, CASE WHEN $12::bigint IS NULL THEN NULL ELSE $11 END) \
         ON CONFLICT (server_id) DO UPDATE SET \
          is_online = COALESCE($2, server_statuses.is_online), \
          player_count = COALESCE($3, server_statuses.player_count), \
          max_players = COALESCE($4, server_statuses.max_players), \
          server_fps = COALESCE($5::float8::numeric, server_statuses.server_fps), \
          uptime_seconds = COALESCE($6, server_statuses.uptime_seconds), \
          current_match_id = CASE WHEN $10 THEN $7 ELSE server_statuses.current_match_id END, \
          ingame_time = COALESCE($8, server_statuses.ingame_time), \
          ingame_weather = COALESCE($9, server_statuses.ingame_weather), \
          updated_at = $11, \
          telemetry_queue_backlog = COALESCE($12, server_statuses.telemetry_queue_backlog), \
          telemetry_queue_capacity = COALESCE($13, server_statuses.telemetry_queue_capacity), \
          telemetry_queue_dropped_total = \
            COALESCE($14, server_statuses.telemetry_queue_dropped_total), \
          telemetry_queue_oldest_age_seconds = \
            COALESCE($15, server_statuses.telemetry_queue_oldest_age_seconds), \
          telemetry_queue_reported_at = CASE WHEN $12::bigint IS NULL \
            THEN server_statuses.telemetry_queue_reported_at ELSE $11 END \
         RETURNING is_online, player_count, server_fps::float8 AS server_fps",
    )
    .bind(server_id)
    .bind(input.is_online)
    .bind(input.player_count)
    .bind(input.max_players)
    .bind(input.server_fps)
    .bind(input.uptime_seconds)
    .bind(match_id)
    .bind(coalesce_str(&input.ingame_time))
    .bind(coalesce_str(&input.ingame_weather))
    .bind(set_match_id)
    .bind(now)
    .bind(telemetry_queue.map(|queue| queue.backlog))
    .bind(telemetry_queue.map(|queue| queue.capacity))
    .bind(telemetry_queue.map(|queue| queue.dropped_total))
    .bind(telemetry_queue.map(|queue| queue.oldest_age_seconds))
    .fetch_one(&mut *transaction)
    // `server_id` is the credential's registered server; a `current_match_id` naming no match
    // is the sender's error and answers 400.
    .await
    .map_err(|e| foreign_key_error(&e).unwrap_or_else(|| e.into()))?;

    // Time-series sample — only when the heartbeat actually measured something the series
    // records. A context-only heartbeat (weather, current match) is not a new data point,
    // and appending one would restate the previous sample as if it were freshly observed.
    if input.player_count.is_some() || input.server_fps.is_some() {
        sqlx::query(
            "INSERT INTO server_status_histories (server_id, player_count, server_fps) \
             VALUES ($1, $2, $3::float8::numeric)",
        )
        .bind(server_id)
        .bind(eff.player_count)
        .bind(eff.server_fps)
        .execute(&mut *transaction)
        .await?;
    }
    transaction.commit().await?;

    // Only a heartbeat that actually reported FPS can trip the low-FPS edge; the online
    // check reads the merged row so a heartbeat that omits `is_online` still warns for a
    // server we know is up.
    if let Some(fps) = input.server_fps
        && prev_healthy
        && fps < LOW_FPS_THRESHOLD
        && eff.is_online
    {
        write_audit(
            &state.pool,
            AuditSeverity::Warn,
            None,
            "system",
            "server.low_fps",
            &format!("Active server FPS dropped below 20 (now {fps:.1})"),
            "server",
            &server_id.to_string(),
        )
        .await;
    }

    // Fan out to SSE subscribers (the same helper the scheduled republisher uses — one payload
    // shape for ingest and poller).
    // Publish the committed row, queue reading included, through the one topic serializer.
    publish_server_status_by_id(&state.pool, &state.hub, server_id).await?;

    Ok(Json(json!({ "ok": true })))
}

#[cfg(test)]
#[path = "tests/server_heartbeat.rs"]
mod tests;
