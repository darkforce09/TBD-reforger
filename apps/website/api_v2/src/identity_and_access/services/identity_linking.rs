//! Single-use confirmation and unlink commit ownership, derived statistics, and required audit together.
use super::{
    identity_ownership::{lock_accounts, lock_identities},
    session_authorization::authorize_on_connection,
};
use crate::administration::services::required_audit::append_required_audit;
use crate::command_center::services::{
    leaderboard_view::refresh_leaderboard_on_connection,
    user_stats::recompute_user_stats_on_connection,
};
use crate::core::{
    application_state::AppState, error_handling::api_error::ApiError, middleware::AuthUser,
};

/// Attendance keeps its factual account and mission context when gameplay attribution changes.
pub const BACKFILL_ATTENDANCE: &str = "INSERT INTO event_registration_participation (registration_id, match_id, arma_id)
    SELECT DISTINCT registration.id, m.id, s.arma_id
    FROM event_missions em INNER JOIN matches m
    ON m.event_id = em.event_id AND m.mission_id = em.mission_id
    INNER JOIN match_player_stats s ON s.match_id = m.id
    INNER JOIN event_registrations registration ON registration.event_mission_id = em.id AND registration.discord_id = $1
    WHERE s.arma_id = $2 AND m.event_id IS NOT NULL AND m.mission_id IS NOT NULL AND m.finalized_at IS NOT NULL
    ON CONFLICT (registration_id, match_id, arma_id) DO NOTHING";

#[derive(Debug, PartialEq, Eq)]
pub struct ConfirmedIdentity {
    pub discord_id: String,
    pub arma_id: String,
    pub arma_character: String,
}

