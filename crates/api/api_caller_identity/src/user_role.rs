//! The web permission level a caller holds.
//!
//! **Role:** the role ladder every authorization decision compares against.
//! **Position:** read from `users.role`, `discord_roles.mapped_role` and a development session's
//! role by the account authority checks; named by the identity, administration and operations
//! domains.
//! **Signals & state:** none; plain data.
//! **Invariants:** [`UserRole`] and the Postgres enum `user_role` hold the same five values,
//! spelled snake_case on the wire and in SQL; the rank order is guest < enlisted < leader <
//! mission_maker < admin.

use serde::{Deserialize, Serialize};

/// Web permission level, synced from Discord roles. Backed by the Postgres ENUM
/// `user_role`. Ordering (low→high): guest < enlisted < leader < mission_maker < admin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "user_role", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    /// A signed-in visitor with no member rank.
    Guest,
    /// A community member.
    Enlisted,
    /// A member who leads squads and slots events.
    Leader,
    /// A member who authors and submits missions.
    MissionMaker,
    /// An administrator of the platform.
    Admin,
}

impl UserRole {
    /// The Postgres/JSON wire string (snake_case).
    pub fn as_str(self) -> &'static str {
        match self {
            UserRole::Guest => "guest",
            UserRole::Enlisted => "enlisted",
            UserRole::Leader => "leader",
            UserRole::MissionMaker => "mission_maker",
            UserRole::Admin => "admin",
        }
    }
}
