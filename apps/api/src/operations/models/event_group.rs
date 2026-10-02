//! A manager-maintained roster and verified partner membership have explicit, distinct provenance.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::event_access_policy::valid_identity;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventGroupSource {
    ManagedRoster {},
    PartnerGuild {
        guild_id: String,
        required_role_ids: Vec<String>,
    },
}

impl EventGroupSource {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventGroup {
    pub id: Uuid,
    pub event_id: Uuid,
    pub name: String,
    pub source: EventGroupSource,
}
