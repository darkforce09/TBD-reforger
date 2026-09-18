//! Mission library reads: the filtered browse list, the single-mission overview, and the
//! per-caller bookmark toggle.
//!
//! Every handler here is `AuthUser` tier and owner-scoped — the scope tabs and the bookmark
//! rows are all keyed on the calling user's discord id.

use axum::extract::{Path, Query, State};
use axum::response::Json;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AuthUser;
use crate::missions::models::mission::{Mission, MissionArmory, MissionVersion};
use crate::missions::services::mission_lookup::load_mission_or_404;
use crate::missions::validation::access::can_view;
use crate::missions::validation::mission_fields::{parse_range, valid_game_mode, valid_terrain};

/// Library list item: mission + denormalized author + bookmark state.
#[derive(Debug, Serialize)]
pub struct MissionCard {
    #[serde(flatten)]
    pub mission: Mission,
    pub author_name: String,
    pub author_avatar: String,
    pub bookmarked: bool,
}

/// Batch-load authors + the caller's bookmarks and build cards.
async fn decorate(
    pool: &PgPool,
    me: &str,
    missions: Vec<Mission>,
) -> sqlx::Result<Vec<MissionCard>> {
    let author_ids: Vec<String> = missions.iter().map(|m| m.author_id.clone()).collect();
    let mission_ids: Vec<Uuid> = missions.iter().map(|m| m.id).collect();

    let authors: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT discord_id, COALESCE(username, '') AS username, COALESCE(avatar_url, '') AS avatar_url FROM users WHERE discord_id = ANY($1)",
    )
    .bind(&author_ids)
    .fetch_all(pool)
    .await?;
    let bookmarks: Vec<Uuid> = sqlx::query_scalar(
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
     current_version_id, COALESCE(rejection_reason, '') AS rejection_reason, reviewed_by, reviewed_at, \
     COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at, \
     COALESCE(updated_at, '0001-01-01 00:00:00+00'::timestamptz) AS updated_at";

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
fn push_filters(qb: &mut QueryBuilder<Postgres>, f: &ListQuery, me: &str) {
    match f.scope.as_deref().unwrap_or("global") {
        "mine" => {
            qb.push(" AND author_id = ").push_bind(me.to_string());
        }
        "bookmarked" => {
            qb.push(" AND id IN (SELECT mission_id FROM mission_bookmarks WHERE discord_id = ")
                .push_bind(me.to_string())
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
    Query(f): Query<ListQuery>,
) -> Result<Json<Value>, ApiError> {
    let me = &user.discord_id;
    let limit = f.limit.filter(|&n| n > 0 && n <= 100).unwrap_or(20);
    let offset = f.offset.filter(|&n| n >= 0).unwrap_or(0);

    let mut cq = QueryBuilder::new("SELECT count(*) FROM missions WHERE deleted_at IS NULL");
    push_filters(&mut cq, &f, me);
    let total: i64 = cq
        .build_query_scalar()
        .fetch_one(&state.pool)
        .await
        .map_err(ApiError::from)?;

    let mut sq = QueryBuilder::new(format!(
        "SELECT {MISSION_COLS} FROM missions WHERE deleted_at IS NULL"
    ));
    push_filters(&mut sq, &f, me);
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
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let m = load_mission_or_404(&state.pool, &id).await?;
    if !can_view(&user, &m) {
        return Err(ApiError::not_found("mission not found"));
    }
    let card = decorate(&state.pool, &user.discord_id, vec![m.clone()])
        .await?
        .pop()
        .unwrap();
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
    let mut body = serde_json::to_value(&card).unwrap();
    let obj = body.as_object_mut().unwrap();
    obj.insert("armory".into(), serde_json::to_value(armory).unwrap());
    if let Some(v) = current_version {
        obj.insert("current_version".into(), serde_json::to_value(v).unwrap());
    }
    Ok(Json(body))
}

/// `POST /api/v1/missions/:id/bookmark` — idempotent add.
///
/// @route POST /api/v1/missions/:id/bookmark
pub async fn bookmark_mission(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let Ok(mid) = Uuid::parse_str(&id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    sqlx::query(
        "INSERT INTO mission_bookmarks (discord_id, mission_id, created_at) VALUES ($1, $2, now()) \
         ON CONFLICT (discord_id, mission_id) DO NOTHING",
    )
    .bind(&user.discord_id)
    .bind(mid)
    .execute(&state.pool)
    .await?;
    Ok(Json(json!({ "bookmarked": true })))
}

/// `DELETE /api/v1/missions/:id/bookmark`.
///
/// @route DELETE /api/v1/missions/:id/bookmark
pub async fn remove_bookmark(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let Ok(mid) = Uuid::parse_str(&id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    let _ = sqlx::query("DELETE FROM mission_bookmarks WHERE discord_id = $1 AND mission_id = $2")
        .bind(&user.discord_id)
        .bind(mid)
        .execute(&state.pool)
        .await;
    Ok(Json(json!({ "bookmarked": false })))
}
