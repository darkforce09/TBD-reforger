//! Cluster-coordinated REST reconciliation with durable leases and rate-limit backoff.

use super::discord_membership_cache::{
    MembershipRefreshLease, accept_membership_observation, claim_membership_refresh,
    record_membership_failure,
};
use crate::core::application_state::AppState;
use chrono::Duration;
use sqlx::PgPool;

/// Seed unknown accounts without inferring membership from a preexisting role cache.
pub async fn enroll_accounts(pool: &PgPool, guild_id: &str) -> sqlx::Result<()> {
    if guild_id.trim().is_empty() {
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO discord_membership_snapshots(discord_id, guild_id)
        SELECT discord_id, $1 FROM users WHERE deleted_at IS NULL ON CONFLICT DO NOTHING",
    )
    .bind(guild_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Admit at most 25 requests per second across replicas after obtaining the account lease.
/// Dispatch follows immediately; network timing still depends on fair OS scheduling.
async fn reserve_request_budget(
    pool: &PgPool,
    lease: &MembershipRefreshLease,
) -> sqlx::Result<bool> {
    let mut tx = pool.begin().await?;
    let admitted = sqlx::query("UPDATE discord_rest_schedule SET next_request_at = clock_timestamp() + interval '40 milliseconds'
        WHERE singleton AND next_request_at <= clock_timestamp() AND EXISTS (
            SELECT 1 FROM discord_membership_snapshots WHERE discord_id = $1 AND guild_id = $2
            AND revision = $3 AND lease_token = $4 AND lease_expires_at > clock_timestamp())")
        .bind(&lease.discord_id).bind(&lease.guild_id).bind(lease.revision).bind(lease.lease_token)
        .execute(&mut *tx).await?.rows_affected() == 1;
    if !admitted {
        sqlx::query(
            "UPDATE discord_membership_snapshots SET lease_token = NULL, lease_expires_at = NULL,
            next_refresh_at = GREATEST(next_refresh_at, clock_timestamp(),
                (SELECT next_request_at FROM discord_rest_schedule WHERE singleton))
            WHERE discord_id = $1 AND guild_id = $2 AND revision = $3 AND lease_token = $4",
        )
        .bind(&lease.discord_id)
        .bind(&lease.guild_id)
        .bind(lease.revision)
        .bind(lease.lease_token)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    Ok(admitted)
}

pub async fn reconcile_one(state: &AppState) -> sqlx::Result<bool> {
    if state.cfg.discord_bot_token.is_empty() {
        return Ok(false);
    }
    let due: Option<(String, String)> = sqlx::query_as("SELECT s.discord_id, s.guild_id FROM discord_membership_snapshots s JOIN users u USING(discord_id)
        WHERE u.deleted_at IS NULL AND next_refresh_at <= clock_timestamp() AND (lease_expires_at IS NULL OR lease_expires_at <= clock_timestamp())
        ORDER BY next_refresh_at, s.discord_id, s.guild_id LIMIT 1")
        .fetch_optional(&state.pool).await?;
    let Some((discord_id, guild_id)) = due else {
        return Ok(false);
    };
    let Some(lease) = claim_membership_refresh(&state.pool, &discord_id, &guild_id, false).await?
    else {
        return Ok(false);
    };
    if !reserve_request_budget(&state.pool, &lease).await? {
        return Ok(false);
    }
    match state
        .discord
        .fetch_member_with_bot(&state.cfg.discord_bot_token, &guild_id, &discord_id)
        .await
    {
        Ok(member) => {
            accept_membership_observation(
                &state.pool,
                &lease,
                member.as_ref(),
                &state.cfg.discord_guild_id,
            )
            .await?;
        }
        Err(failure) => {
            if failure.rate_limited {
                sqlx::query("UPDATE discord_rest_schedule SET next_request_at = GREATEST(next_request_at, clock_timestamp() + $1::bigint * interval '1 millisecond') WHERE singleton")
                    .bind(failure.retry_after.num_milliseconds()).execute(&state.pool).await?;
            }
            record_membership_failure(&state.pool, &lease, failure.retry_after, failure.reason)
                .await?;
        }
    }
    Ok(true)
}

/// OAuth refresh and administration hooks request reconciliation without asserting affiliation.
pub async fn request_account_refresh(pool: &PgPool, discord_id: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE discord_membership_snapshots SET next_refresh_at = LEAST(next_refresh_at, now()) WHERE discord_id = $1")
        .bind(discord_id).execute(pool).await?;
    Ok(())
}

pub struct MembershipLookupFailure {
    pub reason: &'static str,
    pub retry_after: Duration,
    pub rate_limited: bool,
}

impl MembershipLookupFailure {
    pub fn unavailable(reason: &'static str) -> Self {
        Self {
            reason,
            retry_after: Duration::seconds(60),
            rate_limited: false,
        }
    }
}
