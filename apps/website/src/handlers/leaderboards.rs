//! Leaderboards + player stats + SSE server-status — Rust port of `handlers/leaderboards.go`.

use std::convert::Infallible;

use async_stream::stream;
use axum::extract::{Path, Query, State};
use axum::http::HeaderName;
use axum::response::sse::{Event, Sse};
use axum::response::{IntoResponse, Json, Response};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::QueryBuilder;
use tokio::sync::broadcast::error::RecvError;
use uuid::Uuid;

use crate::error::ApiError;
use crate::handlers::load_user;
use crate::middleware::AuthUser;
use crate::models::ServerStatus;
use crate::state::AppState;

/// One ranked entry joined with the user's display info. Numeric MV columns are
/// cast (`::int8` / `::float8`) into the wire types.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct LeaderboardRow {
    pub discord_id: String,
    pub username: String,
    pub avatar_url: String,
    pub kills: i64,
    pub deaths: i64,
    pub kd_ratio: f64,
    pub team_kills: i64,
    pub longest_kill_m: i64,
    pub vehicles_destroyed: i64,
    pub missions_played: i64,
    pub command_wins: i64,
    pub command_win_rate: f64,
    #[sqlx(default)]
    pub rank: i64,
}

/// Whitelisted category → ORDER BY clause (avoids injection).
fn order_clause(category: &str) -> Option<&'static str> {
    match category {
        "kd" => Some("lt.kd_ratio DESC NULLS LAST"),
        "command_win" => Some("lt.command_win_rate DESC NULLS LAST"),
        "missions" => Some("lt.missions_played DESC"),
        "longest_kill" => Some("lt.longest_kill_m DESC"),
        "team_kills" => Some("lt.team_kills DESC"),
        _ => None,
    }
}

const LB_SELECT: &str = "SELECT lt.discord_id, COALESCE(u.username, '') AS username, COALESCE(u.avatar_url, '') AS avatar_url, \
    lt.kills::int8 AS kills, lt.deaths::int8 AS deaths, lt.kd_ratio::float8 AS kd_ratio, \
    lt.team_kills::int8 AS team_kills, lt.longest_kill_m::int8 AS longest_kill_m, \
    lt.vehicles_destroyed::int8 AS vehicles_destroyed, lt.missions_played::int8 AS missions_played, \
    lt.command_wins::int8 AS command_wins, lt.command_win_rate::float8 AS command_win_rate, \
    0::int8 AS rank ";

#[derive(Debug, Deserialize)]
pub struct LeaderboardQuery {
    category: Option<String>,
    q: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

/// `GET /api/v1/leaderboards` — ranked board for a category, searchable by name.
///
/// @route GET /api/v1/leaderboards
pub async fn get_leaderboards(
    State(state): State<AppState>,
    _u: AuthUser,
    Query(q): Query<LeaderboardQuery>,
) -> Result<Json<Value>, ApiError> {
    let category = q.category.as_deref().unwrap_or("kd").to_string();
    let Some(order) = order_clause(&category) else {
        return Err(ApiError::bad_request("unknown category"));
    };
    let limit = q.limit.filter(|&n| n > 0 && n <= 100).unwrap_or(20).min(50);
    let offset = q.offset.filter(|&n| n >= 0).unwrap_or(0);
    let search = q.q.as_deref().unwrap_or("").trim().to_string();

    // Dynamic ORDER BY comes only from the hardcoded whitelist; values are bound.
    let mut qb = QueryBuilder::new(LB_SELECT);
    qb.push("FROM leaderboard_totals lt JOIN users u ON u.discord_id = lt.discord_id AND u.deleted_at IS NULL WHERE u.username ILIKE ");
    qb.push_bind(format!("%{search}%"));
    qb.push(" ORDER BY ").push(order);
    qb.push(" LIMIT ")
        .push_bind(limit)
        .push(" OFFSET ")
        .push_bind(offset);

    let mut rows: Vec<LeaderboardRow> = qb
        .build_query_as()
        .fetch_all(&state.pool)
        .await
        .map_err(ApiError::from)?;
    for (i, row) in rows.iter_mut().enumerate() {
        row.rank = offset + i as i64 + 1;
    }
    Ok(Json(json!({ "category": category, "data": rows })))
}

/// `GET /api/v1/users/:discordId/stats` — one player's aggregate card.
///
/// @route GET /api/v1/users/:discordId/stats
pub async fn get_user_stats(
    State(state): State<AppState>,
    _u: AuthUser,
    Path(discord_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let Some(user) = load_user(&state.pool, &discord_id).await? else {
        return Err(ApiError::not_found("user not found"));
    };

    let mut qb = QueryBuilder::new(LB_SELECT);
    qb.push("FROM leaderboard_totals lt JOIN users u ON u.discord_id = lt.discord_id WHERE lt.discord_id = ");
    qb.push_bind(&discord_id);
    let row: Option<LeaderboardRow> = qb
        .build_query_as()
        .fetch_optional(&state.pool)
        .await
        .map_err(ApiError::from)?;

    let stats = row.unwrap_or(LeaderboardRow {
        discord_id: user.discord_id.clone(),
        username: user.username.clone(),
        avatar_url: user.avatar_url.clone(),
        kills: 0,
        deaths: 0,
        kd_ratio: 0.0,
        team_kills: 0,
        longest_kill_m: 0,
        vehicles_destroyed: 0,
        missions_played: 0,
        command_wins: 0,
        command_win_rate: 0.0,
        rank: 0,
    });

    Ok(Json(json!({
        "stats": stats,
        "total_operations": user.total_deployments,
        "attendance_rate": user.attendance_rate,
    })))
}

/// `GET /api/v1/servers/:id/status/stream` — SSE live server-status feed.
///
/// @route GET /api/v1/servers/:id/status/stream
pub async fn stream_server_status(
    State(state): State<AppState>,
    _u: AuthUser,
    Path(id): Path<String>,
) -> Response {
    let topic = format!("server:{id}");
    let mut rx = state.hub.subscribe(&topic);
    let pool = state.pool.clone();
    let uuid = Uuid::parse_str(&id).ok();

    let body = stream! {
        // Current snapshot first, so the client renders without delay.
        if let Some(sid) = uuid {
            let snap: Result<Option<ServerStatus>, _> = sqlx::query_as(
                "SELECT server_id, is_online, player_count, max_players, server_fps::float8 AS server_fps, uptime_seconds, current_match_id, COALESCE(ingame_time, '') AS ingame_time, COALESCE(ingame_weather, '') AS ingame_weather, COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at FROM server_statuses WHERE server_id = $1",
            ).bind(sid).fetch_optional(&pool).await;
            if let Ok(Some(status)) = snap
                && let Ok(js) = serde_json::to_string(&status) {
                yield Ok::<Event, Infallible>(Event::default().data(js));
            }
        }
        loop {
            match rx.recv().await {
                Ok(bytes) => yield Ok(Event::default().data(String::from_utf8_lossy(&bytes))),
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => break,
            }
        }
    };

    (
        [(HeaderName::from_static("x-accel-buffering"), "no")],
        Sse::new(body),
    )
        .into_response()
}
