//! Mission library + editor handlers — Rust port of `handlers/missions.go` +
//! `handlers/missions_compiled.go`. The `/compiled` route runs the Phase 8 flatten
//! engine live (gate G6 end-to-end).

use std::collections::HashSet;

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

use map_engine_core::mission::flatten::scan_editor_payload_types;

use crate::contract::{validate_mission_document, validate_mission_editor_payload};
use crate::error::ApiError;
use crate::handlers::{is_unique_violation, load_mission, username};
use crate::middleware::{AuthUser, MissionMakerUser, ServiceAuth};
use crate::models::{
    AuditSeverity, GameMode, Mission, MissionArmory, MissionStatus, MissionVersion, TerrainType,
    WeatherType,
};
use crate::services::text::is_http_url;
use crate::services::{
    CompileError, ModMissionDocument, flatten_to_mod_document, mission_terrain_key, write_audit,
};
use crate::state::AppState;

/// `missions.thumbnail_url`, validated at the write boundary. **T-413**, adopting T-405 /
/// T-391's `is_http_url`.
///
/// Create hardcodes `thumbnail_url` to `''` and does not accept a body field — PATCH is the only
/// HTTP writer. The sink is an `<img src>` (`frontend/src/missions.rs`); same absent-guard class
/// as announcements before T-405.
fn validated_thumbnail_url(raw: &str) -> Result<String, ApiError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || is_http_url(trimmed) {
        return Ok(trimmed.to_string());
    }
    Err(ApiError::bad_request(
        "thumbnail_url must be an absolute http:// or https:// URL",
    ))
}

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

/// `HH:MM` or `HH:MM:SS` → the same string, bound verbatim to the `missions.time_of_day` `time`
/// column; `None` when it is not a clock this platform can round-trip.
///
/// ── Why this exists (T-367, from T-366's driven 500s) ────────────────────────────────────────
/// `time_of_day` reached `$N::time` with no validator of its own, so Postgres did the validating and
/// its rejection surfaced as **HTTP 500 `{"error":"internal error"}`**. Driven on the live path:
/// POST `"   "` / `"not-a-time"` / `"\t"` / `"25:00"` → 500 (POST's `is_empty()` guard is untrimmed,
/// so whitespace walks straight through it); PATCH had no guard at all, so `""` 500'd there too.
/// A caller cannot tell any of those from a genuine server fault.
///
/// ── Why it is NARROWER than the column, deliberately ────────────────────────────────────────
/// Measured against Postgres 18 directly: `time` also accepts `24:00`, `0600`, `4:05 PM`, `allballs`,
/// `06:00:00.5` and `06:00:60` (a leap second, silently normalised to `06:01:00`). Every one of those
/// would store fine and then be unreadable to the editor: the SPA's clock parser
/// (`eden_chrome::hhmm_to_minutes`) takes `HH:MM`/`HH:MM:SS` with `h <= 23`, `m <= 59`, `sec <= 59`,
/// and T-192 exists because a value that parser cannot read parks the time-of-day scrubber at the
/// 06:00 default **in silence** — an author who set 21:45 sees 06:00 after a reload. So "what the
/// column accepts" is the wrong bar; the right one is "what the platform can round-trip", and this
/// mirrors `hhmm_to_minutes` exactly so the two boundaries agree (T-346's lesson: the bug is
/// DISAGREEMENT between two sites). It is stricter in one place only — every component must be ASCII
/// digits, because Rust's `u32::from_str` accepts a leading `+` (`"+6:00"` would parse here and then
/// be rejected by Postgres, which is the 500 all over again).
///
/// Blast radius measured before tightening: all **87** live `missions` rows are plain `HH:MM:SS` with
/// zero sub-second components, and every producer that goes through this API emits `HH:MM` (the
/// create dialog, `RowMirror::set_time` via `normalize_clock`) or `HH:MM:SS` (the row hydrate
/// round-trip). The committed seeds `INSERT` directly and never touch this path. Nothing live is
/// rejected.
///
/// Returns the input UNCHANGED rather than a canonical form: this layer stores the author's bytes
/// verbatim, and normalising one side of a column two sites write is how T-346 happened. This
/// REJECTS; it does not repair.
fn valid_time_of_day(s: &str) -> Option<&str> {
    let mut parts = s.split(':');
    let h: u32 = digits(parts.next()?)?;
    let m: u32 = digits(parts.next()?)?;
    if let Some(sec) = parts.next()
        && digits(sec)? > 59
    {
        return None;
    }
    if parts.next().is_some() || h > 23 || m > 59 {
        return None;
    }
    Some(s)
}

