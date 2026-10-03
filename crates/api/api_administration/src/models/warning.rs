//! The disciplinary warning record.
//!
//! @contract personnel-actions.schema.json#/definitions/Warning

use api_identifiers::{DiscordUserId, WarningId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use fleet_wire_contract::rfc3339_timestamps::rfc3339_utc;

/// Disciplinary record; the Personnel Roster "Warnings" column counts these.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Warning {
    /// The warning row id.
    pub id: WarningId,
    /// The warned member's Discord account id.
    pub discord_id: DiscordUserId,
    /// The Discord account id of the administrator who issued it.
    pub issued_by: String,
    /// Why the warning was issued.
    pub reason: String,
    /// When the warning was issued, in RFC 3339 UTC on the wire.
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
}
