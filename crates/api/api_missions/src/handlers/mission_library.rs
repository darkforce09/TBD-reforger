//! Mission library reads: the filtered browse list, the single-mission overview, and the
//! per-caller bookmark toggle.
//!
//! Every handler here is `AuthUser` tier and owner-scoped — the scope tabs and the bookmark
//! rows are all keyed on the calling user's discord id. The overview, both bookmark writes and the
//! `bookmarked` scope apply one visibility rule, `can_view`: a live mission is visible to
//! everyone, any other mission to its author and to administrators. A mission the caller cannot
//! see answers `404 mission not found`, exactly like a mission that does not exist, and a bookmark
//! row on a mission that has left the caller's view drops out of the `bookmarked` scope.
//!
//! @contract mission-library.schema.json#/definitions/MissionLibraryPage
//! @contract mission-library.schema.json#/definitions/MissionCard
//! @contract mission-library.schema.json#/definitions/MissionDetail
//! @contract mission-library.schema.json#/definitions/BookmarkState

use api_identifiers::{DiscordUserId, MissionId};
use axum::extract::rejection::QueryRejection;
use axum::extract::{Query, State};
use axum::response::Json;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, QueryBuilder};

use crate::models::mission::{Mission, MissionArmory, MissionVersion};
use crate::services::mission_lookup::load_mission_or_404;
use crate::validation::access::can_view;
use crate::validation::mission_fields::{parse_range, valid_game_mode, valid_terrain};
use api_foundation::error_handling::api_error::ApiError;
use api_foundation::http::path_parameters::PathParams;
use api_http_layer::middleware::AuthUser;
use api_state::AppState;

/// Library list item: mission + denormalized author + bookmark state.
#[derive(Debug, Serialize)]
pub struct MissionCard {
    /// The mission row, flattened into the card.
    #[serde(flatten)]
    pub mission: Mission,
    /// The author's display name.
    pub author_name: String,
    /// The author's avatar URL; empty when the author has none.
    pub author_avatar: String,
    /// Whether the caller has bookmarked the mission.
    pub bookmarked: bool,
}

/// Batch-load authors + the caller's bookmarks and build cards.
async fn decorate(
    pool: &PgPool,
    me: &DiscordUserId,
    missions: Vec<Mission>,
) -> sqlx::Result<Vec<MissionCard>> {
    let author_ids: Vec<DiscordUserId> = missions.iter().map(|m| m.author_id.clone()).collect();
    let mission_ids: Vec<MissionId> = missions.iter().map(|m| m.id).collect();

    let authors: Vec<(DiscordUserId, String, String)> = sqlx::query_as(
        "SELECT discord_id, COALESCE(username, '') AS username, COALESCE(avatar_url, '') AS avatar_url FROM users WHERE discord_id = ANY($1)",
    )
    .bind(&author_ids)
    .fetch_all(pool)
    .await?;
    let bookmarks: Vec<MissionId> = sqlx::query_scalar(
        "SELECT mission_id FROM mission_bookmarks WHERE discord_id = $1 AND mission_id = ANY($2)",
    )
    .bind(me)
    .bind(&mission_ids)
    .fetch_all(pool)
    .await?;

    Ok(missions
        .into_iter()
        .map(|m| {
            let author = authors.iter().find(|(id, _, _)| *id == m.author_id);
            MissionCard {
                author_name: author.map(|(_, n, _)| n.clone()).unwrap_or_default(),
                author_avatar: author.map(|(_, _, a)| a.clone()).unwrap_or_default(),
                bookmarked: bookmarks.contains(&m.id),
                mission: m,
            }
        })
        .collect())
}

const MISSION_COLS: &str = "id, title, author_id, terrain, COALESCE(custom_terrain_name, '') AS custom_terrain_name, \
     game_mode, weather, time_of_day::text AS time_of_day, max_players, status, \
     COALESCE(thumbnail_url, '') AS thumbnail_url, COALESCE(briefing, '') AS briefing, \
     current_version_id, approved_artifact_id, COALESCE(rejection_reason, '') AS rejection_reason, \
     reviewed_by, reviewed_at, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, \
     COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at";

