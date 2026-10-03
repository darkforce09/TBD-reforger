//! A manager-maintained roster and verified partner membership have explicit, distinct provenance.

use api_identifiers::{EventGroupId, EventId};
use serde::{Deserialize, Serialize};

use super::event_access_policy::valid_identity;

/// Where an event group's membership comes from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventGroupSource {
    /// Members are the accounts a manager lists on the group's roster.
    ManagedRoster {},
    /// Members are verified members of a partner Discord guild holding every required role.
    PartnerGuild {
        /// The partner Discord guild.
        guild_id: String,
        /// Roles a member must all hold in that guild; empty admits every verified member.
        required_role_ids: Vec<String>,
    },
}

impl EventGroupSource {
    /// Checks the partner guild ID and at most 32 role IDs are nonempty and unpadded.
    pub fn validate(&self) -> Result<(), &'static str> {
        if let Self::PartnerGuild {
            guild_id,
            required_role_ids,
        } = self
        {
            if !valid_identity(guild_id) {
                return Err("partner guild ID must contain 1 to 128 unpadded bytes");
            }
            if required_role_ids.len() > 32
                || required_role_ids.iter().any(|id| !valid_identity(id))
            {
                return Err("partner roles allow at most 32 nonempty, unpadded role IDs");
            }
        }
        Ok(())
    }
}

/// One named group of accounts that an event's access grants can refer to.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventGroup {
    /// Group identifier.
    pub id: EventGroupId,
    /// The event that owns the group.
    pub event_id: EventId,
    /// Display name shown to managers.
    pub name: String,
    /// Where the group's membership comes from.
    pub source: EventGroupSource,
}
