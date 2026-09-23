//! An access policy as its editor holds it, the edits made to it, and the policy read back out.
//!
//! **Role:** the plain-data model behind the policy editor — each grant with its conditions, each
//! condition with its kind and the text of every field a kind can need — with the structural edits
//! (add and remove a grant or a condition, change a kind, type into a field), the validation that
//! reads a draft back into the policy the backend takes, and the one-line summaries the policy lists
//! show.
//! **Position:** under the policy editor view, which keeps a draft in a signal and calls these.
//! **Signals & state:** none; every function is pure over the draft it is handed.
//! **Invariants:** every grant and condition carries an id unique within its draft, so a keyed list
//! keeps a row — and the input the operator is typing into — across every other edit. A kind change
//! keeps the text typed for the other kinds, so switching back loses nothing; only the chosen kind's
//! fields are read back. Validation mirrors the backend's bounds — at most 32 grants, one to sixteen
//! conditions each, identifiers of 1 to 128 bytes with no control characters — and trims the
//! surrounding whitespace the backend would refuse. A draft with no grants is valid: it admits
//! nobody, which is a different decision from inheriting.

use crate::v2::core::api::dto::{
    EventAccessCondition, EventAccessGrant, EventAccessPolicy, EventGroupView,
};

/// The most alternatives a policy may hold.
pub(super) const MAX_GRANTS: usize = 32;
/// The most conditions one grant may require.
pub(super) const MAX_CONDITIONS: usize = 16;

/// The five kinds of condition, in the order the editor offers them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum ConditionKind {
    Authenticated,
    TbdMember,
    DiscordRole,
    EventGroup,
    NamedAccount,
}

impl ConditionKind {
    /// Every kind, in offer order.
    pub(super) const ALL: [ConditionKind; 5] = [
        ConditionKind::TbdMember,
        ConditionKind::EventGroup,
        ConditionKind::DiscordRole,
        ConditionKind::NamedAccount,
        ConditionKind::Authenticated,
    ];

    /// The wire spelling, which is also the editor's select value.
    pub(super) fn wire(self) -> &'static str {
        match self {
            ConditionKind::Authenticated => "authenticated",
            ConditionKind::TbdMember => "tbd_member",
            ConditionKind::DiscordRole => "discord_role",
            ConditionKind::EventGroup => "event_group",
            ConditionKind::NamedAccount => "named_account",
        }
    }

    /// The kind a select value names.
    pub(super) fn from_wire(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.wire() == value)
    }

    /// How the editor names the kind.
    pub(super) fn label(self) -> &'static str {
        match self {
            ConditionKind::Authenticated => "Any signed-in account",
            ConditionKind::TbdMember => "Verified TBD member",
            ConditionKind::DiscordRole => "Holds a Discord role",
            ConditionKind::EventGroup => "Member of an event group",
            ConditionKind::NamedAccount => "One named account",
        }
    }
}

/// Which identifier of a condition a text field holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ConditionIdentifier {
    /// The Discord guild of a role condition.
    Guild,
    /// The Discord role of a role condition.
    Role,
    /// The event group of a group condition.
    Group,
    /// The Discord account of a named-account condition.
    Account,
}

/// One condition as the editor holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ConditionFields {
    pub(super) id: u64,
    pub(super) kind: ConditionKind,
    pub(super) guild_id: String,
    pub(super) role_id: String,
    pub(super) group_id: String,
    pub(super) discord_id: String,
}

impl ConditionFields {
    fn blank(id: u64, kind: ConditionKind) -> Self {
        Self {
            id,
            kind,
            guild_id: String::new(),
            role_id: String::new(),
            group_id: String::new(),
            discord_id: String::new(),
        }
    }

    /// The text of one field.
    pub(super) fn field(&self, field: ConditionIdentifier) -> &str {
        match field {
            ConditionIdentifier::Guild => &self.guild_id,
            ConditionIdentifier::Role => &self.role_id,
            ConditionIdentifier::Group => &self.group_id,
            ConditionIdentifier::Account => &self.discord_id,
        }
    }

