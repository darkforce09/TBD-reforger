//! `POST /api/v1/ingest/server-status` — the game server's live-status heartbeat: the partial
//! upsert, the time-series sample, the low-FPS edge warning, and the SSE fan-out.

use axum::extract::State;
use axum::extract::rejection::JsonRejection;
use axum::response::Json;
use chrono::Utc;
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::administration::models::audit_log::AuditSeverity;
use crate::administration::services::audit_writer::write_audit;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::ServiceAuth;
use crate::server_infrastructure::models::server::ServerStatus;
use crate::server_infrastructure::services::status_broadcast::publish_server_status;

use super::ingest_parsing::{coalesce_str, foreign_key_error, parse_uuid_opt};

const LOW_FPS_THRESHOLD: f64 = 20.0;

/// A live-status heartbeat.
///
/// **Every measurement here is `Option` on purpose, and absent means "no new reading" —
/// do not add `#[serde(default)]` back.** This is the one input struct where requiring the
/// fields would be the *wrong* fix. A heartbeat is a periodic push from a game server, and the
/// wire contract has always allowed a sender to report only what it currently knows — the
/// committed integration test posts a heartbeat with no `uptime_seconds`, `ingame_time` or
/// `ingame_weather`, and the low-FPS case omits `max_players` as well. Making those mandatory
/// would break real senders to fix a bug they don't have.
///
/// The bug the optionality closes is that "absent" decoding as an affirmative **zero** is bound
/// straight into the upsert: a heartbeat carrying only `server_id` + `is_online` overwrites a live
/// `player_count=48, server_fps=58.5, max_players=64, uptime_seconds=7200` row with all
/// zeros, appends a permanent `0 / 0.0` row to `server_status_histories` (a time series —
/// that sample can never be corrected), and trips the `server.low_fps` edge trigger into
/// a WARN about an FPS collapse that never happened.
///
/// So the rule is per-field, not blanket: `None` keeps the stored value (`COALESCE` against
/// the existing row), `Some` sets it. `current_match_id` needs three states rather than two
/// — absent keeps, and an explicit `""` clears — because a match really does end and the
/// live row has to stop pointing at it; the empty-string-means-none convention is what
/// [`parse_uuid_opt`] implements. A heartbeat with nothing but a `server_id` carries no
/// reading at all, so it is a 400 rather than a write of nothing.
#[derive(Debug, Deserialize)]
pub struct ServerStatusInput {
    server_id: String,
    is_online: Option<bool>,
    player_count: Option<i64>,
    max_players: Option<i64>,
    server_fps: Option<f64>,
    uptime_seconds: Option<i64>,
    /// Absent = leave the current match alone; `""` = clear it; a uuid = set it.
    current_match_id: Option<String>,
    /// Absent = keep; `""` / whitespace = clear; otherwise set (trimmed). See [`coalesce_str`].
    ingame_time: Option<String>,
    /// Absent = keep; `""` / whitespace = clear; otherwise set (trimmed). See [`coalesce_str`].
    ingame_weather: Option<String>,
}

impl ServerStatusInput {
    /// True when the body says nothing beyond naming the server.
    fn is_empty_reading(&self) -> bool {
        self.is_online.is_none()
            && self.player_count.is_none()
            && self.max_players.is_none()
            && self.server_fps.is_none()
            && self.uptime_seconds.is_none()
            && self.current_match_id.is_none()
            && self.ingame_time.is_none()
            && self.ingame_weather.is_none()
    }
}

/// The server's live status after a heartbeat is folded in — what actually landed in the
/// row, which is what the SSE subscribers and the history sample have to reflect (a partial
/// heartbeat must not publish its own zeros).
#[derive(Debug, sqlx::FromRow)]
struct EffectiveStatus {
    is_online: bool,
    player_count: i64,
    max_players: i64,
    server_fps: f64,
    uptime_seconds: i64,
    current_match_id: Option<Uuid>,
    ingame_time: String,
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
    if input.server_id.trim().is_empty() {
        return Err(ApiError::bad_request("server_id required"));
    }
    let Ok(server_id) = Uuid::parse_str(input.server_id.trim()) else {
        return Err(ApiError::bad_request("invalid server_id"));
    };
    // A heartbeat that reports nothing is not a "server is at zero" reading, it is a
    // malformed request — writing eight defaults for it turns it into one.
    if input.is_empty_reading() {
        return Err(ApiError::bad_request(
            "heartbeat must carry at least one field besides server_id",
        ));
    }

