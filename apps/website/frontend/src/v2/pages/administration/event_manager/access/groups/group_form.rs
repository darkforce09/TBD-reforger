//! A group's name and source as a form holds them, read back into what the backend takes.
//!
//! **Role:** the plain fields of the create and edit forms — name, source kind, partner guild and
//! required roles — the validation that reads them into a name and an [`EventGroupSource`], the
//! change a saved edit sends, and the words a group's source and provenance are shown with.
//! **Position:** under the Groups section of the access panel; the create form and each group card
//! keep one of these in a signal.
//! **Signals & state:** none; pure over the form.
//! **Invariants:** validation mirrors the backend's: a name of 1 to 128 bytes without surrounding
//! whitespace, a guild id of 1 to 128 bytes, and at most 32 role ids, each 1 to 128 bytes. Role ids
//! are typed separated by commas, spaces or new lines and duplicates collapse, since holding a role
//! twice means nothing more. An edit sends only what changed, so saving an untouched form sends
//! nothing at all.

use crate::v2::core::api::dto::{AuthorshipProvenance, EventGroupSource};
use crate::v2::core::utils::utc_timestamp::utc_label;

/// The two kinds of group, as the form's select values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum GroupKind {
    ManagedRoster,
    PartnerGuild,
}

impl GroupKind {
    pub(in super::super) fn wire(self) -> &'static str {
        match self {
            GroupKind::ManagedRoster => "managed_roster",
            GroupKind::PartnerGuild => "partner_guild",
        }
    }

    pub(in super::super) fn from_wire(value: &str) -> Option<Self> {
        match value {
            "managed_roster" => Some(GroupKind::ManagedRoster),
            "partner_guild" => Some(GroupKind::PartnerGuild),
            _ => None,
        }
    }
}

/// The create or edit form's fields.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in super::super) struct GroupForm {
    pub(in super::super) name: String,
    pub(in super::super) kind: GroupKind,
    pub(in super::super) guild_id: String,
    /// Role ids as typed: separated by commas, spaces or new lines.
    pub(in super::super) role_ids: String,
}

impl GroupForm {
    /// An empty form for a new managed roster.
    pub(in super::super) fn blank() -> Self {
        Self {
            name: String::new(),
            kind: GroupKind::ManagedRoster,
            guild_id: String::new(),
            role_ids: String::new(),
        }
    }

    /// A form holding an existing group's name and source.
    pub(in super::super) fn of(name: &str, source: &EventGroupSource) -> Self {
        match source {
            EventGroupSource::ManagedRoster {} => Self {
                name: name.to_string(),
                ..Self::blank()
            },
            EventGroupSource::PartnerGuild {
                guild_id,
                required_role_ids,
            } => Self {
                name: name.to_string(),
                kind: GroupKind::PartnerGuild,
                guild_id: guild_id.clone(),
                role_ids: required_role_ids.join(", "),
            },
        }
    }

    /// The name the backend takes, or what is wrong with it.
    pub(in super::super) fn validated_name(&self) -> Result<String, String> {
        let name = self.name.trim();
        if name.is_empty() || name.len() > 128 {
            return Err("A group name needs 1 to 128 characters".to_string());
        }
        Ok(name.to_string())
    }

    /// The source the backend takes, or what is wrong with it.
    pub(in super::super) fn validated_source(&self) -> Result<EventGroupSource, String> {
        match self.kind {
            GroupKind::ManagedRoster => Ok(EventGroupSource::ManagedRoster {}),
            GroupKind::PartnerGuild => {
                let guild_id = self.guild_id.trim();
                if guild_id.is_empty() || guild_id.len() > 128 {
                    return Err("A partner guild needs its guild id".to_string());
                }
                let roles = parse_role_ids(&self.role_ids);
                if roles.len() > 32 {
                    return Err("A partner group requires at most 32 roles".to_string());
                }
                if roles.iter().any(|role| role.len() > 128) {
                    return Err("A role id holds at most 128 characters".to_string());
                }
                Ok(EventGroupSource::PartnerGuild {
                    guild_id: guild_id.to_string(),
                    required_role_ids: roles,
                })
            }
        }
    }

    /// What a saved edit of the group `name` / `source` sends: the new name and the new source,
    /// each only when it changed; `None` for each means that field is left as it is.
    pub(in super::super) fn changes_from(
        &self,
        name: &str,
        source: &EventGroupSource,
    ) -> Result<(Option<String>, Option<EventGroupSource>), String> {
        let new_name = self.validated_name()?;
        let new_source = self.validated_source()?;
        Ok((
            (new_name != name).then_some(new_name),
            (&new_source != source).then_some(new_source),
        ))
    }
}

/// Role ids typed separated by commas, spaces or new lines, in order, without duplicates.
pub(in super::super) fn parse_role_ids(text: &str) -> Vec<String> {
    let mut roles: Vec<String> = Vec::new();
    for role in text.split(|c: char| c == ',' || c.is_whitespace()) {
        if !role.is_empty() && !roles.iter().any(|seen| seen == role) {
            roles.push(role.to_string());
        }
    }
    roles
}

/// A group's source in words.
pub(in super::super) fn source_line(source: &EventGroupSource) -> String {
    match source {
        EventGroupSource::ManagedRoster {} => {
            "Managed roster: administrators add and remove members".to_string()
        }
        EventGroupSource::PartnerGuild {
            guild_id,
            required_role_ids,
        } if required_role_ids.is_empty() => format!(
            "Partner guild {guild_id}: any verified member of the guild, from bot-verified Discord data"
        ),
        EventGroupSource::PartnerGuild {
            guild_id,
            required_role_ids,
        } => format!(
            "Partner guild {guild_id}: verified members holding every role of {}, from bot-verified \
             Discord data",
            required_role_ids.join(", ")
        ),
    }
}

/// Who or what created or added something, and when: `Rhodes, 2026-07-15 14:20 UTC`.
/// `name_of` turns an account id into a display name when one is known.
pub(in super::super) fn authorship_line(
    account: Option<&str>,
    system_origin: Option<&str>,
    at: &str,
    name_of: impl Fn(&str) -> String,
) -> String {
    let who = match (account, system_origin) {
        (Some(account), _) => name_of(account),
        (None, Some(origin)) => format!("the system ({origin})"),
        (None, None) => "an unknown author".to_string(),
    };
    format!("{who}, {}", utc_label(at))
}

/// A group's provenance in words.
pub(in super::super) fn provenance_line(
    provenance: &AuthorshipProvenance,
    name_of: impl Fn(&str) -> String,
) -> String {
    format!(
        "Created by {}",
        authorship_line(
            provenance.created_by.as_deref(),
            provenance.system_origin.as_deref(),
            &provenance.created_at,
            name_of,
        )
    )
}
