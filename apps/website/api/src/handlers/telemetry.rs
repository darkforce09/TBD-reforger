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

/// A live-status heartbeat.
///
/// **Every measurement here is `Option` on purpose, and absent means "no new reading" —
/// do not add `#[serde(default)]` back (T-316).** This is the one struct in the T-218
/// sweep where requiring the fields would have been the *wrong* fix. A heartbeat is a
/// periodic push from a game server, and the wire contract has always allowed a sender to
/// report only what it currently knows — the committed integration test posts a heartbeat
/// with no `uptime_seconds`, `ingame_time` or `ingame_weather`, and the low-FPS case omits
/// `max_players` as well. Making those mandatory would break real senders to fix a bug
/// they don't have.
///
/// The bug was that "absent" decoded as an affirmative **zero** and was bound straight into
/// the upsert: a heartbeat carrying only `server_id` + `is_online` overwrote a live
/// `player_count=48, server_fps=58.5, max_players=64, uptime_seconds=7200` row with all
/// zeros, appended a permanent `0 / 0.0` row to `server_status_histories` (a time series —
/// that sample can never be corrected), and tripped the `server.low_fps` edge trigger into
/// a WARN about an FPS collapse that never happened.
///
/// So the fix is per-field, not blanket: `None` keeps the stored value (`COALESCE` against
/// the existing row), `Some` sets it. `current_match_id` needs three states rather than two
/// — absent keeps, and an explicit `""` clears — because a match really does end and the
/// live row has to stop pointing at it; the empty-string-means-none convention is already
/// what `parse_uuid_opt` implements. A heartbeat with nothing but a `server_id` carries no
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
    ingame_time: Option<String>,
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
/// heartbeat used to publish its own zeros).
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
    // malformed request — and the old code turned it into one by writing eight defaults.
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

    // Three-state (see `ServerStatusInput`): absent keeps, present sets, `""` clears.
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
    .bind(input.ingame_time.as_deref())
    .bind(input.ingame_weather.as_deref())
    .bind(set_match_id)
    .bind(now)
    .fetch_one(&state.pool)
    .await?;

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

    // Fan out to SSE subscribers (the exact struct Go marshals).
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
    if let Ok(payload) = serde_json::to_vec(&status) {
        state.hub.publish(&format!("server:{server_id}"), payload);
    }

    Ok(Json(json!({ "ok": true })))
}

// --- match results ---

/// The match half of a results POST.
///
/// **`outcome` is deliberately required — do not add `#[serde(default)]` to it, and do not
/// re-derive `Default` for this struct (T-316).** Unlike a heartbeat, a *result* has no
/// honest reason to omit how the match ended; that is the one thing the endpoint exists to
/// report. The default decoded as `""`, `""` mapped to `MissionOutcome::Pending`, and the
/// re-ingest path binds it unconditionally — so re-POSTing a known `source_match_id` with a
/// partial body walked a finished, won match backwards to `pending`. The `Default` derive
/// was the second half of the same hole: `MatchResultsInput.match_data` was `#[serde(default)]`,
/// so a body of `{}` skipped the field requirement entirely and minted an anonymous
/// `pending` match row on every call. Both are gone; `{}` is now a decode error → 400.
///
/// Requiring `outcome` costs a sender nothing — a match that genuinely has not finished can
/// still say `"pending"` out loud. What it removes is the *silent* pending.
///
/// The other three are `Option` rather than required, because unlike `outcome` they each
/// have a legitimate absence and the destructive part was only ever the overwrite:
/// - `winning_faction` — a `failure`/`aborted`/`pending` match has no winner, so demanding
///   one would be a lie. Absent keeps whatever is stored; an explicit `""` clears it, which
///   is the re-adjudication path.
/// - `aar_replay_url` — the replay is uploaded *after* the match, so the POST that carries
///   the result usually cannot know the link yet and a later pass attaches it. Defaulting
///   this to `""` meant the next result POST tore the link back off.
/// - `ended_at` — not named in the ticket, but it sits in the same `UPDATE` and was nulled
///   by the same partial body, so it gets the same treatment.
#[derive(Debug, Deserialize)]
pub struct MatchInput {
    source_match_id: Option<String>,
    event_id: Option<String>,
    mission_id: Option<String>,
    terrain: Option<String>,
    started_at: Option<DateTime<Utc>>,
    ended_at: Option<DateTime<Utc>>,
    outcome: String,
    winning_faction: Option<String>,
    aar_replay_url: Option<String>,
}

/// One player's final line for one match.
///
/// **The counters are deliberately required — do not add `#[serde(default)]` back (T-316).**
/// This row is keyed `(match_id, arma_id, source_event_id)` and holds *final per-match
/// totals*, and the upsert replaces them wholesale, so a re-ingest that omitted them wrote
/// `kills=0 deaths=0 … is_command=false command_win=NULL` over a real scoreline — which
/// `leaderboard_totals` then summed, in the same request, via `refresh_leaderboard`.
///
/// Three fixes were on the table and only one of them is honest:
/// - **`GREATEST(existing, incoming)`** — rejected. It reads as "counters only go up", but
///   half this struct is not a counter: `is_command`, `command_win` and `role_played` were
///   corrupted by the same write and `GREATEST` means nothing for them, so the rule would
///   have to be applied field-by-field and would stop being a rule. Worse, it makes the row
///   a permanent high-water mark: a downward correction after an anti-cheat review could
///   never be applied through the API. And it *absorbs* a broken sender instead of
///   reporting it — on a service-token endpoint with nobody watching, that is the one thing
///   we cannot afford.
/// - **Reject the re-ingest as a duplicate** — rejected. Retry safety is the contract here;
///   the endpoint is documented and tested as idempotent, and a game server that retries a
///   dropped response must not get a 409.
/// - **Full replace, with the fields required** — taken. The POST is authoritative for this
///   player-in-this-match, a restatement is exactly what a retry sends, corrections still
///   work in both directions, and an incomplete body is what it always was: a bug in the
///   sender, now answered with a 400 instead of a silent zeroing.
///
/// `command_win` stays optional because it is a genuine tri-state — `NULL` means "not a
/// command slot / not adjudicated", which is a different statement from `false`.
#[derive(Debug, Deserialize)]
pub struct PlayerStatInput {
    arma_id: String,
    role_played: String,
    kills: i64,
    deaths: i64,
    team_kills: i64,
    longest_kill_m: i64,
    vehicles_destroyed: i64,
    is_command: bool,
    command_win: Option<bool>,
    source_event_id: String,
}

