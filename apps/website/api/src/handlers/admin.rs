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
    // T-343 sweep — correct as-is, recorded so the next sweep does not re-litigate it: the
    // trim and the emptiness test are on the same expression, and the trimmed value is what
    // `push_search` binds, so the guard and the bind cannot disagree. Worst case for a
    // whitespace-only `?q=` is a no-op filter, not a write.
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

/// The role-change body.
///
/// **`role` is deliberately required — do not add `#[serde(default)]` to it (T-343).**
/// It carried one until this ticket. Nothing bad reached the column: the guard below rejects
/// `""` and `valid_role` rejects it a second time, so the default was *masked*. But masked is
/// not absent — this is the T-185 shape (a defaulted `roles` vec decoded as an affirmative
/// "no roles" and demoted admins to enlisted) sitting one guard away from a write that sets a
/// privilege level. The `map_err` below already returns the identical 400 with the identical
/// message for a missing field, so dropping the default is invisible on the wire.
#[derive(Debug, Deserialize)]
pub struct UpdateUserInput {
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
    // T-343 sweep — this `is_empty()` is deliberately NOT `trim().is_empty()`, and `role` is
    // deliberately not trimmed before `valid_role`. This guard fails *closed*: `valid_role` is
    // an exact match over four literals, so `"  admin  "` is rejected with 400 "invalid role"
    // and the value bound into the `UPDATE` is the `UserRole` enum, never the request string.
    // There is therefore no untrimmed write here and no reachable bad state — only a debatable
    // error message. Trimming would *loosen* what the endpoint accepts, which is a product
    // decision rather than a bug fix, so it is not taken unilaterally. Counter-precedent worth
    // knowing if that call is ever revisited: `telemetry.rs:345` does `match m.outcome.trim()`
    // before its enum match, i.e. the crate is not unanimous on this.
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

/// The ban body.
///
/// **`reason` is deliberately required — do not add `#[serde(default)]` to it (T-317).**
/// This is the same shape T-218 fixed on `reject_mission`: a defaulted field is not "no
/// data", it decodes as an affirmative empty string and gets bound straight into an
/// `UPDATE`. Here the write is `ban_reason = $1` on a user who may already be banned, so
/// the default does not create a blank record — it *erases* the reason a previous admin
/// wrote. Measured pre-fix on a row holding a real reason: `POST {}` returned **200** and
/// left `ban_reason` as `''`.
///
/// `Default` is deliberately not derived either. The only thing that ever constructed a
/// default `BanInput` was the `unwrap_or_default()` this ticket deleted; leaving the derive
/// in place would leave the clobber one keystroke away from returning.
#[derive(Debug, Deserialize)]
pub struct BanInput {
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
    // `.ok()` here collapsed *every* extractor failure — a missing body, a wrong
    // `Content-Type`, malformed JSON — into `""` and wrote it. That is worse than it looks
    // on a re-ban: the row already held the previous admin's reason, so the collapse was a
    // silent delete, and `banned_by`/`banned_at` were overwritten in the same statement, so
    // nothing survived to say what the ban had originally been for. `map_err` is what the
    // other ~25 handlers in this crate do; this one was the outlier (T-317).
    //
    // The `Content-Type` case is the one that actually bites: an admin who types a real
    // reason but whose client sends `text/plain` got a 200 and a blank ban. They are told
    // the ban succeeded and never learn the reason was dropped.
    let Json(input) = body.map_err(|_| ApiError::bad_request("reason is required"))?;
    // A reason of spaces is the same lie as no reason. Trim once and use the trimmed value
    // for both the column and the audit message, so the two can never disagree.
    let reason = input.reason.trim();
    if reason.is_empty() {
        return Err(ApiError::bad_request("reason is required"));
    }
    let actor = &admin.0.discord_id;
    let now = Utc::now();
    let res = sqlx::query(
        "UPDATE users SET is_banned = true, ban_reason = $1, banned_by = $2, banned_at = $3 WHERE discord_id = $4",
    )
    .bind(reason)
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

/// The warning body.
///
/// **`reason` is deliberately required — do not add `#[serde(default)]` to it (T-343).**
/// It carried one until this ticket, for the same reason [`BanInput`] did before T-317: the
/// default decodes as an affirmative empty string rather than as absence. Here the guard
/// downstream caught it, so `POST {}` already 400'd — but the default made `{}` and
/// `{"reason":""}` indistinguishable, and left the T-185/T-317 clobber one deleted guard away.
/// `map_err` on the extractor returns the same 400 with the same message for a missing field,
/// so removing it changes nothing on the wire.
#[derive(Debug, Deserialize)]
pub struct WarnInput {
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
    // T-343. This guard was `input.reason.is_empty()` — untrimmed — so `{"reason":"   "}` was
    // accepted. Measured pre-fix: **HTTP 201**, `warnings.reason` = `'   '` (length 3), and an
    // audit line reading `Dev Operator warned 'Warn Target 343':    `.
    //
    // The blast radius is smaller than T-317's and worth stating precisely rather than
    // inheriting by analogy: this is an `INSERT`, not an `UPDATE`, so nothing is clobbered — a
    // pre-existing sentinel warning on the same user survived every bad request intact. What it
    // does cost is a row that counts against the user forever. The Personnel roster's warning
    // tally is `SELECT count(*) FROM warnings` (see `list_users` above) with no predicate on
    // `reason`, and the SPA reds the cell at `> 0`, so a blank warning marks someone as
    // disciplined while recording nothing anyone can read back — no endpoint in this crate
    // returns `warnings.reason` at all; the `RETURNING` clause below is its only reader.
    //
    // Trim once and use the trimmed value for the column *and* the audit message, exactly as
    // `ban_user` does, so the two can never disagree. That also closes a second defect the
    // untrimmed guard was hiding: pre-fix a *valid* `"  arrived late to muster  "` was stored
    // with its padding (26 bytes) where `users.ban_reason` would have stored 22 — two columns
    // holding the same kind of operator-authored text, normalised differently.
    //
    // The message is standardised to "reason is required" (it was "reason required"). That
    // wording is what `ban_user` one handler up uses, what `reject_mission` in `approvals.rs`
    // uses, and what the SPA already tells the operator in `frontend/src/personnel.rs` and
    // `frontend/src/approvals.rs`. This handler was the lone outlier, and a client matching on
    // error text would have been surprised by the difference.
    let Json(input) = body.map_err(|_| ApiError::bad_request("reason is required"))?;
    let reason = input.reason.trim();
    if reason.is_empty() {
        return Err(ApiError::bad_request("reason is required"));
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
    .bind(reason)
    .fetch_one(&state.pool)
    .await?;
    let actor_name = username(&state.pool, actor).await;
    write_audit(
        &state.pool,
        AuditSeverity::Warn,
        Some(actor),
        &actor_name,
        "user.warn",
        &format!("{actor_name} warned '{target_name}': {reason}"),
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

/// The RCON body.
///
/// `action` is required, so it carries no `#[serde(default)]` (T-343) — same reasoning as
/// [`UpdateUserInput`]. `map` and `command` keep theirs **deliberately**: they are genuinely
/// optional (only `change_map` reads `map`, and `command` is reserved), so for those two the
/// default really does mean "not supplied" rather than an affirmative empty answer. That is the
/// distinction the T-185/T-317/T-343 rule turns on, and it is why this sweep does not simply
/// delete every `#[serde(default)]` in the file.
#[derive(Debug, Deserialize)]
pub struct RconInput {
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
    // T-343 sweep — deliberately left untrimmed, same argument as `update_user`'s `role`: the
    // `matches!` immediately below is an exact-set test, so a whitespace-padded action fails
    // closed with 400 "unknown action", and it doubles as normalisation — anything that reaches
    // the audit line below is guaranteed to be one of the four literals, never request bytes.
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
    // T-343. `map` is the one field in this handler whose raw request bytes reach a persisted
    // string, and its guard was `!input.map.is_empty()` — untrimmed. `{"action":"change_map",
    // "map":"   "}` therefore wrote an audit row reading `issued RCON 'change_map ->    '`: a
    // recorded map change naming no map. `audit_logs.message` is a write like any other, so it
    // gets the same treatment as `warnings.reason` above — trim once, test and emit the same
    // value. A whitespace-only `map` now degrades to the bare `change_map` line rather than
    // inventing a destination.
    let map = input.map.trim();
    if input.action == "change_map" && !map.is_empty() {
        detail = format!("{detail} -> {map}");
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
