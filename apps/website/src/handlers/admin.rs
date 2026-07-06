//! Admin — personnel roster, role/ban/warning management, role resync, RCON.
//! Rust port of `handlers/admin.go`. All routes are admin-tier.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::Json;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::{Postgres, QueryBuilder};

use crate::error::ApiError;
use crate::handlers::{PageParams, username};
use crate::middleware::AdminUser;
use crate::models::{AuditSeverity, UserRole};
use crate::services::{resync_all_roles, write_audit};
use crate::state::AppState;

fn valid_role(s: &str) -> Option<UserRole> {
    match s {
        "enlisted" => Some(UserRole::Enlisted),
        "leader" => Some(UserRole::Leader),
        "mission_maker" => Some(UserRole::MissionMaker),
        "admin" => Some(UserRole::Admin),
        _ => None,
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
struct RosterRow {
    discord_id: String,
    username: String,
    discord_handle: String,
    arma_id: Option<String>,
    arma_character: String,
    role: UserRole,
    is_banned: bool,
    warnings: i64,
}

#[derive(Debug, Deserialize)]
pub struct RosterQuery {
    q: Option<String>,
    limit: Option<i64>,
    offset: Option<i64>,
}

/// `GET /api/v1/admin/users` — Personnel Roster + per-user warning counts.
///
/// @route GET /api/v1/admin/users
pub async fn list_users(
    State(state): State<AppState>,
    _a: AdminUser,
    Query(f): Query<RosterQuery>,
) -> Result<Json<Value>, ApiError> {
    let (limit, offset) = PageParams {
        limit: f.limit,
        offset: f.offset,
    }
    .bounds();
    let search = f.q.as_deref().map(str::trim).filter(|s| !s.is_empty());

    let mut cq: QueryBuilder<Postgres> = QueryBuilder::new("SELECT count(*) FROM users WHERE true");
    if let Some(s) = search {
        push_search(&mut cq, s);
    }
    let total: i64 = cq
        .build_query_scalar()
        .fetch_one(&state.pool)
        .await
        .map_err(ApiError::from)?;

    let mut sq: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT discord_id, COALESCE(username, '') AS username, COALESCE(discord_handle, '') AS discord_handle, \
         arma_id, COALESCE(arma_character, '') AS arma_character, role, is_banned, \
         (SELECT count(*) FROM warnings w WHERE w.discord_id = users.discord_id) AS warnings \
         FROM users WHERE true",
    );
    if let Some(s) = search {
        push_search(&mut sq, s);
    }
    sq.push(" ORDER BY username ASC LIMIT ")
        .push_bind(limit)
        .push(" OFFSET ")
        .push_bind(offset);
    let rows: Vec<RosterRow> = sq
        .build_query_as()
        .fetch_all(&state.pool)
        .await
        .map_err(ApiError::from)?;

    Ok(Json(
        json!({ "data": rows, "total": total, "limit": limit, "offset": offset }),
    ))
}

fn push_search(qb: &mut QueryBuilder<Postgres>, s: &str) {
    let like = format!("%{s}%");
    qb.push(" AND (username ILIKE ").push_bind(like.clone());
    qb.push(" OR discord_handle ILIKE ").push_bind(like.clone());
    qb.push(" OR arma_character ILIKE ").push_bind(like.clone());
    qb.push(" OR arma_id ILIKE ").push_bind(like).push(")");
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserInput {
    #[serde(default)]
    role: String,
}

/// `PATCH /api/v1/admin/users/:discordId` — set a user's web role.
///
/// @route PATCH /api/v1/admin/users/:discordId
pub async fn update_user(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(discord_id): Path<String>,
    body: Result<Json<UpdateUserInput>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let Json(input) = body.map_err(|_| ApiError::bad_request("role required"))?;
    if input.role.is_empty() {
        return Err(ApiError::bad_request("role required"));
    }
    let Some(role) = valid_role(&input.role) else {
        return Err(ApiError::bad_request("invalid role"));
    };
    let res = sqlx::query("UPDATE users SET role = $1 WHERE discord_id = $2")
        .bind(role)
        .bind(&discord_id)
        .execute(&state.pool)
        .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::not_found("user not found"));
    }
    let actor = &admin.0.discord_id;
    let actor_name = username(&state.pool, actor).await;
    let target_name = username(&state.pool, &discord_id).await;
    write_audit(
        &state.pool,
        AuditSeverity::Info,
        Some(actor),
        &actor_name,
        "user.role_change",
        &format!("{actor_name} set {target_name} role to {}", role.as_str()),
        "user",
        &discord_id,
    )
    .await;
    Ok(Json(json!({ "discord_id": discord_id, "role": role })))
}