    fn field_mut(&mut self, field: ConditionIdentifier) -> &mut String {
        match field {
            ConditionIdentifier::Guild => &mut self.guild_id,
            ConditionIdentifier::Role => &mut self.role_id,
            ConditionIdentifier::Group => &mut self.group_id,
            ConditionIdentifier::Account => &mut self.discord_id,
        }
    }
}

/// One grant as the editor holds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct GrantFields {
    pub(super) id: u64,
    pub(super) conditions: Vec<ConditionFields>,
}

/// The next id of a draft, advanced by every row the draft gains.
fn next(ids: &mut u64) -> u64 {
    *ids += 1;
    *ids
}

/// A policy's grants as editor rows.
pub(super) fn draft_from_policy(policy: &EventAccessPolicy, ids: &mut u64) -> Vec<GrantFields> {
    policy
        .grants
        .iter()
        .map(|grant| GrantFields {
            id: next(ids),
            conditions: grant
                .conditions
                .iter()
                .map(|condition| {
                    let id = next(ids);
                    match condition {
                        EventAccessCondition::Authenticated {} => {
                            ConditionFields::blank(id, ConditionKind::Authenticated)
                        }
                        EventAccessCondition::TbdMember {} => {
                            ConditionFields::blank(id, ConditionKind::TbdMember)
                        }
                        EventAccessCondition::DiscordRole { guild_id, role_id } => {
                            ConditionFields {
                                guild_id: guild_id.clone(),
                                role_id: role_id.clone(),
                                ..ConditionFields::blank(id, ConditionKind::DiscordRole)
                            }
                        }
                        EventAccessCondition::EventGroup { group_id } => ConditionFields {
                            group_id: group_id.clone(),
                            ..ConditionFields::blank(id, ConditionKind::EventGroup)
                        },
                        EventAccessCondition::NamedAccount { discord_id } => ConditionFields {
                            discord_id: discord_id.clone(),
                            ..ConditionFields::blank(id, ConditionKind::NamedAccount)
                        },
                    }
                })
                .collect(),
        })
        .collect()
}

/// Add an alternative grant holding one verified-member condition, the most common start.
pub(super) fn add_grant(grants: &mut Vec<GrantFields>, ids: &mut u64) {
    let id = next(ids);
    let condition = ConditionFields::blank(next(ids), ConditionKind::TbdMember);
    grants.push(GrantFields {
        id,
        conditions: vec![condition],
    });
}

/// Remove one grant.
pub(super) fn remove_grant(grants: &mut Vec<GrantFields>, grant: u64) {
    grants.retain(|g| g.id != grant);
}

/// Add a condition to one grant.
pub(super) fn add_condition(grants: &mut [GrantFields], grant: u64, ids: &mut u64) {
    if let Some(g) = grants.iter_mut().find(|g| g.id == grant) {
        g.conditions
            .push(ConditionFields::blank(next(ids), ConditionKind::TbdMember));
    }
}

/// Remove one condition. A grant left without conditions is removed with it, because a grant
/// requires at least one condition.
pub(super) fn remove_condition(grants: &mut Vec<GrantFields>, grant: u64, condition: u64) {
    if let Some(g) = grants.iter_mut().find(|g| g.id == grant) {
        g.conditions.retain(|c| c.id != condition);
    }
    grants.retain(|g| !g.conditions.is_empty());
}

/// Change a condition's kind, keeping every field's text.
pub(super) fn set_kind(grants: &mut [GrantFields], condition: u64, kind: ConditionKind) {
    if let Some(c) = find_condition(grants, condition) {
        c.kind = kind;
    }
}

/// Write one field of a condition.
pub(super) fn set_field(
    grants: &mut [GrantFields],
    condition: u64,
    field: ConditionIdentifier,
    value: String,
) {
    if let Some(c) = find_condition(grants, condition) {
        *c.field_mut(field) = value;
    }
}

