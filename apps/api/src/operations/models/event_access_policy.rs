//! Explicit event access grants. Missing child policies inherit; an empty grant list closes access.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventAccessPolicy {
    pub grants: Vec<EventAccessGrant>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventAccessGrant {
    pub conditions: Vec<EventAccessCondition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventAccessCondition {
    Authenticated {},
    TbdMember {},
    DiscordRole { guild_id: String, role_id: String },
    EventGroup { group_id: Uuid },
    NamedAccount { discord_id: String },
}

impl Default for EventAccessPolicy {
    fn default() -> Self {
        Self {
            grants: vec![EventAccessGrant {
                conditions: vec![EventAccessCondition::TbdMember {}],
            }],
        }
    }
}

impl EventAccessPolicy {
    /// Bounds limit authorization work and reject an accidental unconditional empty grant.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.grants.len() > 32 {
            return Err("access policy allows at most 32 alternative grants");
        }
        for grant in &self.grants {
            if grant.conditions.is_empty() || grant.conditions.len() > 16 {
                return Err("each access grant requires 1 to 16 conditions");
            }
            for condition in &grant.conditions {
                match condition {
                    EventAccessCondition::DiscordRole { guild_id, role_id }
                        if !valid_identity(guild_id) || !valid_identity(role_id) =>
                    {
                        return Err(
                            "Discord guild and role IDs must contain 1 to 128 unpadded bytes",
                        );
                    }
                    EventAccessCondition::NamedAccount { discord_id }
                        if !valid_identity(discord_id) =>
                    {
                        return Err("account IDs must contain 1 to 128 unpadded bytes");
                    }
                    EventAccessCondition::EventGroup { group_id } if group_id.is_nil() => {
                        return Err("event group ID must not be nil");
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn valid_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

#[cfg(test)]
#[path = "tests/event_access_policy.rs"]
mod tests;
