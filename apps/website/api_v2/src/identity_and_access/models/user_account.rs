//! The identity account root and the rows that hang off it.
//!
//! Field order and JSON keys are the wire contract: snake_case throughout, an absent value
//! expressed as `skip_serializing_if`, and RFC3339Nano timestamps rendered through
//! [`crate::core::wire_format`]. Soft-delete columns are absent from these structs — the
//! filter is enforced in the query layer (`users` is one of the four soft-deletable tables).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::wire_format::{go_time, go_time_opt};

/// Web permission level, synced from Discord roles. Backed by the Postgres ENUM
/// `user_role`. Ordering (low→high): enlisted < leader < mission_maker < admin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    Enlisted,
    Leader,
    MissionMaker,
    Admin,
}

impl UserRole {
    /// The Postgres/JSON wire string (snake_case).
    pub fn as_str(self) -> &'static str {
        match self {
            UserRole::Enlisted => "enlisted",
            UserRole::Leader => "leader",
            UserRole::MissionMaker => "mission_maker",
            UserRole::Admin => "admin",
        }
    }
}

/// Identity root, keyed by the Discord snowflake (no local passwords).
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub discord_id: String,
    pub username: String,
    pub discord_handle: String,
    pub avatar_url: String,
    /// Enfusion/Steam ID, `null` until linked — serialized even when absent.
    pub arma_id: Option<String>,
    pub arma_character: String,
    pub role: UserRole,
    pub is_banned: bool,
    #[serde(skip_serializing_if = "String::is_empty", default)]
    pub ban_reason: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub banned_by: Option<String>,
    #[serde(with = "go_time_opt", skip_serializing_if = "Option::is_none", default)]
    pub banned_at: Option<DateTime<Utc>>,
    pub total_deployments: i64,
    /// `numeric(5,2)` — queries must `CAST(attendance_rate AS double precision)`.
    pub attendance_rate: f64,
    #[serde(with = "go_time_opt", skip_serializing_if = "Option::is_none", default)]
    pub last_login_at: Option<DateTime<Utc>>,
    #[serde(with = "go_time")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "go_time")]
    pub updated_at: DateTime<Utc>,
}

/// Maps a Discord guild role to a web permission. Highest `priority` wins.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DiscordRole {
    pub discord_role_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub mapped_role: Option<UserRole>,
    pub priority: i64,
}

/// Join between a user and their synced Discord roles.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserDiscordRole {
    pub discord_id: String,
    pub discord_role_id: String,
    #[serde(with = "go_time")]
    pub synced_at: DateTime<Utc>,
}

/// Backs the 6-digit Arma linking flow.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct IdentityLinkCode {
    pub code: String,
    pub discord_id: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub arma_id: Option<String>,
    #[serde(with = "go_time_opt", skip_serializing_if = "Option::is_none", default)]
    pub consumed_at: Option<DateTime<Utc>>,
    #[serde(with = "go_time")]
    pub expires_at: DateTime<Utc>,
    #[serde(with = "go_time")]
    pub created_at: DateTime<Utc>,
}

/// Opaque, rotating refresh credential stored hashed. `token_hash` never reaches the wire.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct RefreshToken {
    pub id: Uuid,
    pub discord_id: String,
    #[serde(skip)]
    pub token_hash: String,
    #[serde(with = "go_time")]
    pub expires_at: DateTime<Utc>,
    #[serde(with = "go_time_opt", skip_serializing_if = "Option::is_none", default)]
    pub revoked_at: Option<DateTime<Utc>>,
    #[serde(with = "go_time")]
    pub created_at: DateTime<Utc>,
}
