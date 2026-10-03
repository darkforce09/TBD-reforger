//! The identity account root and the rows that hang off it.
//!
//! Field order and JSON keys are the wire contract: snake_case throughout, an absent value
//! expressed as `skip_serializing_if`, and RFC3339Nano timestamps rendered through
//! [`fleet_wire_contract::rfc3339_timestamps`]. Soft-delete columns are absent from these structs — the
//! filter is enforced in the query layer (`users` is one of the four soft-deletable tables).
//!
//! @contract current-profile.schema.json#/definitions/UserAccount
//! @contract profile-update.schema.json#/definitions/UserAccount

use api_identifiers::{ArmaPlayerId, DiscordRoleId, DiscordUserId, RefreshTokenId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use api_caller_identity::UserRole;
use fleet_wire_contract::rfc3339_timestamps::{rfc3339_utc, rfc3339_utc_opt};

/// Identity root, keyed by the Discord snowflake (no local passwords).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    /// The Discord user id, the account's key.
    pub discord_id: DiscordUserId,
    /// The Discord username.
    pub username: String,
    /// The Discord handle.
    pub discord_handle: String,
    /// The Discord avatar URL; empty when the account has none.
    pub avatar_url: String,
    /// Enfusion/Steam ID, `null` until linked — serialized even when absent.
    pub arma_id: Option<ArmaPlayerId>,
    /// The linked Arma character name; empty until linked.
    pub arma_character: String,
    /// The account's role on the wire.
    pub role: UserRole,
    /// Whether the account is banned.
    pub is_banned: bool,
    /// Why the account is banned; omitted when empty.
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub ban_reason: String,
    /// The Discord id of the administrator who banned the account.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub banned_by: Option<String>,
    /// When the account was banned.
    #[serde(
        with = "rfc3339_utc_opt",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub banned_at: Option<DateTime<Utc>>,
    /// The number of finished deployments attributed to the account.
    pub total_deployments: i64,
    /// `numeric(5,2)` — queries must `CAST(attendance_rate AS double precision)`.
    pub attendance_rate: f64,
    /// When the account last signed in.
    #[serde(
        with = "rfc3339_utc_opt",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub last_login_at: Option<DateTime<Utc>>,
    /// When the account was created.
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
    /// When the account row last changed.
    #[serde(with = "rfc3339_utc")]
    pub updated_at: DateTime<Utc>,
}

/// Maps a Discord guild role to a web permission. Highest `priority` wins.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DiscordRole {
    /// The Discord guild role id.
    pub discord_role_id: DiscordRoleId,
    /// The Discord role name.
    pub name: String,
    /// The web role the Discord role grants, when it maps to one.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mapped_role: Option<UserRole>,
    /// The precedence among mapped roles; the highest wins.
    pub priority: i64,
}

/// Join between a user and their synced Discord roles.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserDiscordRole {
    /// The Discord user id of the member.
    pub discord_id: DiscordUserId,
    /// The Discord guild role id the member holds.
    pub discord_role_id: DiscordRoleId,
    /// When the role was last synced from Discord.
    #[serde(with = "rfc3339_utc")]
    pub synced_at: DateTime<Utc>,
}

/// Backs the 6-digit Arma linking flow.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct IdentityLinkCode {
    /// The six-digit code the player types in the game.
    pub code: String,
    /// The Discord user id of the account that asked for the code.
    pub discord_id: DiscordUserId,
    /// The Arma identity that spent the code, once spent.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub arma_id: Option<ArmaPlayerId>,
    /// When the code was spent.
    #[serde(
        with = "rfc3339_utc_opt",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub consumed_at: Option<DateTime<Utc>>,
    /// When the code stops being accepted.
    #[serde(with = "rfc3339_utc")]
    pub expires_at: DateTime<Utc>,
    /// When the code was issued.
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
}

/// Opaque, rotating refresh credential stored hashed. `token_hash` never reaches the wire.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RefreshToken {
    /// The refresh token's row id.
    pub id: RefreshTokenId,
    /// The Discord user id of the account the token belongs to.
    pub discord_id: DiscordUserId,
    /// The SHA-256 hash of the opaque token; the raw token is never stored.
    #[serde(skip)]
    pub token_hash: String,
    /// When the token expires.
    #[serde(with = "rfc3339_utc")]
    pub expires_at: DateTime<Utc>,
    /// When the token was revoked, if it was.
    #[serde(
        with = "rfc3339_utc_opt",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub revoked_at: Option<DateTime<Utc>>,
    /// When the token was issued.
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
}