/// One `HH`/`MM`/`SS` component: non-empty ASCII digits only. See [`valid_time_of_day`] on `+`.
fn digits(part: &str) -> Option<u32> {
    if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    part.parse().ok()
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
    let m = load(&state.pool, &id).await?;
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
    // An ABSENT/empty `time_of_day` keeps its documented default; a value that was SUPPLIED and is
    // not a clock is the author's mistake and is refused. Those are different facts and the split is
    // deliberate: treating `"   "` as "unspecified" would be the silent downgrade T-348 argued
    // against for an unrecognised cms status, and trimming it here would put a whitespace rule in a
    // second place (T-356 owns that, in Rust, at one site).
    let time_of_day = if input.time_of_day.is_empty() {
        "14:00".to_string()
    } else {
        let Some(t) = valid_time_of_day(&input.time_of_day) else {
            return Err(ApiError::bad_request(
                "invalid time_of_day (expected HH:MM or HH:MM:SS)",
            ));
        };
        t.to_string()
    };
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

/// `PATCH /api/v1/missions/:id` — edit metadata (mission_maker+ author, or admin).
///
/// **T-408 authz decision:** ownership does **not** outlive the role. `create_mission` already
/// requires [`MissionMakerUser`]; PATCH used to take a plain [`AuthUser`] and gate only on
/// [`can_edit`] (author-or-admin). That meant a demotion to `enlisted` left the former author able
/// to keep editing (including `thumbnail_url`). Safer default: the role that grants create is
/// still required to edit. Admins pass the extractor (`role_rank(admin) >= mission_maker`) and
/// still clear [`can_edit`] via the admin branch. Ownership-outlives-role is **not** the product
/// rule — demotion revokes edit.
///
/// @route PATCH /api/v1/missions/:id
pub async fn update_mission(
    State(state): State<AppState>,
    maker: MissionMakerUser,
    Path(id): Path<String>,
    body: Result<Json<PatchMissionInput>, JsonRejection>,
) -> Result<Json<Mission>, ApiError> {
    let user = &maker.0;
    let m = load(&state.pool, &id).await?;
    if !can_edit(user, &m) {
        return Err(ApiError::forbidden("not your mission"));
    }
    let Json(input) = body.map_err(|_| ApiError::bad_request("invalid body"))?;

    // **T-413.** Validated before the query builder so a rejected URL leaves every other field
    // untouched — PATCH is the only HTTP writer for this column (create hardcodes `''`).
    let thumbnail_url = input
        .thumbnail_url
        .as_deref()
        .map(validated_thumbnail_url)
        .transpose()?;

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
    // Unlike POST there is no default to fall back to — a PATCH naming the key is asking to SET it,
    // and `""` is not a clock. Every value this rejects answered 500 before T-367 (`""`, `"   "`,
    // `"not-a-time"` all did), so nothing that worked stops working.
    if let Some(t) = &input.time_of_day {
        let Some(t) = valid_time_of_day(t) else {
            return Err(ApiError::bad_request(
                "invalid time_of_day (expected HH:MM or HH:MM:SS)",
            ));
        };
        qb.push(", time_of_day = ")
            .push_bind(t.to_string())
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
    if let Some(t) = &thumbnail_url {
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
/// **The only writer of `pending_approval` in the crate.** `apply_status_patch` refuses the value
/// outright (driven: PATCH `{"status":"pending_approval"}` → 400), so `GET /approvals`
/// (`approvals.rs:99`) can only ever show rows this handler wrote. T-234 measured the consequence:
/// the SPA has **zero** callers of this route, so the admin queue is empty in production no matter
/// how many missions exist. The endpoint itself is correct and stays as the one door in; the caller
/// is the SPA's (`apps/website/frontend/src/missions.rs`, the `can_edit` "Manage" button row).
///
/// ── Authorisation, driven over HTTP against a live DB (T-234) ──────────────────────────────────
/// `can_edit` = author **or** admin, the same predicate PATCH and DELETE use. Measured: a
/// `mission_maker` who is not the author → **403 "not your mission"**; an admin who is not the
/// author → **200**, mission moves to `pending_approval`. The admin override is deliberate and
/// consistent with the rest of the file (an admin can already retitle, archive and delete any
/// mission), and `GET /approvals` is admin-only, so the reviewer tier is unchanged either way.
/// Accepted transitions are `draft` and `rejected`; `pending_approval` / `live` / `archived` all
/// answer 409, so a double submit cannot enqueue a mission twice.
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
    // `reviewed_by` / `reviewed_at` are cleared for the same reason `rejection_reason` always was:
    // this row is leaving the reviewed state, and a resubmission is a NEW review round. Before
    // T-234 only the reason was wiped, so a rejected-then-resubmitted mission sat in the queue
    // carrying the previous reviewer's stamp — driven: reject M, resubmit M, then
    // `GET /missions/{M}` served `status: pending_approval` **with** `reviewed_by` and
    // `reviewed_at` from the rejection, i.e. "already reviewed" on a mission awaiting review. Both
    // columns are `skip_serializing_if = "Option::is_none"` (`models/mission.rs:110-113`), so
    // NULLing them removes the fields exactly as they are absent on a never-reviewed mission — no
    // literal `null` and no wire-shape change for any other row.
    //
    // The `status IN (…)` predicate makes the guard above ATOMIC rather than advisory. The check
    // ran against a row loaded in an earlier statement, and the write named the id alone, so
    // anything that moved the mission in between was silently overwritten — most reachably
    // `apply_status_patch`, which accepts `archived` from *any* status, so a concurrent
    // PATCH-to-archived plus this UPDATE left a mission un-archived and queued. `deleted_at IS
    // NULL` mirrors `load_mission` (`handlers/mod.rs:83`) for the same reason: a concurrent soft
    // delete otherwise gets its status rewritten underneath it.
    let done = sqlx::query(
        "UPDATE missions SET status = 'pending_approval', rejection_reason = '', \
         reviewed_by = NULL, reviewed_at = NULL, updated_at = now() \
         WHERE id = $1 AND status IN ('draft', 'rejected') AND deleted_at IS NULL",
    )
    .bind(m.id)
    .execute(&state.pool)
    .await?;
    // Deliberately the SAME 409 the pre-check returns: every way to reach zero rows here means
    // "this mission is no longer submittable", and a second error string for a lost race would
    // only tell the client something it must handle identically (reload, look again).
    if done.rows_affected() == 0 {
        return Err(ApiError::conflict(
            "only draft or rejected missions can be submitted",
        ));
    }
    // The audit log is the ONLY durable record that a submission happened. The mission row has no
    // `submitted_by`/`submitted_at`; `GET /approvals` projects `updated_at` as `submitted_at`
    // (`approvals.rs:101`), and that column is bumped by every later PATCH — driven: PATCHing a
    // pending mission's title moved its queue `submitted_at` forward by the edit's timestamp. So
    // without this row, "who put this in my queue, and when" is unanswerable the moment the author
    // touches the mission again, and unrecoverable once a reviewer approves it. Both counterparts
    // (`mission.approve`, `mission.reject`) already audit; this was the transition that did not.
    //
    // `Info`, matching `mission.approve` — a submission is routine, and `Warn` is reserved for the
    // destructive half (`mission.reject`). The actor is the CALLER, not the author, because the two
    // differ whenever an admin submits on someone's behalf; the message names both in that case so
    // the entry cannot be misread as the author having submitted it themselves.
    let actor = &user.discord_id;
    let actor_name = username(&state.pool, actor).await;
    let message = if *actor == m.author_id {
        format!("{actor_name} submitted mission '{}' for approval", m.title)
    } else {
        let author_name = username(&state.pool, &m.author_id).await;
        format!(
            "{actor_name} submitted {author_name}'s mission '{}' for approval",
            m.title
        )
    };
    write_audit(
        &state.pool,
        AuditSeverity::Info,
        Some(actor),
        &actor_name,
        "mission.submit",
        &message,
        "mission",
        &m.id.to_string(),
    )
    .await;
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
         VALUES ($1, $2, $3::jsonb, $4, $5, now()) RETURNING id, mission_id, semver, json_payload, COALESCE(editor_notes, '') AS editor_notes, created_by, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at",
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
    // Library lists `ORDER BY updated_at DESC` (`list` above); approvals projects
    // `updated_at` as `submitted_at`. A bare `current_version_id` write left both clocks
    // frozen at create/PATCH time, so a save that only touched the editor payload never
    // bubbled the mission to the top of either surface. Bump in the same statement that
    // points `current_version_id` — same shape as `submit_mission`'s `updated_at = now()`.
    sqlx::query("UPDATE missions SET current_version_id = $1, updated_at = now() WHERE id = $2")
        .bind(version.id)
        .bind(m.id)
        .execute(&state.pool)
        .await?;
    // Mirror `mission.submit` / `mission.approve`: the audit row is the only durable
    // "who saved what, when" record. The version row itself has `created_by`, but nothing
    // indexes saves onto the mission timeline the admin audit log reads.
    let actor = &user.discord_id;
    let actor_name = username(&state.pool, actor).await;
    let message = if *actor == m.author_id {
        format!(
            "{actor_name} saved version {} of mission '{}'",
            input.semver, m.title
        )
    } else {
        let author_name = username(&state.pool, &m.author_id).await;
        format!(
            "{actor_name} saved version {} of {author_name}'s mission '{}'",
            input.semver, m.title
        )
    };
    write_audit(
        &state.pool,
        AuditSeverity::Info,
        Some(actor),
        &actor_name,
        "mission.version",
        &message,
        "mission",
        &m.id.to_string(),
    )
    .await;
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
        sqlx::query_as("SELECT id, mission_id, semver, json_payload, COALESCE(editor_notes, '') AS editor_notes, created_by, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at FROM mission_versions WHERE id = $1 AND mission_id = $2")
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
        "SELECT id, mission_id, faction, category, item_name, quantity, COALESCE(icon, '') AS icon, sort_order FROM mission_armories WHERE mission_id = $1 ORDER BY sort_order ASC",
    )
    .bind(m.id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(json!({ "data": items })))
}

/// One row of the replacement armory.
///
/// **`item_name` and `faction` are deliberately required — do not add `#[serde(default)]` to
/// either (T-315, T-346).** `category`/`icon`/`sort_order` keep their defaults on purpose: those
/// are presentation hints, matched by nothing, and an absent one degrades a row without making it
/// a lie. A row with `item_name: ""` renders in the faction dossier as a blank line the author
/// cannot identify, cannot select and cannot delete except by replacing the whole armory again.
///
/// Measured on the pre-fix binary: `{"items":[{}]}` answered **200** and left exactly that —
/// four real rows deleted, one nameless row inserted. That is the same mistake as `{}` one level
/// down, so fixing only the outer field would have left a trivial bypass of this very fix.
///
/// **T-346 — `faction` was grouped with the presentation hints above, and that was wrong.** It is
/// not a hint, it is the **join key** of the Event Hub dossier. [`get_event`] groups the armory by
/// it (`events.rs:796`) and the SPA then matches those groups against the mission's faction list
/// by *exact string equality* (`frontend/event_hub.rs:415`, `.find(|f| &f.faction == faction)`).
/// That list is built from a **different table** — `orbat_slots.faction` (`events.rs:894`) — so a
/// `faction` here that does not match one of those byte-for-byte renders a dossier card with **no
/// items at all**.
///
/// Measured on the pre-fix binary, against a mission whose ORBAT declares `USA`:
/// `{"items":[{"item_name":"M4A1"}]}` answered **200**, stored `faction: ""`, and the Event Hub's
/// USA card rendered **0** items — the author sees success and their own value echoed back, the
/// players see an empty armory. The `#[serde(default)]` made that reachable with **no whitespace
/// anywhere in the request**, which is why trimming alone would not have closed it.
#[derive(Debug, Deserialize)]
pub struct ArmoryItemInput {
    faction: String,
    #[serde(default)]
    category: String,
    item_name: String,
    quantity: Option<i64>,
    #[serde(default)]
    icon: String,
    #[serde(default)]
    sort_order: i64,
}

/// The armory replacement body.
///
/// **`items` is deliberately required — do not add `#[serde(default)]` to it (T-315).**
/// This is the fourth instance of one shape (T-185 roles, T-218 rejection reason, and this):
/// a defaulted field does not decode as "no data", it decodes as an affirmative *empty* value
/// and is then handed to a destructive write. Here the write is
/// `DELETE FROM mission_armories WHERE mission_id = $1`, run unconditionally before the inserts,
/// so `{}` deleted every armory row for the mission, inserted nothing, and answered **200** —
/// silent, total and unrecoverable, since the armory is not versioned with the mission.
///
/// An empty armory is still a legitimate request; it just has to be *stated*. `{"items":[]}`
/// means "clear the armory" and still succeeds. `{}` means the caller never mentioned the
/// armory at all, and now fails to decode, which the handler maps to 400.
#[derive(Debug, Deserialize)]
pub struct SetArmoryInput {
    items: Vec<ArmoryItemInput>,
}

/// `PUT /api/v1/missions/:id/armory` — replace the armory wholesale (author/admin).
///
/// Wholesale means the first statement in the transaction is an unconditional DELETE, so every
/// way this body can be wrong is a way to lose the armory. `{"items":[]}` clears it deliberately
/// and answers 200; a missing `items`, a blank `item_name`, a missing body, the wrong
/// `Content-Type` and malformed JSON all answer 400 with the rows untouched (T-315) — as do a
/// missing, blank or whitespace-padded `faction`, because it is the Event Hub's join key (T-346).
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
    // `map_err`, not `.ok().unwrap_or_default()` — the latter collapses a missing body, a wrong
    // `Content-Type` and malformed JSON into an empty armory and writes it. This handler already
    // had the guard; `items` losing `#[serde(default)]` above is what finally makes `{}` reach it.
    //
    // The message names every required field because all of them now fail here: `{}` misses
    // `items`, and `{"items":[{}]}` misses both a `faction` and an `item_name`. Naming only the
    // outer one sends the author of the second body looking for a field their request plainly has.
    let Json(input) = body.map_err(|_| {
        ApiError::bad_request("items is required, and every item needs a faction and an item_name")
    })?;
    // Validate every item BEFORE opening the transaction. The DELETE is the first statement in
    // it, so validating inside the loop would mean the armory is already gone by the time the
    // bad row is found — correct only because the transaction rolls back, and needlessly
    // load-bearing on that.
    //
    // `item_name` is a label, so a blank one is the same lie as no name and `trim` decides both
    // the rejection and the stored value; the two have to agree or `" "` is rejected while
    // `" M4 "` is stored with its padding.
    //
    // `faction` is a join key, so it is validated but **never rewritten** — see the note on
    // [`ArmoryItemInput`] and on its `bind` below.
    for (i, it) in input.items.iter().enumerate() {
        if it.item_name.trim().is_empty() {
            return Err(ApiError::bad_request(format!(
                "items[{i}].item_name is required"
            )));
        }
        if it.faction.trim().is_empty() {
            return Err(ApiError::bad_request(format!(
                "items[{i}].faction is required"
            )));
        }
        // Refused, not silently canonicalised. `"  USA  "` and `"USA"` are different factions to
        // every reader of this column, and this handler is not the one that gets to decide they
        // are the same — see the `bind` below for why trimming here would move the bug.
        if it.faction != it.faction.trim() {
            return Err(ApiError::bad_request(format!(
                "items[{i}].faction must not have leading or trailing whitespace"
            )));
        }
    }

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
        // **Verbatim, and it must stay verbatim (T-346).** The other side of this join,
        // `orbat_slots.faction`, is written with no normalisation at all: `events.rs:391` binds
        // `OrbatSquadTemplate.faction`, which is itself `#[serde(default)]` and untrimmed at
        // `crates/map-engine-core/src/mission/orbat.rs:23-25`, straight from the attach request.
        // Trimming here would therefore make the two sites *disagree* on a padded value instead of
        // agreeing. Measured on the pre-fix binary: an ORBAT declaring `"  USA  "` plus an armory
        // row `"  USA  "` renders correctly **today** (1 item on the card), and a unilateral trim
        // turns that into 0 — moving T-346's bug rather than fixing it, which is exactly the trap
        // T-343 flagged at `events.rs:1735` and `events.rs:1923`.
        //
        // The guard above is agreement-preserving under *either* hypothesis about the other side:
        // only a value already equal to its trimmed form is storable, and for such a value
        // verbatim and trimmed are the same bytes. Canonicalising a padded value here is not a
        // fix, it *is* the disagreement.
        .bind(&it.faction)
        .bind(&it.category)
        .bind(it.item_name.trim())
        .bind(it.quantity)
        .bind(&it.icon)
        .bind(it.sort_order)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    let items: Vec<MissionArmory> = sqlx::query_as(
        "SELECT id, mission_id, faction, category, item_name, quantity, COALESCE(icon, '') AS icon, sort_order FROM mission_armories WHERE mission_id = $1 ORDER BY sort_order ASC",
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

/// Cap on the schema findings echoed to the caller. Every constraint in
/// `mission.schema.json` under `slots[]` is per-slot, so one systematic defect on a
/// large mission yields one finding per slot — without a cap the error body can
/// dwarf the document it is complaining about. The full count always ships, and the
/// full list always reaches the server log.
const MAX_REPORTED_FINDINGS: usize = 20;

/// Serialize the compiled document and hold it to `mission.schema.json` before it can
/// reach the mod.
///
/// The document is the entire website↔mod interface (TBD_MOD_DESIGN §2, "JSON is the
/// contract"), and it is **generated** — the caller is a game server that supplied
/// nothing but an id and can do nothing about a violation. So a violation is a
/// server-side defect and answers **500**, not 4xx: it is either bad stored editor
/// data or a flatten bug, and this handler cannot tell those apart. Reporting the
/// latter as a client/state error would let a real compile regression hide as "your
/// mission is misconfigured".
///
/// Returns the validated bytes so the response body is byte-identical to what was
/// checked — re-serializing a validated value would leave a gap for the two to drift.
///
/// @contract mission.schema.json#/
fn validated_compiled_body(
    mission_id: &str,
    doc: &ModMissionDocument,
) -> Result<Vec<u8>, ApiError> {
    let body =
        serde_json::to_vec(doc).map_err(|_| ApiError::internal("could not compile mission"))?;

    let findings = validate_mission_document(&body).map_err(|e| {
        tracing::error!(mission = %mission_id, error = %e, "mission schema failed to compile");
        ApiError::internal("mission validation unavailable")
    })?;
    if findings.is_empty() {
        return Ok(body);
    }

    // The schema reaches `slots[]` through both the top-level `properties` and the
    // per-schemaVersion `allOf` branch, so every slot finding arrives twice — and a
    // systematic defect produces one pair per slot, so this has to stay O(n) (a
    // `Vec::contains` scan here is quadratic on a 100k-slot mission).
    let mut seen: HashSet<String> = HashSet::with_capacity(findings.len());
    let unique: Vec<String> = findings
        .into_iter()
        .filter(|f| seen.insert(f.clone()))
        .collect();

    // The mod's own error path (TBD_MissionLoader.OnBackendFetchError) discards the
    // response body and fails over to its cached copy, so this log line — not the
    // JSON below — is what an operator actually reads.
    tracing::error!(
        mission = %mission_id,
        findings = unique.len(),
        detail = %unique.join("; "),
        "compiled mission document violates mission.schema.json",
    );

    let shown: Vec<&String> = unique.iter().take(MAX_REPORTED_FINDINGS).collect();
    Err(ApiError::with_details(
        StatusCode::INTERNAL_SERVER_ERROR,
        "compiled mission failed schema validation",
        json!({
            "schema": "mission.schema.json",
            "findingCount": unique.len(),
            "findings": shown,
        }),
    ))
}

/// One row of `GET /api/v1/ingest/missions`.
///
/// The field names are **camelCase on purpose** and are NOT the usual snake_case API
/// contract: they are read by `TBD_MissionListEntry`
/// (`apps/mod/tbd-framework/Scripts/Game/TBD/Backend/TBD_MissionListLoader.c:3-9`), and
/// Enfusion's `JsonLoadContext` maps JSON keys onto class fields **by name**. A key the
/// struct does not declare is not an error there — it is simply invisible, so a
/// snake_case `slot_count` would parse to `0` for every mission with no warning
/// anywhere. Renamed explicitly rather than via a container attribute so the coupling is
/// visible on the field that has it (T-181.51).
#[derive(Debug, Serialize)]
pub struct IngestMissionListEntry {
    /// The mission UUID — the id the mod persists via `TBD_BackendConfig.SetMissionId`
    /// and then fetches at `GET /api/v1/missions/{id}/compiled`. NOT the compiled
    /// document's `meta.id`, which is a different (schema-shaped) id space.
    id: String,
    name: String,
    /// The compiled document's `meta.terrain`, from the one shared derivation — see
    /// [`map_engine_core::mission::flatten::mission_terrain_key`].
    terrain: String,
    #[serde(rename = "slotCount")]
    slot_count: i64,
}

/// `GET /api/v1/ingest/missions` — every runnable mission, for the in-game admin browser
/// (service-token tier).
///
/// ── WHY THIS IS NOT `list_missions` ────────────────────────────────────────────────────
/// [`list_missions`] is **owner-scoped**: it takes an [`AuthUser`] and every branch of
/// `push_filters` binds `me = &user.discord_id` (mine / bookmarked / live-or-my-drafts). A
/// service token is a game server, not a person — it has no "me", so that handler cannot
/// simply be re-tiered. This one applies no owner filter at all, which is the correct
/// answer for a machine that has to be able to run any mission an admin names.
///
/// `slotCount` is the count of PLACED editor slots in the mission's current version, read
/// in SQL (`jsonb_array_length`) rather than by compiling each mission — a compile per row
/// would parse every payload in the library on one request. It is the same array the
/// flatten walks, so `slotCount == 0` predicts exactly the `409 no placed slots` the mod
/// would get from `/compiled`, which is what `TBD_FrameworkManager.SelectMissionByNumber`
/// warns on.
///
/// @route GET /api/v1/ingest/missions
pub async fn ingest_list_missions(
    State(state): State<AppState>,
    _svc: ServiceAuth,
) -> Result<Json<Value>, ApiError> {
    // LEFT JOIN: a mission with no saved version still belongs in the list (the admin can
    // see it exists and that it has 0 slots) — an INNER JOIN would make it vanish silently.
    // The `jsonb_typeof` guard is not decoration: `jsonb_array_length` RAISES on a
    // non-array, which would turn one malformed payload into a 500 for the whole list.
    let rows: Vec<(Uuid, String, String, String, i32)> = sqlx::query_as(
        "SELECT m.id, m.title, m.terrain::text, COALESCE(m.custom_terrain_name, ''), \
                CASE WHEN jsonb_typeof(v.json_payload -> 'editor' -> 'slots') = 'array' \
                     THEN jsonb_array_length(v.json_payload -> 'editor' -> 'slots') \
                     ELSE 0 END \
         FROM missions m \
         LEFT JOIN mission_versions v ON v.id = m.current_version_id \
         WHERE m.deleted_at IS NULL \
         ORDER BY m.title ASC, m.id ASC",
    )
    .fetch_all(&state.pool)
    .await?;

    let missions: Vec<IngestMissionListEntry> = rows
        .into_iter()
        .map(
            |(id, title, terrain, custom, slots)| IngestMissionListEntry {
                id: id.to_string(),
                name: title,
                terrain: mission_terrain_key(&terrain, &custom),
                slot_count: i64::from(slots),
            },
        )
        .collect();

    let count = missions.len();
    Ok(Json(json!({ "missions": missions, "count": count })))
}

/// `GET /api/v1/missions/:id/compiled` — the canonical mod document (service-token).
/// Runs the Phase 8 flatten engine live (gate G6 end-to-end), then holds the result to
/// `mission.schema.json` before serving it — see [`validated_compiled_body`].
///
/// @route GET /api/v1/missions/:id/compiled
/// @contract mission.schema.json#/
pub async fn get_compiled_mission(
    State(state): State<AppState>,
    _svc: ServiceAuth,
    Path(id): Path<String>,
) -> Result<Response, ApiError> {
    let m = load(&state.pool, &id).await?;
    let Some(vid) = m.current_version_id else {
        return Err(ApiError::conflict("no saved version to compile"));
    };
    let v: Option<MissionVersion> = sqlx::query_as("SELECT id, mission_id, semver, json_payload, COALESCE(editor_notes, '') AS editor_notes, created_by, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at FROM mission_versions WHERE id = $1")
        .bind(vid)
        .fetch_optional(&state.pool)
        .await?;
    let Some(v) = v else {
        return Err(ApiError::conflict("no saved version to compile"));
    };
    let doc = match flatten_to_mod_document(&m, v.json_payload.0.get().as_bytes()) {
        Ok(doc) => doc,
        Err(CompileError::NoSlots) => return Err(ApiError::conflict("no placed slots")),
        Err(CompileError::Parse(detail)) => {
            return Err(unreadable_stored_payload(&id, &detail));
        }
    };
    let body = validated_compiled_body(&id, &doc)?;
    Ok(([(header::CONTENT_TYPE, "application/json")], body).into_response())
}

/// A stored payload the mission compiler cannot deserialise (`CompileError::Parse`).
///
/// **Still 500, deliberately** — the same argument [`validated_compiled_body`] makes: the caller is
/// a game server that supplied nothing but an id. What T-367 changed are the two things that were
/// actually wrong here:
///
/// 1. **The diagnosis was destroyed.** The arm read `Err(CompileError::Parse(_))` and dropped the
///    detail on the floor, so the only trace of a permanently-uncompilable mission was the access
///    log's bare `status=500`. The `serde` message names the offending key; it now reaches both the
///    log and the response body. Without it, "unrecoverable without hand-editing the stored JSON"
///    was literally true — nobody could see WHICH key was wrong.
/// 2. **The state was reachable.** Every byte in `mission_versions.json_payload` arrives through
///    [`validate_payload`], which since T-367 runs the compiler's own deserialiser, so a payload
///    that cannot parse is a 400 at save. This branch is no longer reachable through the API at all,
///    and reaching it now means one of exactly two things — the row was written around the API
///    (direct SQL, a restore, a seed), or the save-time precheck and this parse DISAGREE, which is a
///    defect in the precheck. Both deserve a 500. Answering 4xx would let that second case hide as
///    "your mission is misconfigured", the trap [`validated_compiled_body`] documents.
///
/// Recovery needs no hand-editing of stored JSON either way: saving a new version moves
/// `missions.current_version_id`, and the save boundary now guarantees the replacement compiles.
fn unreadable_stored_payload(mission_id: &str, detail: &str) -> ApiError {
    // The mod's error path (TBD_MissionLoader.OnBackendFetchError) discards the response body and
    // fails over to its cached copy, so this log line — not the JSON below — is what an operator
    // actually reads.
    tracing::error!(
        mission = %mission_id,
        detail = %detail,
        "stored mission payload does not deserialise into the editor graph the compiler reads",
    );
    ApiError::with_details(
        StatusCode::INTERNAL_SERVER_ERROR,
        "could not compile mission",
        json!({
            "reason": "stored payload does not match the editor graph the compiler reads",
            "detail": detail,
        }),
    )
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
            let v: MissionVersion = sqlx::query_as("SELECT id, mission_id, semver, json_payload, COALESCE(editor_notes, '') AS editor_notes, created_by, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at FROM mission_versions WHERE id = $1")
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
        "SELECT id, mission_id, faction, category, item_name, quantity, COALESCE(icon, '') AS icon, sort_order FROM mission_armories WHERE mission_id = $1 ORDER BY sort_order ASC",
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
///
/// This is the COMPLETE write boundary for `mission_versions.json_payload`: the two `INSERT`s in
/// this file (`create_mission`, `create_version`) are the only ones in the crate, and both go
/// through here. Two independent passes feed one `details` array, so the wire shape is unchanged:
///
/// * **`validate_mission_editor_payload`** — `mission-editor-payload.schema.json`, plus the
///   T-181.44 `wire_safety` walk it already carries.
/// * **`scan_editor_payload_types`** (T-367) — the mission compiler's OWN deserialiser, run here so
///   a shape it cannot read is a **400 at save**, in front of the author, instead of a **500 at
///   `GET /missions/:id/compiled`** in front of a game server that supplied nothing but an id. That
///   pass is not expressible in the payload schema without restating the compiler's structs in a
///   second language, which is precisely the drift that produced the bug — see
///   [`scan_editor_payload_types`] for the measurements and the argument.
///
/// The type pass reports nothing when the bytes are not JSON at all; the schema pass owns that
/// message ("payload is not valid JSON") and a second copy of it would be noise.
fn validate_payload(payload: &str) -> Result<(), ApiError> {
    let mut details = validate_mission_editor_payload(payload.as_bytes())
        .map_err(|_| ApiError::internal("payload validation unavailable"))?;
    details.extend(scan_editor_payload_types(payload.as_bytes()));
    if details.is_empty() {
        return Ok(());
    }
    Err(ApiError::with_details(
        StatusCode::BAD_REQUEST,
        "invalid mission payload",
        json!(details),
    ))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The `time_of_day` accept set, pinned against behaviour MEASURED on Postgres 18 rather than
    /// assumed — see [`valid_time_of_day`] for why the two columns of this table differ.
    ///
    /// The `false` rows split into two kinds, and both matter:
    ///
    /// * Postgres would REJECT them (`"   "`, `"not-a-time"`, `"25:00"`, `"06:60"`, `"+6:00"`, `""`).
    ///   Each was a live **500** before T-367; each is now a 400. `"+6:00"` is the one Rust's
    ///   `u32::from_str` would have let through on its own — it takes a leading `+`.
    /// * Postgres would ACCEPT them (`"24:00"`, `"0600"`, `"4:05 PM"`, `"allballs"`,
    ///   `"06:00:00.5"`, `"06:00:60"`). Those are refused on purpose: they store fine and are then
    ///   unreadable to `eden_chrome::hhmm_to_minutes`, which parks the author's scrubber back at the
    ///   06:00 default without saying anything. That is the T-192 bug, and letting one in through
    ///   this door would recreate it.
    #[test]
    fn time_of_day_accepts_the_clocks_the_platform_can_round_trip() {
        for (input, accepted) in [
            // What every producer on this path actually emits.
            ("06:00", true),
            ("06:00:00", true),
            ("6:00", true),
            ("23:59:59", true),
            ("00:00", true),
            ("21:45:00", true),
            // Postgres rejects these — each was a 500.
            ("", false),
            ("   ", false),
            ("\t", false),
            ("not-a-time", false),
            ("25:00", false),
            ("06:60", false),
            ("+6:00", false),
            (" 6:00", false),
            ("06:00:", false),
            ("06:00:00:00", false),
            // Postgres ACCEPTS these; the editor cannot read them back.
            ("24:00", false),
            ("0600", false),
            ("4:05 PM", false),
            ("allballs", false),
            ("06:00:00.5", false),
            ("06:00:60", false),
        ] {
            assert_eq!(
                valid_time_of_day(input).is_some(),
                accepted,
                "time_of_day {input:?}"
            );
        }
    }

    /// The value is stored as the author wrote it. This layer REJECTS; it does not repair — a
    /// one-sided normalisation of a column two sites write is how T-346 happened.
    #[test]
    fn an_accepted_time_of_day_is_returned_verbatim() {
        assert_eq!(valid_time_of_day("6:00"), Some("6:00"));
        assert_eq!(valid_time_of_day("06:00:00"), Some("06:00:00"));
    }

    /// T-408 Class-R: PATCH must require `MissionMakerUser`, same tier as create — demotion
    /// revokes edit. Ownership-outlives-role was the pre-fix omission and is deliberately
    /// rejected.
    ///
    /// RED: change `update_mission`'s extractor back to `user: AuthUser` — this pin fails.
    #[test]
    fn update_mission_requires_mission_maker_tier() {
        const SRC: &str = include_str!("missions.rs");
        let production = SRC
            .split("#[cfg(test)]")
            .next()
            .expect("missions.rs must have a #[cfg(test)] module");
        // Isolate the update_mission signature so a MissionMakerUser on create alone cannot
        // false-green this pin. Cut at the `) ->` return arrow, not the first `)` (State(state)).
        let start = production
            .find("pub async fn update_mission(")
            .expect("update_mission must exist in production source");
        let after = &production[start..];
        let end = after
            .find(") ->")
            .expect("update_mission must have a `) ->` return arrow");
        let sig = &after[..=end];
        assert!(
            sig.contains("maker: MissionMakerUser"),
            "update_mission must take MissionMakerUser (role still required to edit); got:\n{sig}"
        );
        assert!(
            !sig.contains("user: AuthUser"),
            "update_mission must not take bare AuthUser (that is the demotion-survives-edit bug); got:\n{sig}"
        );
    }
}
