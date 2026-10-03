//! Faction library model — operator-authored reusable factions consumed by the
//! Mission Creator palette (side → faction → roles/vehicles).
//!
//! @contract faction-library.schema.json#/
//!
//! @contract arsenal-envelopes.schema.json#/definitions/UserFaction

use api_identifiers::{DiscordUserId, UserFactionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use api_foundation::wire_format::RawJson;
use fleet_wire_contract::rfc3339_timestamps::rfc3339_utc;

/// One reusable faction. `doc` is the full faction-library document (validated against
/// the generated contract on every write); `side`/`name` are projections of the same
/// fields for listing and the (owner, name) uniqueness rule.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserFaction {
    /// Primary key (uuid).
    pub id: UserFactionId,
    /// Discord id of the operator who owns the faction.
    pub owner_id: DiscordUserId,
    /// Side key projected from the document: `BLUFOR`, `OPFOR`, `INDFOR` or `CIV`.
    pub side: String,
    /// Display name projected from the document; unique per owner.
    pub name: String,
    /// The full faction-library document, passed through as stored `jsonb`.
    pub doc: RawJson,
    /// When the faction was created (RFC 3339 UTC).
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
    /// When the faction last changed (RFC 3339 UTC).
    #[serde(with = "rfc3339_utc")]
    pub updated_at: DateTime<Utc>,
}
