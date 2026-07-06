//! Mission library + editor handlers — Rust port of `handlers/missions.go` +
//! `handlers/missions_compiled.go`. The `/compiled` route runs the Phase 8 flatten
//! engine live (gate G6 end-to-end).

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Json, Response};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use serde_json::{Value, json};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

use crate::contract::validate_mission_editor_payload;
use crate::error::ApiError;
use crate::handlers::{is_unique_violation, load_mission};
use crate::middleware::{AuthUser, MissionMakerUser, ServiceAuth};
use crate::models::{
    GameMode, Mission, MissionArmory, MissionStatus, MissionVersion, TerrainType, WeatherType,
};
use crate::services::{CompileError, ModMissionDocument, flatten_to_mod_document};
use crate::state::AppState;

// --- enum validators (mirror Go valid*; empty weather → clear) ---

fn valid_terrain(s: &str) -> Option<TerrainType> {
    match s {
        "everon" => Some(TerrainType::Everon),
        "arland" => Some(TerrainType::Arland),
        "custom" => Some(TerrainType::Custom),
        _ => None,
    }
}
fn valid_game_mode(s: &str) -> Option<GameMode> {
    match s {
        "pve_coop" => Some(GameMode::PveCoop),
        "pvp" => Some(GameMode::Pvp),
        "zeus" => Some(GameMode::Zeus),
        _ => None,
    }
}
fn valid_weather(s: &str) -> Option<WeatherType> {
    match s {
        "" | "clear" => Some(WeatherType::Clear),
        "overcast" => Some(WeatherType::Overcast),
        "heavy_rain" => Some(WeatherType::HeavyRain),
        "dense_fog" => Some(WeatherType::DenseFog),
        _ => None,
    }
}

fn can_edit(u: &AuthUser, m: &Mission) -> bool {
    m.author_id == u.discord_id || u.role == "admin"
}
fn can_view(u: &AuthUser, m: &Mission) -> bool {
    m.status == MissionStatus::Live || can_edit(u, m)
}

fn parse_range(s: &str) -> Option<(i64, i64)> {
    let (lo, hi) = s.split_once('-')?;
    let lo: i64 = lo.trim().parse().ok()?;
    let hi: i64 = hi.trim().parse().ok()?;
    (lo <= hi).then_some((lo, hi))
}

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
        "SELECT discord_id, username, avatar_url FROM users WHERE discord_id = ANY($1)",
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

