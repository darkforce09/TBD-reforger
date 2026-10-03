//! One player's aggregate statistics card.
//!
//! **Role:** the `GET /api/v1/users/{discordId}/stats` handler.
//! **Position:** reads the account through `api_identity_and_access` and the player's
//! leaderboard row through the projection of [`super::leaderboards`]; called by the router
//! through [`crate::routes()`].
//! **Signals & state:** none; one read per request.
//! **Invariants:** an unknown or deleted account is a 404; a known account without a leaderboard
//! row still answers its card.
//!
//! @contract command-center.schema.json#/definitions/PlayerStatsCard

use api_identifiers::DiscordUserId;
use axum::extract::State;
use axum::response::Json;
use serde_json::{Value, json};
use sqlx::QueryBuilder;

use super::leaderboards::{LB_SELECT, LeaderboardRow};
use api_foundation::error_handling::api_error::ApiError;
use api_foundation::http::path_parameters::PathParams;
use api_http_layer::middleware::AuthUser;
use api_identity_and_access::services::user_lookup::load_user;
use api_state::AppState;

/// `GET /api/v1/users/:discordId/stats` — one player's aggregate card.
///
/// @route GET /api/v1/users/:discordId/stats
pub async fn get_user_stats(
    State(state): State<AppState>,
    _u: AuthUser,
    PathParams(discord_id): PathParams<DiscordUserId>,
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
        kd_ratio: None,
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