fn find_condition(grants: &mut [GrantFields], condition: u64) -> Option<&mut ConditionFields> {
    grants
        .iter_mut()
        .flat_map(|g| g.conditions.iter_mut())
        .find(|c| c.id == condition)
}

/// One identifier as the backend takes it: trimmed, 1 to 128 bytes, no control characters.
fn identifier(value: &str, what: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{what} is required"));
    }
    if value.len() > 128 || value.chars().any(char::is_control) {
        return Err(format!(
            "{what} must be 1 to 128 characters without control characters"
        ));
    }
    Ok(value.to_string())
}

/// Read a draft back into the policy the backend takes, or say what is wrong with it.
pub(super) fn policy_from_draft(grants: &[GrantFields]) -> Result<EventAccessPolicy, String> {
    if grants.len() > MAX_GRANTS {
        return Err(format!(
            "A policy holds at most {MAX_GRANTS} alternative grants"
        ));
    }
    let mut out = Vec::with_capacity(grants.len());
    for (index, grant) in grants.iter().enumerate() {
        let label = format!("Grant {}", index + 1);
        if grant.conditions.is_empty() || grant.conditions.len() > MAX_CONDITIONS {
            return Err(format!("{label} needs 1 to {MAX_CONDITIONS} conditions"));
        }
        let mut conditions = Vec::with_capacity(grant.conditions.len());
        for c in &grant.conditions {
            conditions.push(match c.kind {
                ConditionKind::Authenticated => EventAccessCondition::Authenticated {},
                ConditionKind::TbdMember => EventAccessCondition::TbdMember {},
                ConditionKind::DiscordRole => EventAccessCondition::DiscordRole {
                    guild_id: identifier(&c.guild_id, &format!("{label}: the guild id"))?,
                    role_id: identifier(&c.role_id, &format!("{label}: the role id"))?,
                },
                ConditionKind::EventGroup => EventAccessCondition::EventGroup {
                    group_id: identifier(&c.group_id, &format!("{label}: the group"))?,
                },
                ConditionKind::NamedAccount => EventAccessCondition::NamedAccount {
                    discord_id: identifier(&c.discord_id, &format!("{label}: the account id"))?,
                },
            });
        }
        out.push(EventAccessGrant { conditions });
    }
    Ok(EventAccessPolicy { grants: out })
}

/// One condition in words: `Verified TBD member`, `Member of Byte Parity roster`.
pub(super) fn condition_summary(
    condition: &EventAccessCondition,
    groups: &[EventGroupView],
) -> String {
    match condition {
        EventAccessCondition::Authenticated {} => "Any signed-in account".to_string(),
        EventAccessCondition::TbdMember {} => "Verified TBD member".to_string(),
        EventAccessCondition::DiscordRole { guild_id, role_id } => {
            format!("Holds role {role_id} in guild {guild_id}")
        }
        EventAccessCondition::EventGroup { group_id } => {
            match groups.iter().find(|group| &group.id == group_id) {
                Some(group) => format!("Member of {}", group.name),
                None => format!("Member of group {group_id}"),
            }
        }
        EventAccessCondition::NamedAccount { discord_id } => format!("Account {discord_id}"),
    }
}

/// A whole policy in words: its alternatives joined by "or", each grant's conditions by "and".
pub(super) fn policy_summary(policy: &EventAccessPolicy, groups: &[EventGroupView]) -> String {
    if policy.grants.is_empty() {
        return "Admits nobody".to_string();
    }
    let alternatives: Vec<String> = policy
        .grants
        .iter()
        .map(|grant| {
            grant
                .conditions
                .iter()
                .map(|c| condition_summary(c, groups))
                .collect::<Vec<_>>()
                .join(" and ")
        })
        .collect();
    format!("Admits: {}", alternatives.join("; or "))
}
