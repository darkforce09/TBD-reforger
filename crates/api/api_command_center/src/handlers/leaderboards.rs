//! The ranked community leaderboard tables.
//!
//! **Role:** the `GET /api/v1/leaderboards` handler, the row it serves and the projection the
//! statistics card shares.
//! **Position:** reads the `leaderboard_totals` materialized view (kept current by
//! `api_member_activity`) joined with `users`; called by the router through [`crate::routes()`]
//! and by the API's integration suites; [`super::user_stats_card`] reuses the projection.
//! **Signals & state:** none; one read per request.
//! **Invariants:** the ordering comes only from the fixed category list, never from request text;
//! every ordering ends on `discord_id`, so tied scores page deterministically; a page holds at
//! most 50 rows.
//!
//! @contract command-center.schema.json#/definitions/LeaderboardPage
//! @contract command-center.schema.json#/definitions/LeaderboardRow

use api_identifiers::DiscordUserId;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::response::Json;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::QueryBuilder;

use api_foundation::error_handling::api_error::ApiError;
use api_http_layer::middleware::AuthUser;
use api_state::AppState;

/// One ranked entry joined with the user's display info. Numeric MV columns are
/// cast (`::int8` / `::float8`) into the wire types.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct LeaderboardRow {
    /// The player's Discord account.
    pub discord_id: DiscordUserId,
    /// The player's display name; empty when the account has none.
    pub username: String,
    /// The player's avatar URL; empty when the account has none.
    pub avatar_url: String,
    /// Kills over every recorded match.
    pub kills: i64,
    /// Deaths over every recorded match.
    pub deaths: i64,
    /// `NULL` when no `match_player_stats` row for this player has a measured `deaths`
    /// reading. Distinct from `0.0` (measured zero-death / flawless aggregate).
    pub kd_ratio: Option<f64>,
    /// Team kills over every recorded match.
    pub team_kills: i64,
    /// The longest recorded kill, in metres.
    pub longest_kill_m: i64,
    /// Vehicles destroyed over every recorded match.
    pub vehicles_destroyed: i64,
    /// Distinct matches the player has a stat line in.
    pub missions_played: i64,
    /// Command-role lines that recorded a command win.
    pub command_wins: i64,
    /// Command wins over command-role lines, from 0 to 1, rounded to three places; 0 with no
    /// command-role line.
    pub command_win_rate: f64,
    /// The 1-based place on the board the request reads, numbered by the leaderboard handler;
    /// 0 on the statistics card.
    #[sqlx(default)]
    pub rank: i64,
}

/// Whitelisted category → ORDER BY clause (avoids injection). Every arm ends in the
/// `lt.discord_id ASC` tie-breaker: the view is unique on `discord_id`, so equal scores page
/// deterministically — without it LIMIT/OFFSET over tied scores may repeat one row across pages
/// and skip another.
fn order_clause(category: &str) -> Option<&'static str> {
    match category {
        "kd" => Some("lt.kd_ratio DESC NULLS LAST, lt.discord_id ASC"),
        "command_win" => Some("lt.command_win_rate DESC NULLS LAST, lt.discord_id ASC"),
        "missions" => Some("lt.missions_played DESC, lt.discord_id ASC"),
        "longest_kill" => Some("lt.longest_kill_m DESC, lt.discord_id ASC"),
        "team_kills" => Some("lt.team_kills DESC, lt.discord_id ASC"),
        _ => None,
    }
}

/// The projection every [`LeaderboardRow`] read shares, board and single-player card alike.
pub(super) const LB_SELECT: &str = "SELECT lt.discord_id, COALESCE(u.username, '') AS username, COALESCE(u.avatar_url, '') AS avatar_url, \
    lt.kills::int8 AS kills, lt.deaths::int8 AS deaths, lt.kd_ratio::float8 AS kd_ratio, \
    lt.team_kills::int8 AS team_kills, lt.longest_kill_m::int8 AS longest_kill_m, \
    lt.vehicles_destroyed::int8 AS vehicles_destroyed, lt.missions_played::int8 AS missions_played, \
    lt.command_wins::int8 AS command_wins, lt.command_win_rate::float8 AS command_win_rate, \
    0::int8 AS rank ";

/// The query of `GET /api/v1/leaderboards`: `category` (default `kd`), the name search `q`, and
/// `limit` (1 to 100, default 20, capped at 50) and `offset` (default 0).
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
    query: Result<Query<LeaderboardQuery>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let Query(q) = query
        .map_err(|rejection| ApiError::from_query_rejection(rejection, "leaderboard query"))?;
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

#[cfg(test)]
#[path = "tests/leaderboards.rs"]
mod tests;
