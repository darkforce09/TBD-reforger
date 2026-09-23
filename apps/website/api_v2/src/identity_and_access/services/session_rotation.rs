//! Atomic refresh rotation and session logout under a common account lock.

use super::account_authority::{load_account_authority, lock_account};
use super::session_issuance::arma_id_is_linked;
use super::session_storage::{
    RefreshRow, SESSION_LIFETIME, SessionRow, insert_refresh, revoke_account_sessions,
    revoke_session,
};
use crate::core::application_state::AppState;
use crate::core::authentication_primitives::hash_token;
use crate::core::error_handling::api_error::ApiError;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// The unlocked lookup discovers only the lock key; mutable state is reread after locking.
async fn token_owner(state: &AppState, hash: &str) -> Result<Option<String>, ApiError> {
    Ok(
        sqlx::query_scalar("SELECT discord_id FROM refresh_tokens WHERE token_hash = $1")
            .bind(hash)
            .fetch_optional(&state.pool)
            .await?,
    )
}

pub async fn rotate_session(
    state: &AppState,
    raw: &str,
) -> Result<(String, DateTime<Utc>, String), ApiError> {
    let hash = hash_token(raw);
    let owner = token_owner(state, &hash)
        .await?
        .ok_or_else(|| ApiError::unauthorized("invalid refresh token"))?;
    let mut tx = state.pool.begin().await?;
    if !lock_account(&mut tx, &owner).await? {
        return Err(ApiError::unauthorized("user not found"));
    }
    let token: RefreshRow = sqlx::query_as(
        "SELECT id, session_id, expires_at, revoked_at FROM refresh_tokens
        WHERE token_hash = $1 AND discord_id = $2",
    )
    .bind(&hash)
    .bind(&owner)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| ApiError::unauthorized("invalid refresh token"))?;
    let session: SessionRow = sqlx::query_as(
        "SELECT id, discord_id, expires_at, revoked_at, development_role
        FROM authentication_sessions WHERE id = $1 AND discord_id = $2 FOR UPDATE",
    )
    .bind(token.session_id)
    .bind(&owner)
    .fetch_one(&mut *tx)
    .await?;
    // Credentials from a development session cannot authorize production-side revocation.
    if session.development_role.is_some() && !state.cfg.is_development() {
        return Err(ApiError::unauthorized("invalid session provenance"));
    }
    // A retired family has no live successor. Its old credentials cannot revoke a later login.
    if session.revoked_at.is_some() {
        return Err(ApiError::unauthorized("expired or revoked session"));
    }
    if token.revoked_at.is_some() {
        revoke_account_sessions(&mut tx, &owner).await?;
        crate::administration::services::required_audit::append_required_audit(
            &mut tx,
            &owner,
            "auth.refresh_replay",
            &owner,
            "Refresh replay revoked account sessions",
        )
        .await?;
        tx.commit().await?;
        return Err(ApiError::unauthorized("refresh token reuse detected"));
    }
    let account = load_account_authority(&mut tx, &owner, &state.cfg.discord_guild_id)
        .await?
        .ok_or_else(|| ApiError::unauthorized("user not found"))?;
    let now = account.observed_at;
    if token.expires_at <= now || session.expires_at <= now {
        return Err(ApiError::unauthorized("expired or revoked session"));
    }
    if account.is_banned || account.deleted_at.is_some() {
        revoke_account_sessions(&mut tx, &owner).await?;
        tx.commit().await?;
        return Err(ApiError::forbidden("account is unavailable"));
    }
    let role = session
        .development_role
        .or(account.permissions(now).effective_role)
        .ok_or_else(|| ApiError::forbidden("account is banned"))?;
    sqlx::query("UPDATE refresh_tokens SET revoked_at = clock_timestamp() WHERE id = $1")
        .bind(token.id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE authentication_sessions SET expires_at = $2 WHERE id = $1")
        .bind(session.id)
        .bind(now + SESSION_LIFETIME)
        .execute(&mut *tx)
        .await?;
    let refresh = insert_refresh(&mut tx, &owner, session.id).await?;
    sqlx::query("UPDATE discord_membership_snapshots SET next_refresh_at = LEAST(next_refresh_at, now()) WHERE discord_id = $1")
        .bind(&owner).execute(&mut *tx).await?;
    let (access, expires_at) = state
        .jwt
        .issue_access(
            &owner,
            session.id,
            role.as_str(),
            arma_id_is_linked(&account.arma_id),
        )
        .map_err(|_| ApiError::internal("could not issue token"))?;
    tx.commit().await?;
    Ok((access, expires_at, refresh))
}

/// Any retained token in the session, including an already rotated token, can log that session out.
pub async fn logout_session(state: &AppState, raw: &str) -> Result<(), ApiError> {
    let hash = hash_token(raw);
    let Some(owner) = token_owner(state, &hash).await? else {
        return Ok(());
    };
    let mut tx = state.pool.begin().await?;
    lock_account(&mut tx, &owner).await?;
    let session: Option<Uuid> = sqlx::query_scalar(
        "SELECT session_id FROM refresh_tokens WHERE token_hash = $1 AND discord_id = $2",
    )
    .bind(&hash)
    .bind(&owner)
    .fetch_optional(&mut *tx)
    .await?;
    if let Some(session) = session {
        revoke_session(&mut tx, session).await?;
        crate::administration::services::required_audit::append_required_audit(
            &mut tx,
            &owner,
            "auth.logout",
            &owner,
            "Authentication session revoked",
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}
