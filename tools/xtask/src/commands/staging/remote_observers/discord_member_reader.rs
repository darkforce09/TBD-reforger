//! The bot's read of the operator's Discord member record, made on the staging host.
//!
//! **Role:** builds the member read the Discord procedure polls (role present or gone) and parses
//! the tool's `discord-member-read {json}` answer line into a [`MemberReadLine`].
//!
//! **Position:** runs `staging-fixtures observe-discord-member`
//! (`apps/api/src/bin/staging_fixtures/discord_member_probe/`) through
//! `remote_actions/host_fixture_commands.rs`; used by the Discord procedure's steps and
//! `staging preflight --discord`.
//!
//! **Signals & state:** none; pure builder and parser.
//!
//! **Invariants:** the bot token stays on the host (the tool reads it from the API env file);
//! the read changes nothing, so it may run in preflight; a line counts only when it starts with
//! the prefix and its JSON carries every field a judge reads (a missing field is no answer, never
//! a default); of several read lines the last one counts, because the tool prints one per request
//! in order.

use serde::Deserialize;

pub(crate) use crate::commands::staging::remote_actions::host_fixture_commands::GuildScope;
use crate::commands::staging::remote_actions::host_fixture_commands::observe_discord_member;
use crate::commands::staging::remote_observers::remote_command::RemoteCommand;
use crate::commands::staging::staging_settings::StagingSettings;

/// The prefix of the line one member read prints.
const MEMBER_READ_PREFIX: &str = "discord-member-read ";

/// One bot read of a guild member, as the host tool printed it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub(crate) struct MemberReadLine {
    /// The guild's Discord id.
    pub guild_id: String,
    /// When Discord's answer arrived, host clock.
    pub answered_at_unix_ms: u64,
    /// `member`, `nonmember`, `rate_limited` or `unavailable`.
    pub outcome: String,
    /// The member's role ids, for a `member` answer.
    pub roles: Option<Vec<String>>,
}

impl MemberReadLine {
    /// Whether the member holds `role`: `None` when the answer is not a membership.
    pub(crate) fn holds_role(&self, role: &str) -> Option<bool> {
        match self.outcome.as_str() {
            "member" => Some(self.roles.iter().flatten().any(|held| held == role)),
            "nonmember" => Some(false),
            _ => None,
        }
    }
}

/// The member read in `guild`.
pub(crate) fn member(settings: &StagingSettings, guild: GuildScope) -> RemoteCommand {
    RemoteCommand {
        observer: "discord member read",
        ..observe_discord_member(settings, guild)
    }
}

/// The last `discord-member-read {json}` line of `output`; other lines are ignored.
pub(crate) fn member_read(output: &str) -> Option<MemberReadLine> {
    output.lines().rev().find_map(|line| {
        serde_json::from_str(line.trim_end().strip_prefix(MEMBER_READ_PREFIX)?).ok()
    })
}
