//! Audited, expiring recovery for verified Discord permissions during synchronization outages.

use super::account_authority::{load_account_authority, lock_account};
use crate::administration::services::required_audit::append_required_audit;
use crate::core::application_state::AppState;
use crate::core::error_handling::api_error::ApiError;
use crate::core::middleware::AuthUser;
use chrono::{DateTime, Duration, Utc};

pub async fn extend_membership_grace(
    state: &AppState,
    actor: &AuthUser,
    target: &str,
    duration_hours: i64,
    reason: &str,
) -> Result<DateTime<Utc>, ApiError> {
    if !(1..=48).contains(&duration_hours) {
        return Err(ApiError::bad_request(
            "duration_hours must be an integer from 1 to 48",
        ));
    }
    let reason = reason.trim();
    if reason.is_empty() || reason.len() > 2000 {
        return Err(ApiError::bad_request("reason must contain 1–2000 bytes"));
    }
    let mut tx = state.pool.begin().await?;
    let mut accounts = vec![actor.discord_id.as_str(), target];
    accounts.sort_unstable();
    accounts.dedup();
    for account in accounts {
        if !lock_account(&mut tx, account).await? {
            return Err(ApiError::not_found("user not found"));
        }
    }
    let now: DateTime<Utc> = sqlx::query_scalar("SELECT clock_timestamp()")
        .fetch_one(&mut *tx)
        .await?;
    let session_valid: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM authentication_sessions
        WHERE id = $1 AND discord_id = $2 AND revoked_at IS NULL AND expires_at > clock_timestamp()
          AND development_role IS NULL)",
    )
    .bind(actor.session_claims.sid)
    .bind(&actor.discord_id)
    .fetch_one(&mut *tx)
    .await?;
    // Session expiry uses the database clock; access credentials use their issuer's API clock.
    if !session_valid
        || actor.session_claims.sub != actor.discord_id
        || actor.session_claims.exp <= Utc::now().timestamp()
    {
        return Err(ApiError::unauthorized("session unavailable"));
    }
    let guild = &state.cfg.discord_guild_id;
    let authority = load_account_authority(&mut tx, &actor.discord_id, guild)
        .await?
        .ok_or_else(|| ApiError::unauthorized("account unavailable"))?;
    if !authority.can_manage_override(now) {
        return Err(ApiError::forbidden("verified administrator required"));
    }
    let target_account = load_account_authority(&mut tx, target, guild)
        .await?
        .ok_or_else(|| ApiError::not_found("user not found"))?;
    if target_account.is_banned
        || target_account.deleted_at.is_some()
        || target_account.membership_status != "member"
        || target_account.verified_at.is_none_or(|t| t > now)
    {
        return Err(ApiError::conflict(
            "override requires a verified, available guild member",
        ));
    }
    let prior_expiry: Option<DateTime<Utc>> = sqlx::query_scalar(
        "SELECT expires_at FROM discord_membership_grace_overrides
         WHERE discord_id = $1 AND guild_id = $2",
    )
    .bind(target)
    .bind(guild)
    .fetch_optional(&mut *tx)
    .await?;
    let expires_at = now + Duration::hours(duration_hours);
    sqlx::query("INSERT INTO discord_membership_grace_overrides
        (discord_id, guild_id, authorized_by, reason, created_at, expires_at) VALUES ($1, $2, $3, $4, $5, $6)
        ON CONFLICT(discord_id, guild_id) DO UPDATE SET authorized_by = EXCLUDED.authorized_by,
            reason = EXCLUDED.reason, created_at = EXCLUDED.created_at, expires_at = EXCLUDED.expires_at")
        .bind(target).bind(guild).bind(&actor.discord_id).bind(reason).bind(now).bind(expires_at)
        .execute(&mut *tx).await?;
    append_required_audit(
        &mut tx,
        &actor.discord_id,
        "membership.grace_extended",
        target,
        &format!(
            "Guild {guild}; previous expiry: {}; resulting expiry: {expires_at}; reason: {reason}",
            prior_expiry.map_or_else(|| "none".to_owned(), |expiry| expiry.to_rfc3339()),
        ),
    )
    .await?;
    tx.commit().await?;
    Ok(expires_at)
}