pub async fn confirm_identity(
    state: &AppState,
    code: &str,
    arma_id: &str,
    character: &str,
) -> Result<ConfirmedIdentity, ApiError> {
    let arma_id = arma_id.trim();
    if code.len() != 6
        || !code.bytes().all(|c| c.is_ascii_digit())
        || arma_id.is_empty()
        || arma_id.len() > 128
        || character.len() > 256
    {
        return Err(ApiError::bad_request(
            "a six-digit code, Arma identity (at most 128 bytes), and character (at most 256 bytes) are required",
        ));
    }
    // This read locates the account only; the locked row below decides whether the code can be spent.
    let discord_id: String =
        sqlx::query_scalar("SELECT discord_id FROM identity_link_codes WHERE code = $1")
            .bind(code)
            .fetch_optional(&state.pool)
            .await?
            .ok_or_else(|| ApiError::not_found("invalid or expired code"))?;
    let mut tx = state.pool.begin().await?;
    lock_identities(&mut tx, &[arma_id]).await?;
    let mut affected: Vec<String> = sqlx::query_scalar(
        "SELECT discord_id FROM match_player_stats WHERE arma_id = $1 AND discord_id IS NOT NULL
        UNION SELECT discord_id FROM users WHERE arma_id = $1",
    )
    .bind(arma_id)
    .fetch_all(&mut *tx)
    .await?;
    affected.push(discord_id.clone());
    lock_accounts(&mut tx, &affected).await?;
    let account: Option<(Option<String>, bool, bool, String)> = sqlx::query_as(
        "SELECT arma_id, is_banned, deleted_at IS NOT NULL, COALESCE(arma_character, '') AS arma_character FROM users WHERE discord_id = $1",
    )
    .bind(&discord_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((current, banned, deleted, current_character)) = account else {
        return Err(ApiError::not_found("account not found"));
    };
    if banned || deleted {
        return Err(ApiError::forbidden("account is unavailable"));
    }
    let row: (bool, bool, bool, Option<String>) = sqlx::query_as("SELECT consumed_at IS NOT NULL, cancelled_at IS NOT NULL,
        expires_at > clock_timestamp(), arma_id FROM identity_link_codes WHERE code = $1 AND discord_id = $2 FOR UPDATE")
        .bind(code).bind(&discord_id).fetch_one(&mut *tx).await?;
    if row.0 {
        if row.3.as_deref() == Some(arma_id) && current.as_deref() == Some(arma_id) {
            return Ok(ConfirmedIdentity {
                discord_id,
                arma_id: arma_id.to_owned(),
                arma_character: current_character,
            });
        }
        return Err(ApiError::conflict("code has already been consumed"));
    }
    if row.1 || !row.2 {
        return Err(ApiError::not_found("invalid or expired code"));
    }
    if current
        .as_deref()
        .is_some_and(|id| !id.trim().is_empty() && id != arma_id)
    {
        return Err(ApiError::conflict(
            "unlink the current Arma identity before linking another",
        ));
    }
    release_deleted_owner(&mut tx, arma_id, &discord_id).await?;
    let spent = sqlx::query("UPDATE identity_link_codes SET consumed_at = clock_timestamp(), arma_id = $1
        WHERE code = $2 AND consumed_at IS NULL AND cancelled_at IS NULL AND expires_at > clock_timestamp()")
        .bind(arma_id).bind(code).execute(&mut *tx).await?.rows_affected();
    if spent != 1 {
        return Err(ApiError::not_found("invalid or expired code"));
    }
    sqlx::query("UPDATE users SET arma_id = $1, arma_character = $2, updated_at = now() WHERE discord_id = $3")
        .bind(arma_id).bind(character).bind(&discord_id).execute(&mut *tx).await?;
    let claimed = sqlx::query("UPDATE match_player_stats SET discord_id = $1 WHERE arma_id = $2 AND discord_id IS DISTINCT FROM $1")
        .bind(&discord_id).bind(arma_id).execute(&mut *tx).await?.rows_affected();
    let attended = sqlx::query(BACKFILL_ATTENDANCE)
        .bind(&discord_id)
        .bind(arma_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    crate::operations::services::participation_attribution::refresh_attendance(&mut tx, &affected)
        .await?;
    affected.sort_unstable();
    affected.dedup();
    for account in &affected {
        recompute_user_stats_on_connection(&mut tx, account).await?;
    }
    append_required_audit(&mut tx, &discord_id, "identity.link", &discord_id,
        &format!("Verified Arma identity; attributed {claimed} historical match rows and {attended} attendance records")).await?;
    refresh_leaderboard_on_connection(&mut tx).await?;
    tx.commit().await?;
    Ok(ConfirmedIdentity {
        discord_id,
        arma_id: arma_id.to_owned(),
        arma_character: character.to_owned(),
    })
}

/// The caller holds the identity and every current/derived owner account lock, and validates
/// the claimant's pending code before releasing an unavailable account's current ownership.
async fn release_deleted_owner(
    connection: &mut sqlx::PgConnection,
    arma_id: &str,
    claimant: &str,
) -> Result<(), ApiError> {
    let owner: Option<(String, bool)> = sqlx::query_as(
        "SELECT discord_id, deleted_at IS NOT NULL FROM users WHERE arma_id = $1 AND discord_id <> $2",
    ).bind(arma_id).bind(claimant).fetch_optional(&mut *connection).await?;
    let Some((owner, deleted)) = owner else {
        return Ok(());
    };
    if !deleted {
        return Err(ApiError::conflict(
            "arma id already linked to another account",
        ));
    }
    super::session_storage::revoke_account_sessions(connection, &owner).await?;
    sqlx::query("UPDATE identity_link_codes SET cancelled_at = clock_timestamp(), cancellation_reason = 'deleted_owner_reclaimed'
        WHERE discord_id = $1 AND consumed_at IS NULL AND cancelled_at IS NULL")
        .bind(&owner).execute(&mut *connection).await?;
    sqlx::query("UPDATE users SET arma_id = NULL, arma_character = '', updated_at = clock_timestamp() WHERE discord_id = $1")
        .bind(&owner).execute(&mut *connection).await?;
    append_required_audit(connection, claimant, "identity.deleted_owner_released", &owner,
        "Released a deleted account's current Arma ownership during verified reassignment; factual account authorship remains unchanged").await?;
    Ok(())
}

pub async fn unlink_identity(state: &AppState, user: &AuthUser) -> Result<(), ApiError> {
    for _ in 0..3 {
        let observed: Option<String> =
            sqlx::query_scalar("SELECT arma_id FROM users WHERE discord_id = $1")
                .bind(&user.discord_id)
                .fetch_optional(&state.pool)
                .await?
                .flatten();
        let mut tx = state.pool.begin().await?;
        if let Some(id) = observed.as_deref().filter(|id| !id.trim().is_empty()) {
            lock_identities(&mut tx, &[id]).await?;
        }
        lock_accounts(&mut tx, std::slice::from_ref(&user.discord_id)).await?;
        authorize_on_connection(&mut tx, &state.cfg, &user.session_claims).await?;
        let current: Option<String> =
            sqlx::query_scalar("SELECT arma_id FROM users WHERE discord_id = $1")
                .bind(&user.discord_id)
                .fetch_one(&mut *tx)
                .await?;
        if current != observed {
            tx.rollback().await?;
            continue;
        }
        sqlx::query("UPDATE identity_link_codes SET cancelled_at = clock_timestamp(), cancellation_reason = 'unlinked'
            WHERE discord_id = $1 AND consumed_at IS NULL AND cancelled_at IS NULL")
            .bind(&user.discord_id).execute(&mut *tx).await?;
        let released = sqlx::query("UPDATE match_player_stats SET discord_id = NULL WHERE arma_id = $1 AND discord_id = $2")
            .bind(&current).bind(&user.discord_id).execute(&mut *tx).await?.rows_affected();
        sqlx::query("UPDATE users SET arma_id = NULL, arma_character = '', updated_at = now() WHERE discord_id = $1")
            .bind(&user.discord_id).execute(&mut *tx).await?;
        recompute_user_stats_on_connection(&mut tx, &user.discord_id).await?;
        append_required_audit(&mut tx, &user.discord_id, "identity.unlink", &user.discord_id,
            &format!("Unlinked Arma identity; released {released} historical match rows; signup and attendance facts remain attributed")).await?;
        refresh_leaderboard_on_connection(&mut tx).await?;
        tx.commit().await?;
        return Ok(());
    }
    Err(ApiError::conflict(
        "identity changed concurrently; retry unlinking",
    ))
}
