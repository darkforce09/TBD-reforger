//! Current account permissions from a verified, guild-scoped Discord snapshot.
//!
//! **Role:** reads an account's current authority (role, ban, membership snapshot, override)
//! in one statement, and takes the account lock every session writer takes first.
//! **Position:** read by session issuance, rotation and authorization, the Discord membership
//! writers, and the administrator checks of missions and server infrastructure.
//! **Signals & state:** none; each read runs on its caller's connection.
//! **Invariants:** only the configured guild grants a site role, never `users.role`; equal
//! mapping priorities resolve to the smallest Discord role id in C collation.

use crate::UserRole;
use crate::cached_membership_permissions::{
    CachedMembershipPermissionDecision, can_manage_sync_override,
    evaluate_cached_membership_permissions,
};
use api_identifiers::{ArmaPlayerId, DiscordGuildId, DiscordUserId};
use chrono::{DateTime, Utc};
use sqlx::PgConnection;

/// One account's authority facts, observed at `observed_at`.
#[derive(Debug, sqlx::FromRow)]
pub struct AccountAuthority {
    /// The account.
    pub discord_id: DiscordUserId,
    /// The database clock when the facts were read; every decision is made at this instant.
    pub observed_at: DateTime<Utc>,
    /// The linked Arma identity, if any (see [`crate::arma_identity_link::arma_id_is_linked`]).
    pub arma_id: Option<ArmaPlayerId>,
    /// Whether the account is banned; a ban removes every role.
    pub is_banned: bool,
    /// When the account was deleted, if it was.
    pub deleted_at: Option<DateTime<Utc>>,
    /// The guild membership status of the snapshot (`member`, or `unknown` with no snapshot).
    pub membership_status: String,
    /// When the guild snapshot was last verified, if ever.
    pub verified_at: Option<DateTime<Utc>>,
    /// The last synchronization failure of the snapshot, which marks it stale.
    pub last_error: Option<String>,
    /// The highest-priority mapped role of the guild snapshot (`enlisted` with none).
    pub cached_role: UserRole,
    /// When the audited grace override ends, if one exists.
    pub override_until: Option<DateTime<Utc>>,
}

impl AccountAuthority {
    /// The permission decision these facts give at `now`.
    pub fn permissions(&self, now: DateTime<Utc>) -> CachedMembershipPermissionDecision {
        let mut decision = evaluate_cached_membership_permissions(
            now,
            self.verified_at,
            self.cached_role,
            self.membership_status != "member",
            self.override_until,
            self.is_banned,
        );
        decision.stale |= self.last_error.is_some();
        decision
    }

    /// Whether this account may manage membership synchronization overrides at `now`.
    pub fn can_manage_override(&self, now: DateTime<Utc>) -> bool {
        can_manage_sync_override(
            now,
            self.verified_at,
            self.cached_role,
            self.membership_status != "member",
            self.is_banned || self.deleted_at.is_some(),
        )
    }
}

/// The permission query never reads users.role: only the configured TBD guild grants site roles.
/// Equal mapping priorities choose the lexicographically smallest Discord role ID (C collation).
pub async fn load_account_authority(
    connection: &mut PgConnection,
    discord_id: &DiscordUserId,
    guild_id: &DiscordGuildId,
) -> sqlx::Result<Option<AccountAuthority>> {
    sqlx::query_as(ACCOUNT_AUTHORITY_QUERY)
        .bind(discord_id)
        .bind(guild_id)
        .fetch_optional(connection)
        .await
}

/// Whether the account holds administrator authority now: it exists, is not deleted, and its
/// current membership permissions resolve to the administrator role.
pub async fn holds_administrator_authority(
    connection: &mut PgConnection,
    discord_id: &DiscordUserId,
    guild_id: &DiscordGuildId,
) -> sqlx::Result<bool> {
    let Some(account) = load_account_authority(connection, discord_id, guild_id).await? else {
        return Ok(false);
    };
    Ok(account.deleted_at.is_none()
        && account.permissions(account.observed_at).effective_role == Some(UserRole::Admin))
}

/// All session writers lock the account before any session or refresh-token row.
pub async fn lock_account(
    connection: &mut PgConnection,
    discord_id: &DiscordUserId,
) -> sqlx::Result<bool> {
    Ok(sqlx::query_scalar::<_, String>(
        "SELECT discord_id FROM users WHERE discord_id = $1 FOR UPDATE",
    )
    .bind(discord_id)
    .fetch_optional(connection)
    .await?
    .is_some())
}

/// The account authority statement: `$1` the account, `$2` the configured guild.
pub const ACCOUNT_AUTHORITY_QUERY: &str =
    "SELECT clock_timestamp() AS observed_at, u.discord_id, u.arma_id, u.is_banned, u.deleted_at,
         COALESCE(s.membership_status, 'unknown') AS membership_status, s.verified_at, s.last_error,
         COALESCE((SELECT d.mapped_role FROM user_discord_roles r
           JOIN discord_roles d USING (discord_role_id)
           WHERE r.discord_id = u.discord_id AND r.guild_id = $2 AND d.mapped_role IS NOT NULL
           ORDER BY d.priority DESC, d.discord_role_id COLLATE \"C\" ASC LIMIT 1),
           'enlisted'::user_role) AS cached_role, o.expires_at AS override_until
         FROM users u LEFT JOIN discord_membership_snapshots s
           ON s.discord_id = u.discord_id AND s.guild_id = $2
         LEFT JOIN discord_membership_grace_overrides o
           ON o.discord_id = s.discord_id AND o.guild_id = s.guild_id
         WHERE u.discord_id = $1";
