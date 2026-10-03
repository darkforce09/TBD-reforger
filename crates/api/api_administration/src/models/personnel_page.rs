//! One page of the personnel roster, as `GET /api/v1/admin/users` answers it.
//!
//! **Role:** the wire shape of the administrator's member roster: the page envelope and the row
//! each member is listed as.
//! **Position:** filled by [`crate::handlers::personnel_roster::list_users`] from
//! `users` and `warnings`; read by the personnel page of the single-page app.
//! **Signals & state:** none; plain data.
//! **Invariants:** `page` is at least 1 and `per_page` is 1 to 100; `total` counts every matching
//! member, so a page past the end carries no items and the real total; `username`,
//! `discord_handle` and `arma_character` are empty strings when unset, and `arma_id` is `null`
//! until the member links an Arma identity.
//! @contract personnel-roster.schema.json#/definitions/PersonnelPage

use api_identifiers::{ArmaPlayerId, DiscordUserId};
use serde::{Deserialize, Serialize};

use api_caller_identity::UserRole;

/// One member as the roster lists them.
/// @contract personnel-roster.schema.json#/definitions/PersonnelRow
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, sqlx::FromRow)]
pub struct PersonnelRow {
    /// The member's Discord account id.
    pub discord_id: DiscordUserId,
    /// The member's display name; empty when unset.
    pub username: String,
    /// The member's Discord handle; empty when unset.
    pub discord_handle: String,
    /// `null` until the member links an Arma identity; always present on the wire.
    pub arma_id: Option<ArmaPlayerId>,
    /// The member's in-game character name; empty when unset.
    pub arma_character: String,
    /// The member's role, derived from their Discord roles.
    pub role: UserRole,
    /// Whether the member is banned.
    pub is_banned: bool,
    /// The member's disciplinary warnings, counted from `warnings`.
    pub warnings: i64,
    /// The denormalized `users.total_deployments` counter the telemetry pipeline maintains.
    pub total_deployments: i64,
}

/// One page of members in `lower(username)`, `discord_id` order.
/// @contract personnel-roster.schema.json#/definitions/PersonnelPage
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonnelPage {
    /// The members on this page.
    pub items: Vec<PersonnelRow>,
    /// The 1-based page served.
    pub page: i64,
    /// The page size served, after clamping to 100.
    pub per_page: i64,
    /// Every member the search matches, independent of the page.
    pub total: i64,
}