#[derive(Debug, Deserialize, Default)]
pub struct BanInput {
    #[serde(default)]
    reason: String,
}

/// `POST /api/v1/admin/users/:discordId/ban` — ban + revoke tokens.
///
/// @route POST /api/v1/admin/users/:discordId/ban
pub async fn ban_user(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(discord_id): Path<String>,
    body: Result<Json<BanInput>, JsonRejection>,
) -> Result<Json<Value>, ApiError> {
    let reason = body.ok().map(|Json(b)| b.reason).unwrap_or_default();
    let actor = &admin.0.discord_id;
    let now = Utc::now();
    let res = sqlx::query(
        "UPDATE users SET is_banned = true, ban_reason = $1, banned_by = $2, banned_at = $3 WHERE discord_id = $4",
    )
    .bind(&reason)
    .bind(actor)
    .bind(now)
    .bind(&discord_id)
    .execute(&state.pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::not_found("user not found"));
    }
    // Revoke active refresh tokens so the ban takes hold once the access token expires.
    sqlx::query(
        "UPDATE refresh_tokens SET revoked_at = $1 WHERE discord_id = $2 AND revoked_at IS NULL",
    )
    .bind(now)
    .bind(&discord_id)
    .execute(&state.pool)
    .await?;
    let actor_name = username(&state.pool, actor).await;
    let target_name = username(&state.pool, &discord_id).await;
    write_audit(
        &state.pool,
        AuditSeverity::Warn,
        Some(actor),
        &actor_name,
        "user.ban",
        &format!("{actor_name} permanently banned user '{target_name}'. Reason: '{reason}'"),
        "user",
        &discord_id,
    )
    .await;
    Ok(Json(json!({ "banned": true })))
}

/// `DELETE /api/v1/admin/users/:discordId/ban` — lift a ban.
///
/// @route DELETE /api/v1/admin/users/:discordId/ban
pub async fn unban_user(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(discord_id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let res = sqlx::query(
        "UPDATE users SET is_banned = false, ban_reason = '', banned_by = NULL, banned_at = NULL WHERE discord_id = $1",
    )
    .bind(&discord_id)
    .execute(&state.pool)
    .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::not_found("user not found"));
    }
    let actor = &admin.0.discord_id;
    let actor_name = username(&state.pool, actor).await;
    let target_name = username(&state.pool, &discord_id).await;
    write_audit(
        &state.pool,
        AuditSeverity::Info,
        Some(actor),
        &actor_name,
        "user.unban",
        &format!("{actor_name} unbanned user '{target_name}'"),
        "user",
        &discord_id,
    )
    .await;
    Ok(Json(json!({ "banned": false })))
}

#[derive(Debug, Deserialize)]
pub struct WarnInput {
    #[serde(default)]
    reason: String,
}