/// Both keys are required: the handler's own error message has always claimed "match and
/// players are required", and `#[serde(default)]` on either one made that a lie. `players`
/// may still be `[]` — an explicit empty roster is a statement; a missing key is not.
#[derive(Debug, Deserialize)]
pub struct MatchResultsInput {
    #[serde(rename = "match")]
    match_data: MatchInput,
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

    // `""` is no longer an accepted outcome: it used to be spelled `Pending`, which is how a
    // won match reverted. An unfinished match must say `"pending"` explicitly.
    let outcome = match m.outcome.trim() {
        "success" => MissionOutcome::Success,
        "failure" => MissionOutcome::Failure,
        "aborted" => MissionOutcome::Aborted,
        "pending" => MissionOutcome::Pending,
        _ => return Err(ApiError::bad_request("invalid outcome")),
    };

    // Validate the whole roster before opening the transaction — an empty `arma_id` or
    // `source_event_id` decodes fine but is junk in a row whose dedupe key is
    // `(match_id, arma_id, source_event_id)`, and a blank key silently collapses distinct
    // players onto one row. Same reasoning as T-218's `reason.trim()`: an empty string is
    // the same lie as a missing field.
    for p in &input.players {
        if p.arma_id.trim().is_empty() {
            return Err(ApiError::bad_request("player arma_id is required"));
        }
        if p.source_event_id.trim().is_empty() {
            return Err(ApiError::bad_request("player source_event_id is required"));
        }
    }

    let mut tx = state.pool.begin().await?;
    let (match_id, event_id) = upsert_match(&mut tx, &m, outcome).await?;

    let mut resolved: Vec<String> = Vec::new();
    for p in &input.players {
        // Bind the trimmed forms (T-218 house pattern): they are two thirds of the dedupe
        // key, so `"abc"` and `" abc"` must not become two rows for the same player.
        let arma_id = p.arma_id.trim();
        let source_event_id = p.source_event_id.trim();
        let discord_id: Option<String> = sqlx::query_scalar(
            "SELECT discord_id FROM users WHERE arma_id = $1 AND deleted_at IS NULL",
        )
        .bind(arma_id)
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
        .bind(arma_id)
        .bind(&discord_id)
        .bind(&p.role_played)
        .bind(p.kills)
        .bind(p.deaths)
        .bind(p.team_kills)
        .bind(p.longest_kill_m)
        .bind(p.vehicles_destroyed)
        .bind(p.is_command)
        .bind(p.command_win)
        .bind(source_event_id)
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
    // Terrain stays optional, but a non-empty value we don't recognise is a typo, not a
    // "no terrain" — silently storing NULL for it is the same silent-data-loss shape as the
    // rest of this ticket. Mirrors the `invalid outcome` 400 above.
    let terrain = match m
        .terrain
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        None => None,
        Some(t) => Some(valid_terrain(t).ok_or_else(|| ApiError::bad_request("invalid terrain"))?),
    };

    if let Some(src) = m.source_match_id.as_deref().filter(|s| !s.is_empty()) {
        let existing: Option<(Uuid, Option<Uuid>)> =
            sqlx::query_as("SELECT id, event_id FROM matches WHERE source_match_id = $1")
                .bind(src)
                .fetch_optional(&mut *tx)
                .await?;
        if let Some((id, ev)) = existing {
            // `COALESCE($n, <column>)` — a bare column name on the right of a SET reads the
            // pre-update row, so an omitted field keeps what is already there instead of
            // overwriting it with a decoded default. `outcome` is bound unconditionally
            // because it is required on the way in.
            sqlx::query(
                "UPDATE matches SET \
                  ended_at = COALESCE($1, ended_at), \
                  outcome = $2, \
                  winning_faction = COALESCE($3, winning_faction), \
                  aar_replay_url = COALESCE($4, aar_replay_url) \
                 WHERE id = $5",
            )
            .bind(m.ended_at)
            .bind(outcome)
            .bind(m.winning_faction.as_deref())
            .bind(m.aar_replay_url.as_deref())
            .bind(id)
            .execute(&mut *tx)
            .await?;
            return Ok((id, ev));
        }
    }

    // On create, an absent winner/AAR still stores `''` rather than NULL — `models::Match`
    // decodes both as a non-optional `String`, so a NULL would break the read path.
    let row: (Uuid, Option<Uuid>) = sqlx::query_as(
        "INSERT INTO matches \
         (source_match_id, event_id, mission_id, terrain, started_at, ended_at, outcome, \
          winning_faction, aar_replay_url, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, COALESCE($8, ''), COALESCE($9, ''), now()) \
         RETURNING id, event_id",
    )
    .bind(&m.source_match_id)
    .bind(event_id)
    .bind(mission_id)
    .bind(terrain)
    .bind(started)
    .bind(m.ended_at)
    .bind(outcome)
    .bind(m.winning_faction.as_deref())
    .bind(m.aar_replay_url.as_deref())
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