/// The library list's query: the scope, the terrain, mode and player-count filters and the page.
#[derive(Debug, Deserialize)]
pub struct ListQuery {
    scope: Option<String>,
    terrain: Option<String>,
    mode: Option<String>,
    player_count: Option<String>,
    q: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

/// Push the scope + filter WHERE conditions (shared by count + select).
///
/// The `bookmarked` scope keeps only the bookmarked missions the caller can see now: the SQL form
/// of [`can_view`] (live, or authored by the caller, or any mission for an administrator).
fn push_filters(qb: &mut QueryBuilder<Postgres>, f: &ListQuery, user: &AuthUser) {
    let me = &user.discord_id;
    match f.scope.as_deref().unwrap_or("global") {
        "mine" => {
            qb.push(" AND author_id = ").push_bind(me.to_string());
        }
        "bookmarked" => {
            qb.push(" AND id IN (SELECT mission_id FROM mission_bookmarks WHERE discord_id = ")
                .push_bind(me.to_string())
                .push(") AND (status = 'live' OR author_id = ")
                .push_bind(me.to_string())
                .push(" OR ")
                .push_bind(user.role == "admin")
                .push(")");
        }
        _ => {
            qb.push(" AND (status = 'live' OR (author_id = ")
                .push_bind(me.to_string())
                .push(" AND status <> 'archived'))");
        }
    }
    if let Some(t) = f
        .terrain
        .as_deref()
        .filter(|t| !t.is_empty() && *t != "all")
        && let Some(terrain) = valid_terrain(t)
    {
        qb.push(" AND terrain = ").push_bind(terrain);
    }
    if let Some(m) = f.mode.as_deref().filter(|m| !m.is_empty() && *m != "all")
        && let Some(mode) = valid_game_mode(m)
    {
        qb.push(" AND game_mode = ").push_bind(mode);
    }
    if let Some(pc) = f
        .player_count
        .as_deref()
        .filter(|p| !p.is_empty() && *p != "all")
        && let Some((lo, hi)) = parse_range(pc)
    {
        qb.push(" AND max_players >= ")
            .push_bind(lo)
            .push(" AND max_players <= ")
            .push_bind(hi);
    }
    if let Some(search) = f.q.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        qb.push(" AND title ILIKE ")
            .push_bind(format!("%{search}%"));
    }
}

/// `GET /api/v1/missions` — library browser (scope tabs + filters).
///
/// @route GET /api/v1/missions
pub async fn list_missions(
    State(state): State<AppState>,
    user: AuthUser,
    query: Result<Query<ListQuery>, QueryRejection>,
) -> Result<Json<Value>, ApiError> {
    let Query(f) = query
        .map_err(|rejection| ApiError::from_query_rejection(rejection, "mission library query"))?;
    let me = &user.discord_id;
    let limit = f.limit.filter(|&n| n > 0 && n <= 100).unwrap_or(20);
    let offset = f.offset.filter(|&n| n >= 0).unwrap_or(0);

    let mut cq = QueryBuilder::new("SELECT count(*) FROM missions WHERE deleted_at IS NULL");
    push_filters(&mut cq, &f, &user);
    let total: i64 = cq
        .build_query_scalar()
        .fetch_one(&state.pool)
        .await
        .map_err(ApiError::from)?;

    let mut sq = QueryBuilder::new(format!(
        "SELECT {MISSION_COLS} FROM missions WHERE deleted_at IS NULL"
    ));
    push_filters(&mut sq, &f, &user);
    sq.push(" ORDER BY updated_at DESC LIMIT ")
        .push_bind(limit)
        .push(" OFFSET ")
        .push_bind(offset);
    let missions: Vec<Mission> = sq
        .build_query_as()
        .fetch_all(&state.pool)
        .await
        .map_err(ApiError::from)?;

    let cards = decorate(&state.pool, me, missions).await?;
    Ok(Json(
        json!({ "data": cards, "total": total, "limit": limit, "offset": offset }),
    ))
}

