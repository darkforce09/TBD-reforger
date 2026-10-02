//! Fenced REST observations: a superseded request cannot restore older membership grants.

use super::account_authority::{load_account_authority, lock_account};
use super::discord_client::GuildMember;
use crate::operations::services::event_reservations::reevaluation_queue::request_reevaluation_for_account;
use chrono::Duration;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
pub struct MembershipRefreshLease {
    pub discord_id: String,
    pub guild_id: String,
    pub revision: i64,
    pub lease_token: Uuid,
}

/// OAuth hooks supersede older requests. Periodic workers claim only unleased due rows.
pub async fn claim_membership_refresh(
    pool: &PgPool,
    discord_id: &str,
    guild_id: &str,
    force: bool,
) -> sqlx::Result<Option<MembershipRefreshLease>> {
    if guild_id.trim().is_empty() {
        return Ok(None);
    }
    let mut tx = pool.begin().await?;
    if !lock_account(&mut tx, discord_id).await? {
        return Ok(None);
    }
    let active: bool =
        sqlx::query_scalar("SELECT deleted_at IS NULL FROM users WHERE discord_id = $1")
            .bind(discord_id)
            .fetch_one(&mut *tx)
            .await?;
    if !active {
        return Ok(None);
    }
    sqlx::query(
        "INSERT INTO discord_membership_snapshots(discord_id, guild_id) VALUES ($1, $2)
        ON CONFLICT DO NOTHING",
    )
    .bind(discord_id)
    .bind(guild_id)
    .execute(&mut *tx)
    .await?;
    let lease = sqlx::query_as("UPDATE discord_membership_snapshots SET revision = revision + 1,
        lease_token = $3, lease_expires_at = clock_timestamp() + interval '45 seconds'
        WHERE discord_id = $1 AND guild_id = $2 AND ($4 OR
          (next_refresh_at <= clock_timestamp() AND (lease_expires_at IS NULL OR lease_expires_at <= clock_timestamp())))
        RETURNING discord_id, guild_id, revision, lease_token")
        .bind(discord_id).bind(guild_id).bind(Uuid::new_v4()).bind(force).fetch_optional(&mut *tx).await?;
    tx.commit().await?;
    Ok(lease)
}

/// Commit membership and roles together; None is a confirmed nonmember, never a transport failure.
/// A changed status or role set enqueues re-evaluation of the account's event reservations in
/// the same transaction; the re-evaluation itself runs later in an event-first transaction.
pub async fn accept_membership_observation(
    pool: &PgPool,
    lease: &MembershipRefreshLease,
    member: Option<&GuildMember>,
    main_guild: &str,
) -> sqlx::Result<bool> {
    let mut tx = pool.begin().await?;
    lock_account(&mut tx, &lease.discord_id).await?;
    let previous: Option<(String, Vec<String>)> = sqlx::query_as(
        "SELECT s.membership_status, ARRAY(SELECT r.discord_role_id FROM user_discord_roles r
             WHERE r.discord_id = s.discord_id AND r.guild_id = s.guild_id
             ORDER BY r.discord_role_id COLLATE \"C\")
         FROM discord_membership_snapshots s WHERE s.discord_id = $1 AND s.guild_id = $2",
    )
    .bind(&lease.discord_id)
    .bind(&lease.guild_id)
    .fetch_optional(&mut *tx)
    .await?;
    let changed = sqlx::query("UPDATE discord_membership_snapshots SET membership_status = $5,
        verified_at = clock_timestamp(), last_error = NULL, lease_token = NULL, lease_expires_at = NULL,
        next_refresh_at = clock_timestamp() + interval '40 seconds'
        WHERE discord_id = $1 AND guild_id = $2 AND revision = $3 AND lease_token = $4
          AND lease_expires_at > clock_timestamp()")
        .bind(&lease.discord_id).bind(&lease.guild_id).bind(lease.revision).bind(lease.lease_token)
        .bind(if member.is_some() { "member" } else { "nonmember" }).execute(&mut *tx).await?;
    if changed.rows_affected() == 0 {
        return Ok(false);
    }
    sqlx::query("DELETE FROM user_discord_roles WHERE discord_id = $1 AND guild_id = $2")
        .bind(&lease.discord_id)
        .bind(&lease.guild_id)
        .execute(&mut *tx)
        .await?;
    if let Some(member) = member {
        sqlx::query("INSERT INTO user_discord_roles(discord_id, guild_id, discord_role_id, synced_at)
            SELECT $1, $2, role_id, now() FROM unnest($3::text[]) AS role_id ON CONFLICT DO NOTHING")
            .bind(&lease.discord_id).bind(&lease.guild_id).bind(&member.roles).execute(&mut *tx).await?;
    } else {
        sqlx::query("DELETE FROM discord_membership_grace_overrides WHERE discord_id = $1 AND guild_id = $2")
            .bind(&lease.discord_id).bind(&lease.guild_id).execute(&mut *tx).await?;
    }
    let mut observed_roles: Vec<String> = member.map(|m| m.roles.clone()).unwrap_or_default();
    observed_roles.sort_unstable();
    observed_roles.dedup();
    let observed_status = if member.is_some() {
        "member"
    } else {
        "nonmember"
    };
    if previous
        .as_ref()
        .is_none_or(|(status, roles)| status != observed_status || *roles != observed_roles)
    {
        request_reevaluation_for_account(&mut tx, &lease.discord_id)
            .await
            .map_err(|error| sqlx::Error::Protocol(error.message))?;
    }
    if lease.guild_id == main_guild
        && let Some(account) =
            load_account_authority(&mut tx, &lease.discord_id, main_guild).await?
    {
        let role = account
            .permissions(account.observed_at)
            .effective_role
            .unwrap_or(crate::identity_and_access::models::user_account::UserRole::Guest);
        sqlx::query("UPDATE users SET role = $2, updated_at = now() WHERE discord_id = $1")
            .bind(&lease.discord_id)
            .bind(role)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(true)
}

/// Failure records never change verified membership, roles, or reservation ownership.
pub async fn record_membership_failure(
    pool: &PgPool,
    lease: &MembershipRefreshLease,
    retry_delay: Duration,
    reason: &str,
) -> sqlx::Result<()> {
    sqlx::query(
        "UPDATE discord_membership_snapshots SET last_error = $5, next_refresh_at = clock_timestamp() + $6::bigint * interval '1 millisecond',
        lease_token = NULL, lease_expires_at = NULL
        WHERE discord_id = $1 AND guild_id = $2 AND revision = $3 AND lease_token = $4",
    )
    .bind(&lease.discord_id)
    .bind(&lease.guild_id)
    .bind(lease.revision)
    .bind(lease.lease_token)
    .bind(reason)
    .bind(retry_delay.num_milliseconds().clamp(1000, 7 * 24 * 60 * 60 * 1000))
    .execute(pool)
    .await?;
    Ok(())
}