/// `POST /api/v1/admin/users/:discordId/warnings` — record a disciplinary warning.
///
/// @route POST /api/v1/admin/users/:discordId/warnings
pub async fn issue_warning(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(discord_id): Path<String>,
    body: Result<Json<WarnInput>, JsonRejection>,
) -> Result<(StatusCode, Json<crate::models::Warning>), ApiError> {
    let Json(input) = body.map_err(|_| ApiError::bad_request("reason required"))?;
    if input.reason.is_empty() {
        return Err(ApiError::bad_request("reason required"));
    }
    let target_name: Option<String> =
        sqlx::query_scalar("SELECT COALESCE(username, '') FROM users WHERE discord_id = $1")
            .bind(&discord_id)
            .fetch_optional(&state.pool)
            .await?;
    let Some(target_name) = target_name else {
        return Err(ApiError::not_found("user not found"));
    };
    let actor = &admin.0.discord_id;
    let warning: crate::models::Warning = sqlx::query_as(
        "INSERT INTO warnings (discord_id, issued_by, reason, created_at) VALUES ($1, $2, $3, now()) RETURNING id, discord_id, issued_by, reason, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at",
    )
    .bind(&discord_id)
    .bind(actor)
    .bind(&input.reason)
    .fetch_one(&state.pool)
    .await?;
    let actor_name = username(&state.pool, actor).await;
    write_audit(
        &state.pool,
        AuditSeverity::Warn,
        Some(actor),
        &actor_name,
        "user.warn",
        &format!("{actor_name} warned '{target_name}': {}", input.reason),
        "user",
        &discord_id,
    )
    .await;
    Ok((StatusCode::CREATED, Json(warning)))
}

/// `POST /api/v1/admin/roles/sync` — re-apply discord_roles mappings.
///
/// @route POST /api/v1/admin/roles/sync
pub async fn resync_roles(
    State(state): State<AppState>,
    admin: AdminUser,
) -> Result<Json<Value>, ApiError> {
    let updated = resync_all_roles(&state.pool)
        .await
        .map_err(|_| ApiError::internal("resync failed"))?;
    let actor = &admin.0.discord_id;
    let actor_name = username(&state.pool, actor).await;
    write_audit(
        &state.pool,
        AuditSeverity::Info,
        Some(actor),
        &actor_name,
        "roles.resync",
        &format!("{actor_name} triggered a role resync"),
        "system",
        "",
    )
    .await;
    Ok(Json(json!({ "updated": updated })))
}

#[derive(Debug, Deserialize)]
pub struct RconInput {
    #[serde(default)]
    action: String,
    #[serde(default)]
    map: String,
    #[serde(default)]
    command: String,
}

/// `POST /api/v1/admin/servers/:id/rcon` — validate + audit an RCON command.
///
/// @route POST /api/v1/admin/servers/:id/rcon
pub async fn send_rcon(
    State(state): State<AppState>,
    admin: AdminUser,
    Path(id): Path<String>,
    body: Result<Json<RconInput>, JsonRejection>,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let Json(input) = body.map_err(|_| ApiError::bad_request("action required"))?;
    if input.action.is_empty() {
        return Err(ApiError::bad_request("action required"));
    }
    if !matches!(
        input.action.as_str(),
        "restart" | "change_map" | "kick" | "custom"
    ) {
        return Err(ApiError::bad_request("unknown action"));
    }
    let Ok(server_id) = uuid::Uuid::parse_str(&id) else {
        return Err(ApiError::not_found("server not found"));
    };
    let srv_name: Option<String> = sqlx::query_scalar("SELECT name FROM servers WHERE id = $1")
        .bind(server_id)
        .fetch_optional(&state.pool)
        .await?;
    let Some(srv_name) = srv_name else {
        return Err(ApiError::not_found("server not found"));
    };
    let mut detail = input.action.clone();
    if input.action == "change_map" && !input.map.is_empty() {
        detail = format!("{detail} -> {}", input.map);
    }
    let _ = &input.command; // reserved for the custom-command bridge (audited via action)
    let actor = &admin.0.discord_id;
    let actor_name = username(&state.pool, actor).await;
    write_audit(
        &state.pool,
        AuditSeverity::Info,
        Some(actor),
        &actor_name,
        "server.rcon",
        &format!("{actor_name} issued RCON '{detail}' on {srv_name}"),
        "server",
        &id,
    )
    .await;
    Ok((
        StatusCode::ACCEPTED,
        Json(json!({ "accepted": true, "action": input.action })),
    ))
}
