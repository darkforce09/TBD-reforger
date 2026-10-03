//! Explicit event access grants. Missing child policies inherit; an empty grant list closes access.

use api_identifiers::EventGroupId;
use serde::{Deserialize, Serialize};

/// Who may see and reserve an event, squad or seat: any one grant admits an account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventAccessPolicy {
    /// Alternative grants, at most 32; an empty list admits no account.
    pub grants: Vec<EventAccessGrant>,
}

/// One alternative of a policy: it admits an account when every condition holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventAccessGrant {
    /// Conditions that must all hold, 1 to 16.
    pub conditions: Vec<EventAccessCondition>,
}

/// One fact about an account a grant can require; the wire object names it in `kind`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum EventAccessCondition {
    /// The account is signed in.
    Authenticated {},
    /// The account is a verified TBD member.
    TbdMember {},
    /// The account holds a role in a Discord guild.
    DiscordRole {
        /// The Discord guild that grants the role.
        guild_id: String,
        /// The Discord role the account must hold there.
        role_id: String,
    },
    /// The account belongs to an event group.
    EventGroup {
        /// The event group whose members the condition admits.
        group_id: EventGroupId,
    },
    /// The account is one named Discord account.
    NamedAccount {
        /// The Discord account the condition admits.
        discord_id: String,
    },
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
                    EventAccessCondition::EventGroup { group_id }
                        if group_id.as_uuid().is_nil() =>
                    {
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
