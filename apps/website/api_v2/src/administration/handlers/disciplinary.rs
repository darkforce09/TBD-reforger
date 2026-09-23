//! Disciplinary actions against a member: bans, ban lifts, and warnings.

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::Json;
use chrono::Utc;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::administration::models::audit_log::AuditSeverity;
use crate::administration::models::warning::Warning;
use crate::administration::services::audit_writer::{actor_display_name, write_audit};
use crate::administration::services::required_audit::append_actor_audit_with_severity;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AdminUser;
use crate::identity_and_access::services::identity_ownership::lock_accounts;
use crate::operations::services::event_reservations::reevaluation_queue::request_reevaluation_for_account;

/// The ban body.
///
/// **`reason` is deliberately required — do not add `#[serde(default)]` to it.** A default is not
/// "no data": it decodes as an affirmative empty string and is bound straight into the `UPDATE`
/// below. That write is `ban_reason = $1` on a user who may already be banned, so the default
/// does not create a blank record — it *erases* the reason a previous admin wrote, and the same
/// statement overwrites `banned_by`/`banned_at`, so nothing survives to say what the ban was
/// originally for.
///
/// `Default` is deliberately not derived either: nothing in this crate needs a default
/// `BanInput`, and the derive would leave that clobber one keystroke away.
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
    // `map_err`, not `.ok()`: collapsing every extractor failure — a missing body, a wrong
    // `Content-Type`, malformed JSON — into `""` would write that empty string. On a re-ban the
    // row already holds the previous admin's reason, so the collapse is a silent delete, and
    // `banned_by`/`banned_at` go with it in the same statement. `map_err` is what the other ~25
    // handlers in this crate do.
    //
    // The `Content-Type` case is the one that actually bites: an admin who types a real reason
    // but whose client sends `text/plain` would get a 200 and a blank ban. They are told the ban
    // succeeded and never learn the reason was dropped.
    let Json(input) = body.map_err(|_| ApiError::bad_request("reason is required"))?;
    // A reason of spaces is the same lie as no reason. Trim once and use the trimmed value for
    // both the column and the audit message, so the two can never disagree.
    let reason = input.reason.trim();
    if reason.is_empty() {
        return Err(ApiError::bad_request("reason is required"));
    }
    let actor = &admin.0.discord_id;
    let now = Utc::now();
    // The ban, its credential revocation, the reservation re-evaluation request and the required
    // audit commit together; the event-first re-evaluation runs later.
    let mut tx = state.pool.begin().await?;
    lock_accounts(&mut tx, &[actor.clone(), discord_id.clone()]).await?;
    let res = sqlx::query(
        "UPDATE users SET is_banned = true, ban_reason = $1, banned_by = $2, banned_at = $3 WHERE discord_id = $4",
    )
    .bind(reason)
    .bind(actor)
    .bind(now)
    .bind(&discord_id)
    .execute(&mut *tx)
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
    .execute(&mut *tx)
    .await?;
    request_reevaluation_for_account(&mut tx, &discord_id).await?;
    let actor_name = transactional_display_name(&mut tx, actor).await?;
    let target_name = transactional_display_name(&mut tx, &discord_id).await?;
    append_actor_audit_with_severity(
        &mut tx,
        AuditSeverity::Warn,
        actor,
        "user.ban",
        "user",
        &discord_id,
        &format!("{actor_name} permanently banned user '{target_name}'. Reason: '{reason}'"),
    )
    .await?;
    tx.commit().await?;
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
    let actor = &admin.0.discord_id;
    let mut tx = state.pool.begin().await?;
    lock_accounts(&mut tx, &[actor.clone(), discord_id.clone()]).await?;
    let res = sqlx::query(
        "UPDATE users SET is_banned = false, ban_reason = '', banned_by = NULL, banned_at = NULL WHERE discord_id = $1",
    )
    .bind(&discord_id)
    .execute(&mut *tx)
    .await?;
    if res.rows_affected() == 0 {
        return Err(ApiError::not_found("user not found"));
    }
    // Waiting entries of the restored account may become promotable again.
    request_reevaluation_for_account(&mut tx, &discord_id).await?;
    let actor_name = transactional_display_name(&mut tx, actor).await?;
    let target_name = transactional_display_name(&mut tx, &discord_id).await?;
    append_actor_audit_with_severity(
        &mut tx,
        AuditSeverity::Info,
        actor,
        "user.unban",
        "user",
        &discord_id,
        &format!("{actor_name} unbanned user '{target_name}'"),
    )
    .await?;
    tx.commit().await?;
    Ok(Json(json!({ "banned": false })))
}