/// `GET /api/v1/missions/:id` — Mission Overview (card + armory + current version).
///
/// @route GET /api/v1/missions/:id
pub async fn get_mission(
    State(state): State<AppState>,
    user: AuthUser,
    PathParams(id): PathParams<String>,
) -> Result<Json<Value>, ApiError> {
    let m = load_visible_mission(&state.pool, &user, &id).await?;
    let card = decorate(&state.pool, &user.discord_id, vec![m.clone()])
        .await?
        .pop()
        .ok_or_else(|| ApiError::internal("could not build mission overview"))?;
    let armory: Vec<MissionArmory> = sqlx::query_as(
        "SELECT id, mission_id, faction, category, item_name, quantity, COALESCE(icon, '') AS icon, sort_order FROM mission_armories WHERE mission_id = $1 ORDER BY sort_order ASC",
    )
    .bind(m.id)
    .fetch_all(&state.pool)
    .await?;
    let current_version: Option<MissionVersion> = match m.current_version_id {
        Some(vid) => {
            sqlx::query_as("SELECT id, mission_id, semver, json_payload, COALESCE(editor_notes, '') AS editor_notes, created_by, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at FROM mission_versions WHERE id = $1")
                .bind(vid)
                .fetch_optional(&state.pool)
                .await?
        }
        None => None,
    };
    let overview_failure = |_| ApiError::internal("could not build mission overview");
    let mut body = serde_json::to_value(&card).map_err(overview_failure)?;
    let armory = serde_json::to_value(armory).map_err(overview_failure)?;
    let current_version = current_version
        .map(serde_json::to_value)
        .transpose()
        .map_err(overview_failure)?;
    let obj = body
        .as_object_mut()
        .ok_or_else(|| ApiError::internal("could not build mission overview"))?;
    obj.insert("armory".into(), armory);
    if let Some(v) = current_version {
        obj.insert("current_version".into(), v);
    }
    Ok(Json(body))
}

/// Load the mission the overview or a bookmark write addresses: `400` for a malformed id,
/// `404 mission not found` for a mission that does not exist or that the caller cannot see
/// ([`can_view`]).
async fn load_visible_mission(
    pool: &PgPool,
    user: &AuthUser,
    id: &str,
) -> Result<Mission, ApiError> {
    let mission = load_mission_or_404(pool, id).await?;
    if !can_view(user, &mission) {
        return Err(ApiError::not_found("mission not found"));
    }
    Ok(mission)
}

/// `POST /api/v1/missions/:id/bookmark` — idempotent add of a mission the caller can see.
///
/// @route POST /api/v1/missions/:id/bookmark
pub async fn bookmark_mission(
    State(state): State<AppState>,
    user: AuthUser,
    PathParams(id): PathParams<String>,
) -> Result<Json<Value>, ApiError> {
    let mission = load_visible_mission(&state.pool, &user, &id).await?;
    sqlx::query(
        "INSERT INTO mission_bookmarks (discord_id, mission_id, created_at) VALUES ($1, $2, now()) \
         ON CONFLICT (discord_id, mission_id) DO NOTHING",
    )
    .bind(&user.discord_id)
    .bind(mission.id)
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "bookmarked": true })))
}

/// `DELETE /api/v1/missions/:id/bookmark` — idempotent removal of the caller's bookmark on a
/// mission the caller can see.
///
/// @route DELETE /api/v1/missions/:id/bookmark
pub async fn remove_bookmark(
    State(state): State<AppState>,
    user: AuthUser,
    PathParams(id): PathParams<String>,
) -> Result<Json<Value>, ApiError> {
    let mission = load_visible_mission(&state.pool, &user, &id).await?;
    sqlx::query("DELETE FROM mission_bookmarks WHERE discord_id = $1 AND mission_id = $2")
        .bind(&user.discord_id)
        .bind(mission.id)
        .execute(&state.pool)
        .await?;
    Ok(Json(json!({ "bookmarked": false })))
}