    // Edge-trigger the low-FPS warning: only when crossing below the threshold. Read the
    // pre-update value before the upsert.
    let prev_fps: Option<f64> =
        sqlx::query_scalar("SELECT server_fps::float8 FROM server_statuses WHERE server_id = $1")
            .bind(server_id)
            .fetch_optional(&state.pool)
            .await?;
    let prev_healthy = prev_fps.map(|f| f >= LOW_FPS_THRESHOLD).unwrap_or(true);

    // Three-state (see [`ServerStatusInput`]): absent keeps, present sets, `""` clears.
    let set_match_id = input.current_match_id.is_some();
    let match_id = parse_uuid_opt(&input.current_match_id);
    let now = Utc::now();

    // `COALESCE($n, <stored>)` in the DO UPDATE — deliberately reading the bind parameters
    // and not `EXCLUDED`, because `EXCLUDED` holds the row the VALUES clause already
    // defaulted, so it is never NULL and would defeat the whole point. `RETURNING` gives us
    // the merged row so the history sample and the SSE payload report what the server's
    // state actually is, not just the slice of it this heartbeat happened to mention.
    let eff: EffectiveStatus = sqlx::query_as(
        "INSERT INTO server_statuses \
         (server_id, is_online, player_count, max_players, server_fps, uptime_seconds, \
          current_match_id, ingame_time, ingame_weather, updated_at) \
         VALUES ($1, COALESCE($2, false), COALESCE($3, 0), COALESCE($4, 64), \
                 COALESCE($5::float8, 0)::numeric, COALESCE($6, 0), \
                 $7, COALESCE($8, ''), COALESCE($9, ''), $11) \
         ON CONFLICT (server_id) DO UPDATE SET \
          is_online = COALESCE($2, server_statuses.is_online), \
          player_count = COALESCE($3, server_statuses.player_count), \
          max_players = COALESCE($4, server_statuses.max_players), \
          server_fps = COALESCE($5::float8::numeric, server_statuses.server_fps), \
          uptime_seconds = COALESCE($6, server_statuses.uptime_seconds), \
          current_match_id = CASE WHEN $10 THEN $7 ELSE server_statuses.current_match_id END, \
          ingame_time = COALESCE($8, server_statuses.ingame_time), \
          ingame_weather = COALESCE($9, server_statuses.ingame_weather), \
          updated_at = $11 \
         RETURNING is_online, player_count, max_players, server_fps::float8 AS server_fps, \
          uptime_seconds, current_match_id, COALESCE(ingame_time, '') AS ingame_time, \
          COALESCE(ingame_weather, '') AS ingame_weather",
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
    .fetch_one(&state.pool)
    // `server_id` is bound straight from the body with no existence check, and since `0018`
    // constraint 10 enforces it a heartbeat for an unregistered server would otherwise be a 500.
    // Both pointers on this INSERT are covered: `server_id` today, `current_match_id` the moment
    // its constraint lands.
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
        // **Deliberately NOT mapped.** `server_status_histories_server_id_fkey` names the
        // same parent as the statement above, which just succeeded — so by the time this runs the
        // server provably existed, and the only way to reach a 23503 here is a deregistration
        // landing between the two statements. That is a race in the platform's own state, not a
        // bad body: answering 400 "unknown server_id" would tell the bridge to stop sending a
        // payload that was correct when it was sent. A 500 for a genuine race is the honest
        // answer, and an arm no test can reach is an arm no perturbation can prove.
        .execute(&state.pool)
        .await?;
    }

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
    let status = ServerStatus {
        server_id,
        is_online: eff.is_online,
        player_count: eff.player_count,
        max_players: eff.max_players,
        server_fps: eff.server_fps,
        uptime_seconds: eff.uptime_seconds,
        current_match_id: eff.current_match_id,
        ingame_time: eff.ingame_time,
        ingame_weather: eff.ingame_weather,
        updated_at: now,
    };
    publish_server_status(&state.hub, &status);

    Ok(Json(json!({ "ok": true })))
}

#[cfg(test)]
#[path = "tests/server_heartbeat.rs"]
mod tests;