/// The display name audit messages use, read inside the business transaction.
async fn transactional_display_name(
    connection: &mut sqlx::PgConnection,
    discord_id: &str,
) -> Result<String, ApiError> {
    let name: Option<String> =
        sqlx::query_scalar("SELECT COALESCE(username, '') FROM users WHERE discord_id = $1")
            .bind(discord_id)
            .fetch_optional(connection)
            .await?;
    Ok(match name {
        Some(name) if !name.trim().is_empty() => name,
        _ => discord_id.to_owned(),
    })
}

/// The warning body.
///
/// **`reason` is deliberately required — do not add `#[serde(default)]` to it.** A default
/// decodes as an affirmative empty string rather than as absence, which makes `{}` and
/// `{"reason":""}` indistinguishable to this handler and leaves the clobber [`BanInput`]
/// documents one deleted guard away. `map_err` on the extractor returns the same 400 with the
/// same message for a missing field, so requiring it changes nothing on the wire.
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
) -> Result<(StatusCode, Json<Warning>), ApiError> {
    // The guard is `trim()`-based, so `{"reason":"   "}` is rejected rather than stored. The
    // blast radius of a blank warning is smaller than a blank ban's and worth stating precisely
    // rather than inheriting by analogy: this is an `INSERT`, not an `UPDATE`, so nothing is
    // clobbered. What it costs is a row that counts against the member forever. The Personnel
    // Roster's warning tally is `SELECT count(*) FROM warnings` with no predicate on `reason`,
    // and the SPA reds the cell at `> 0`, so a blank warning marks someone as disciplined while
    // recording nothing anyone can read back — no endpoint in this crate returns
    // `warnings.reason` at all; the `RETURNING` clause below is its only reader.
    //
    // Trim once and use the trimmed value for the column *and* the audit message, exactly as
    // `ban_user` does, so the two can never disagree. That also keeps this column normalised the
    // same way `users.ban_reason` is — two columns holding the same kind of operator-authored
    // text.
    //
    // The message is "reason is required", the wording `ban_user` uses, `reject_mission` in
    // `missions/handlers/approvals_queue.rs` uses, and the SPA already shows the operator. A client
    // matching on error text would be surprised by a difference.
    let Json(input) = body.map_err(|_| ApiError::bad_request("reason is required"))?;
    let reason = input.reason.trim();
    if reason.is_empty() {
        return Err(ApiError::bad_request("reason is required"));
    }
    // Existence first — `actor_display_name` falls through to `discord_id` when the row is
    // missing (or blank), so it cannot be the not-found check. Ban/role_change use
    // `rows_affected` on their UPDATE; warn is an INSERT, so we ask EXISTS.
    //
    // `target_name` goes through `actor_display_name`, not a hand-rolled COALESCE: a duplicated
    // SELECT without the `discord_id` fallback logs `"… warned '': …"` for a member whose
    // `username` is blank, while every sibling action (ban/unban/role_change) names the target by
    // id. Calling the shared helper lands its display decision on both halves of one audit line.
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM users WHERE discord_id = $1)")
            .bind(&discord_id)
            .fetch_one(&state.pool)
            .await?;
    if !exists {
        return Err(ApiError::not_found("user not found"));
    }
    let actor = &admin.0.discord_id;
    let warning: Warning = sqlx::query_as(
        "INSERT INTO warnings (discord_id, issued_by, reason, created_at) VALUES ($1, $2, $3, now()) RETURNING id, discord_id, issued_by, reason, COALESCE(created_at, '0001-01-01 00:00:00+00'::timestamptz) AS created_at",
    )
    .bind(&discord_id)
    .bind(actor)
    .bind(reason)
    .fetch_one(&state.pool)
    .await?;
    let actor_name = actor_display_name(&state.pool, actor).await;
    let target_name = actor_display_name(&state.pool, &discord_id).await;
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