const MISSION_COLS: &str = "id, title, author_id, terrain, custom_terrain_name, game_mode, weather, \
     time_of_day::text AS time_of_day, max_players, status, thumbnail_url, briefing, \
     current_version_id, rejection_reason, reviewed_by, reviewed_at, created_at, updated_at";

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
    let m = load(&state.pool, &id).await?;
    if !can_view(&user, &m) {
        return Err(ApiError::not_found("mission not found"));
    }
    let card = decorate(&state.pool, &user.discord_id, vec![m.clone()])
        .await?
        .pop()
        .unwrap();
    let armory: Vec<MissionArmory> = sqlx::query_as(
        "SELECT * FROM mission_armories WHERE mission_id = $1 ORDER BY sort_order ASC",
    )
    .bind(m.id)
    .fetch_all(&state.pool)
    .await?;
    let current_version: Option<MissionVersion> = match m.current_version_id {
        Some(vid) => {
            sqlx::query_as("SELECT * FROM mission_versions WHERE id = $1")
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

#[derive(Debug, Deserialize)]
pub struct CreateMissionInput {
    #[serde(default)]
    title: String,
    #[serde(default)]
    terrain: String,
    #[serde(default)]
    custom_terrain_name: String,
    #[serde(default)]
    game_mode: String,
    #[serde(default)]
    weather: String,
    #[serde(default)]
    time_of_day: String,
    #[serde(default)]
    max_players: i64,
    #[serde(default)]
    briefing: String,
    payload: Option<Box<RawValue>>,
}

/// `POST /api/v1/missions` — draft mission + initial v0.1.0 version (mission_maker+).
///
/// @route POST /api/v1/missions
pub async fn create_mission(
    State(state): State<AppState>,
    maker: MissionMakerUser,
    body: Result<Json<CreateMissionInput>, JsonRejection>,
) -> Result<(StatusCode, Json<Mission>), ApiError> {
    let Json(input) = body.map_err(|_| {
        ApiError::bad_request("title, terrain, game_mode and max_players are required")
    })?;
    if input.title.is_empty() || input.terrain.is_empty() || input.game_mode.is_empty() {
        return Err(ApiError::bad_request(
            "title, terrain, game_mode and max_players are required",
        ));
    }
    let Some(terrain) = valid_terrain(&input.terrain) else {
        return Err(ApiError::bad_request("invalid terrain"));
    };
    let Some(mode) = valid_game_mode(&input.game_mode) else {
        return Err(ApiError::bad_request("invalid game_mode"));
    };
    let Some(weather) = valid_weather(&input.weather) else {
        return Err(ApiError::bad_request("invalid weather"));
    };
    if input.max_players < 1 || input.max_players > 256 {
        return Err(ApiError::bad_request(
            "title, terrain, game_mode and max_players are required",
        ));
    }
    let time_of_day = if input.time_of_day.is_empty() {
        "14:00"
    } else {
        &input.time_of_day
    }
    .to_string();
    let payload_str = input.payload.as_ref().map_or("{}", |p| p.get()).to_string();

    validate_payload(&payload_str)?;

    let author = &maker.0.discord_id;
    let mut tx = state.pool.begin().await?;
    let mission_id: Uuid = sqlx::query_scalar(
        "INSERT INTO missions (title, author_id, terrain, custom_terrain_name, game_mode, weather, \
         time_of_day, max_players, status, thumbnail_url, briefing, rejection_reason, created_at, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7::time, $8, 'draft', '', $9, '', now(), now()) RETURNING id",
    )
    .bind(&input.title)
    .bind(author)
    .bind(terrain)
    .bind(&input.custom_terrain_name)
    .bind(mode)
    .bind(weather)
    .bind(&time_of_day)
    .bind(input.max_players)
    .bind(&input.briefing)
    .fetch_one(&mut *tx)
    .await?;
    let version_id: Uuid = sqlx::query_scalar(
        "INSERT INTO mission_versions (mission_id, semver, json_payload, editor_notes, created_by, created_at) \
         VALUES ($1, '0.1.0', $2::jsonb, '', $3, now()) RETURNING id",
    )
    .bind(mission_id)
    .bind(&payload_str)
    .bind(author)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query("UPDATE missions SET current_version_id = $1 WHERE id = $2")
        .bind(version_id)
        .bind(mission_id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;

    let mission = load(&state.pool, &mission_id.to_string()).await?;
    Ok((StatusCode::CREATED, Json(mission)))
}

#[derive(Debug, Deserialize)]
pub struct PatchMissionInput {
    title: Option<String>,
    terrain: Option<String>,
    custom_terrain_name: Option<String>,
    game_mode: Option<String>,
    weather: Option<String>,
    time_of_day: Option<String>,
    max_players: Option<i64>,
    briefing: Option<String>,
    thumbnail_url: Option<String>,
    status: Option<String>,
}

/// `PATCH /api/v1/missions/:id` — edit metadata (author/admin).
///
/// @route PATCH /api/v1/missions/:id
pub async fn update_mission(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    body: Result<Json<PatchMissionInput>, JsonRejection>,
) -> Result<Json<Mission>, ApiError> {
    let m = load(&state.pool, &id).await?;
    if !can_edit(&user, &m) {
        return Err(ApiError::forbidden("not your mission"));
    }
    let Json(input) = body.map_err(|_| ApiError::bad_request("invalid body"))?;

    let mut qb = QueryBuilder::new("UPDATE missions SET updated_at = now()");
    if let Some(t) = &input.title {
        qb.push(", title = ").push_bind(t.clone());
    }
    if let Some(t) = &input.terrain {
        let Some(terrain) = valid_terrain(t) else {
            return Err(ApiError::bad_request("invalid terrain"));
        };
        qb.push(", terrain = ").push_bind(terrain);
    }
    if let Some(c) = &input.custom_terrain_name {
        qb.push(", custom_terrain_name = ").push_bind(c.clone());
    }
    if let Some(g) = &input.game_mode {
        let Some(mode) = valid_game_mode(g) else {
            return Err(ApiError::bad_request("invalid game_mode"));
        };
        qb.push(", game_mode = ").push_bind(mode);
    }
    if let Some(w) = &input.weather {
        let Some(weather) = valid_weather(w) else {
            return Err(ApiError::bad_request("invalid weather"));
        };
        qb.push(", weather = ").push_bind(weather);
    }
    if let Some(t) = &input.time_of_day {
        qb.push(", time_of_day = ")
            .push_bind(t.clone())
            .push("::time");
    }
    if let Some(mp) = input.max_players {
        if !(1..=256).contains(&mp) {
            return Err(ApiError::bad_request(
                "max_players must be between 1 and 256",
            ));
        }
        qb.push(", max_players = ").push_bind(mp);
    }
    if let Some(b) = &input.briefing {
        qb.push(", briefing = ").push_bind(b.clone());
    }
    if let Some(t) = &input.thumbnail_url {
        qb.push(", thumbnail_url = ").push_bind(t.clone());
    }
    if let Some(target) = &input.status {
        apply_status_patch(&state.pool, &m, target, &mut qb).await?;
    }
    qb.push(" WHERE id = ").push_bind(m.id);
    qb.build()
        .execute(&state.pool)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(load(&state.pool, &id).await?))
}

/// Validate + push the only status changes PATCH may make (archive / unarchive).
async fn apply_status_patch(
    pool: &PgPool,
    m: &Mission,
    target: &str,
    qb: &mut QueryBuilder<Postgres>,
) -> Result<(), ApiError> {
    let status = match target {
        "archived" => MissionStatus::Archived,
        "draft" => MissionStatus::Draft,
        _ if m.status.as_wire() == target => return Ok(()), // idempotent no-op
        _ => {
            return Err(ApiError::bad_request(
                "status can only be changed to archived, or to draft to unarchive",
            ));
        }
    };
    if status == m.status {
        return Ok(());
    }
    match status {
        MissionStatus::Archived => {
            let upcoming: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM event_missions WHERE mission_id = $1 AND start_time > now()",
            )
            .bind(m.id)
            .fetch_one(pool)
            .await?;
            if upcoming > 0 {
                return Err(ApiError::conflict(
                    "mission is attached to an upcoming event — detach it there first",
                ));
            }
            qb.push(", status = 'archived'");
        }
        MissionStatus::Draft => {
            if m.status != MissionStatus::Archived {
                return Err(ApiError::conflict(
                    "only archived missions can be set back to draft",
                ));
            }
            qb.push(", status = 'draft'");
        }
        _ => {}
    }
    Ok(())
}

/// `DELETE /api/v1/missions/:id` — soft delete (author/admin), blocked if attached.
///
/// @route DELETE /api/v1/missions/:id
pub async fn delete_mission(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let m = load(&state.pool, &id).await?;
    if !can_edit(&user, &m) {
        return Err(ApiError::forbidden("not your mission"));
    }
    let attached: i64 =
        sqlx::query_scalar("SELECT count(*) FROM event_missions WHERE mission_id = $1")
            .bind(m.id)
            .fetch_one(&state.pool)
            .await?;
    if attached > 0 {
        return Err(ApiError::conflict(
            "mission is attached to an event — detach it (or archive the mission) instead",
        ));
    }
    sqlx::query("UPDATE missions SET deleted_at = now() WHERE id = $1")
        .bind(m.id)
        .execute(&state.pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/v1/missions/:id/submit` — draft/rejected → pending (author/admin).
///
/// @route POST /api/v1/missions/:id/submit
pub async fn submit_mission(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Mission>, ApiError> {
    let m = load(&state.pool, &id).await?;
    if !can_edit(&user, &m) {
        return Err(ApiError::forbidden("not your mission"));
    }
    if m.status != MissionStatus::Draft && m.status != MissionStatus::Rejected {
        return Err(ApiError::conflict(
            "only draft or rejected missions can be submitted",
        ));
    }
    sqlx::query(
        "UPDATE missions SET status = 'pending_approval', rejection_reason = '', updated_at = now() \
         WHERE id = $1",
    )
    .bind(m.id)
    .execute(&state.pool)
    .await?;
    Ok(Json(load(&state.pool, &id).await?))
}

// --- versions ---

#[derive(Debug, Deserialize)]
pub struct CreateVersionInput {
    #[serde(default)]
    semver: String,
    payload: Option<Box<RawValue>>,
    #[serde(default)]
    editor_notes: String,
}

/// `POST /api/v1/missions/:id/versions` — save a 2D-editor snapshot (author/admin).
///
/// @route POST /api/v1/missions/:id/versions
pub async fn create_version(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    body: Result<Json<CreateVersionInput>, JsonRejection>,
) -> Result<(StatusCode, Json<MissionVersion>), ApiError> {
    let m = load(&state.pool, &id).await?;
    if !can_edit(&user, &m) {
        return Err(ApiError::forbidden("not your mission"));
    }
    let Json(input) = body.map_err(|rej| {
        if rej.status() == StatusCode::PAYLOAD_TOO_LARGE {
            let mb = state.cfg.mission_version_body_limit() / (1 << 20);
            ApiError::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                format!("payload too large (max {mb} MB)"),
            )
        } else {
            ApiError::bad_request("semver and payload are required")
        }
    })?;
    let (Some(payload), false) = (&input.payload, input.semver.is_empty()) else {
        return Err(ApiError::bad_request("semver and payload are required"));
    };
    let payload_str = payload.get();
    validate_payload(payload_str)?;

    let version: Result<MissionVersion, sqlx::Error> = sqlx::query_as(
        "INSERT INTO mission_versions (mission_id, semver, json_payload, editor_notes, created_by, created_at) \
         VALUES ($1, $2, $3::jsonb, $4, $5, now()) RETURNING *",
    )
    .bind(m.id)
    .bind(&input.semver)
    .bind(payload_str)
    .bind(&input.editor_notes)
    .bind(&user.discord_id)
    .fetch_one(&state.pool)
    .await;
    let version = match version {
        Ok(v) => v,
        Err(e) if is_unique_violation(&e) => {
            return Err(ApiError::conflict("version already exists"));
        }
        Err(e) => return Err(e.into()),
    };
    sqlx::query("UPDATE missions SET current_version_id = $1 WHERE id = $2")
        .bind(version.id)
        .bind(m.id)
        .execute(&state.pool)
        .await?;
    Ok((StatusCode::CREATED, Json(version)))
}

/// `GET /api/v1/missions/:id/versions/:vid` — a specific version payload.
///
/// @route GET /api/v1/missions/:id/versions/:vid
pub async fn get_version(
    State(state): State<AppState>,
    user: AuthUser,
    Path((id, vid)): Path<(String, String)>,
) -> Result<Json<MissionVersion>, ApiError> {
    let m = load(&state.pool, &id).await?;
    if !can_view(&user, &m) {
        return Err(ApiError::not_found("mission not found"));
    }
    let Ok(vid) = Uuid::parse_str(&vid) else {
        return Err(ApiError::bad_request("invalid version id"));
    };
    let v: Option<MissionVersion> =
        sqlx::query_as("SELECT * FROM mission_versions WHERE id = $1 AND mission_id = $2")
            .bind(vid)
            .bind(m.id)
            .fetch_optional(&state.pool)
            .await?;
    v.map(Json)
        .ok_or_else(|| ApiError::not_found("version not found"))
}

// --- armory + bookmarks ---

/// `GET /api/v1/missions/:id/armory`.
///
/// @route GET /api/v1/missions/:id/armory
pub async fn get_armory(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let m = load(&state.pool, &id).await?;
    if !can_view(&user, &m) {
        return Err(ApiError::not_found("mission not found"));
    }
    let items: Vec<MissionArmory> = sqlx::query_as(
        "SELECT * FROM mission_armories WHERE mission_id = $1 ORDER BY sort_order ASC",
    )
    .bind(m.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({ "data": items })))
}

#[derive(Debug, Deserialize)]
pub struct ArmoryItemInput {
    #[serde(default)]
    faction: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    item_name: String,
    quantity: Option<i64>,
    #[serde(default)]
    icon: String,
    #[serde(default)]
    sort_order: i64,
}
#[derive(Debug, Deserialize)]
pub struct SetArmoryInput {
    #[serde(default)]
    items: Vec<ArmoryItemInput>,
}

/// `PUT /api/v1/missions/:id/armory` — replace the armory wholesale (author/admin).
///
/// @route PUT /api/v1/missions/:id/armory
pub async fn set_armory(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    body: Result<Json<SetArmoryInput>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let m = load(&state.pool, &id).await?;
    if !can_edit(&user, &m) {
        return Err(ApiError::forbidden("not your mission"));
    }
    let Json(input) = body.map_err(|_| ApiError::bad_request("invalid body"))?;

    let mut tx = state.pool.begin().await?;
    sqlx::query("DELETE FROM mission_armories WHERE mission_id = $1")
        .bind(m.id)
        .execute(&mut *tx)
        .await?;
    for it in &input.items {
        sqlx::query(
            "INSERT INTO mission_armories (mission_id, faction, category, item_name, quantity, icon, sort_order) \
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(m.id)
        .bind(&it.faction)
        .bind(&it.category)
        .bind(&it.item_name)
        .bind(it.quantity)
        .bind(&it.icon)
        .bind(it.sort_order)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    let items: Vec<MissionArmory> = sqlx::query_as(
        "SELECT * FROM mission_armories WHERE mission_id = $1 ORDER BY sort_order ASC",
    )
    .bind(m.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({ "data": items })))
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

// --- export + compiled ---

/// `GET /api/v1/missions/:id/export` — strict export envelope download (mission_maker+).
///
/// @route GET /api/v1/missions/:id/export
pub async fn export_mission(
    State(state): State<AppState>,
    maker: MissionMakerUser,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let m = load(&state.pool, &id).await?;
    if !can_view(&maker.0, &m) {
        return Err(ApiError::not_found("mission not found"));
    }
    let doc = build_mission_doc(&state.pool, &m).await?;
    let body = serde_json::to_vec_pretty(&doc)
        .map_err(|_| ApiError::internal("could not build mission export"))?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/json".to_string()),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"mission.json\"".to_string(),
            ),
        ],
        body,
    )
        .into_response())
}

/// `GET /api/v1/missions/:id/compiled` — the canonical mod document (service-token).
/// Runs the Phase 8 flatten engine live (gate G6 end-to-end).
///
/// @route GET /api/v1/missions/:id/compiled
pub async fn get_compiled_mission(
    State(state): State<AppState>,
    _svc: ServiceAuth,
    Path(id): Path<String>,
) -> Result<Json<ModMissionDocument>, ApiError> {
    let m = load(&state.pool, &id).await?;
    let Some(vid) = m.current_version_id else {
        return Err(ApiError::conflict("no saved version to compile"));
    };
    let v: Option<MissionVersion> = sqlx::query_as("SELECT * FROM mission_versions WHERE id = $1")
        .bind(vid)
        .fetch_optional(&state.pool)
        .await?;
    let Some(v) = v else {
        return Err(ApiError::conflict("no saved version to compile"));
    };
    match flatten_to_mod_document(&m, v.json_payload.0.get().as_bytes()) {
        Ok(doc) => Ok(Json(doc)),
        Err(CompileError::NoSlots) => Err(ApiError::conflict("no placed slots")),
        Err(CompileError::Parse(_)) => Err(ApiError::internal("could not compile mission")),
    }
}

// --- shared ---

#[derive(Debug, Serialize)]
struct ArmoryExport {
    faction: String,
    category: String,
    item: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    quantity: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MissionJson {
    export_format_version: i64,
    mission_id: String,
    title: String,
    terrain: String,
    game_mode: String,
    weather: String,
    time_of_day: String,
    max_players: i64,
    pub(crate) version: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    briefing: String,
    armory: Vec<ArmoryExport>,
    payload: Box<RawValue>,
    #[serde(with = "crate::models::serde_helpers::go_time")]
    exported_at: chrono::DateTime<Utc>,
}

/// Assemble the strict export envelope (shared by export + inject).
pub(crate) async fn build_mission_doc(pool: &PgPool, m: &Mission) -> Result<MissionJson, ApiError> {
    let (payload, version) = match m.current_version_id {
        Some(vid) => {
            let v: MissionVersion = sqlx::query_as("SELECT * FROM mission_versions WHERE id = $1")
                .bind(vid)
                .fetch_one(pool)
                .await
                .map_err(|_| ApiError::internal("could not build mission export"))?;
            (v.json_payload.0, v.semver)
        }
        None => (
            RawValue::from_string("{}".into()).unwrap(),
            "0.0.0".to_string(),
        ),
    };
    let armory: Vec<MissionArmory> = sqlx::query_as(
        "SELECT * FROM mission_armories WHERE mission_id = $1 ORDER BY sort_order ASC",
    )
    .bind(m.id)
    .fetch_all(pool)
    .await?;
    let export_armory = armory
        .into_iter()
        .map(|a| ArmoryExport {
            faction: a.faction,
            category: a.category,
            item: a.item_name,
            quantity: a.quantity,
        })
        .collect();
    let terrain = if m.terrain == TerrainType::Custom && !m.custom_terrain_name.is_empty() {
        m.custom_terrain_name.clone()
    } else {
        m.terrain.as_str().to_string()
    };
    Ok(MissionJson {
        export_format_version: 1,
        mission_id: m.id.to_string(),
        title: m.title.clone(),
        terrain,
        game_mode: m.game_mode_wire(),
        weather: m.weather.as_str().to_string(),
        time_of_day: m.time_of_day.clone(),
        max_players: m.max_players,
        version,
        briefing: m.briefing.clone(),
        armory: export_armory,
        payload,
        exported_at: Utc::now(),
    })
}

/// Parse `:id` and load the mission (404 on bad id or missing).
async fn load(pool: &PgPool, id: &str) -> Result<Mission, ApiError> {
    let Ok(id) = Uuid::parse_str(id) else {
        return Err(ApiError::bad_request("invalid id"));
    };
    load_mission(pool, id)
        .await?
        .ok_or_else(|| ApiError::not_found("mission not found"))
}

/// Validate a payload string against the editor schema (400 + details / 500).
fn validate_payload(payload: &str) -> Result<(), ApiError> {
    match validate_mission_editor_payload(payload.as_bytes()) {
        Ok(details) if details.is_empty() => Ok(()),
        Ok(details) => Err(ApiError::with_details(
            StatusCode::BAD_REQUEST,
            "invalid mission payload",
            json!(details),
        )),
        Err(_) => Err(ApiError::internal("payload validation unavailable")),
    }
}

// tiny wire-string helpers on the model enums used above.
impl Mission {
    fn game_mode_wire(&self) -> String {
        self.game_mode.as_str().to_string()
    }
}
impl MissionStatus {
    fn as_wire(self) -> &'static str {
        match self {
            MissionStatus::Draft => "draft",
            MissionStatus::PendingApproval => "pending_approval",
            MissionStatus::Live => "live",
            MissionStatus::Rejected => "rejected",
            MissionStatus::Archived => "archived",
        }
    }
}
