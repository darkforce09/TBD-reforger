//! The committed database reads the Discord procedure's probes run, and the ids they bind.
//!
//! **Role:** the [`DiscordTargets`] every probe closes over (the settings, the operator, the
//! partner guild and role), the committed SELECTs over the membership snapshots, the partner
//! event, the registration, the audit log and the grace override, and the helpers that read their
//! `psql -A -t` rows.
//!
//! **Position:** used by `partner_steps.rs`, `outage_steps.rs`, `rate_limit_steps.rs` and the
//! preflight checks; each read runs through `remote_observers/database_reader.rs`.
//!
//! **Signals & state:** none; constants and pure builders.
//!
//! **Invariants:** every statement is a committed `SELECT` whose values arrive as `:'name'`
//! bindings; times come back as Unix milliseconds of the database's clock, the clock every write
//! the procedure judges was stamped with; the operator's main-guild snapshot is the one whose
//! guild is not the partner guild, because the staging partner guild is the only partner group
//! any event of the operator names.

use std::rc::Rc;

use crate::error::{Result, ResultExt};

use crate::remote_observers::database_reader::{self, CommittedQuery};
use crate::remote_observers::remote_command::RemoteCommand;
use crate::staging_settings::StagingSettings;

/// The ids and settings every Discord probe needs.
#[derive(Debug, Clone)]
pub(super) struct DiscordTargets {
    pub settings: Rc<StagingSettings>,
    /// `TBD_STAGING_OPERATOR_DISCORD_ID`.
    pub operator: String,
    /// `TBD_STAGING_PARTNER_GUILD_ID`.
    pub partner_guild: String,
    /// `TBD_STAGING_PARTNER_ROLE_ID`.
    pub partner_role: String,
}

impl DiscordTargets {
    /// The targets `settings` names; an unset key is an error naming it.
    pub(super) fn from_settings(settings: &StagingSettings) -> Result<Self> {
        Ok(Self {
            operator: settings.operator()?.to_string(),
            partner_guild: settings
                .partner_guild_id
                .clone()
                .context("TBD_STAGING_PARTNER_GUILD_ID is not set in deploy.env")?,
            partner_role: settings
                .partner_role_id
                .clone()
                .context("TBD_STAGING_PARTNER_ROLE_ID is not set in deploy.env")?,
            settings: Rc::new(settings.clone()),
        })
    }

    /// `query` with `bindings`, read-only in the staging database container.
    pub(super) fn select(
        &self,
        query: &CommittedQuery,
        bindings: &[(&str, String)],
    ) -> Result<RemoteCommand> {
        database_reader::select(&self.settings.database_container, query, bindings)
    }
}

/// A snapshot's state: `status|verified_ms|last_error|next_refresh_ms|now_ms` (0 and empty for
/// null).
macro_rules! snapshot_columns {
    () => {
        "membership_status, coalesce(floor(extract(epoch FROM verified_at) * 1000)::bigint, 0), \
         coalesce(last_error, ''), floor(extract(epoch FROM next_refresh_at) * 1000)::bigint, \
         floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint"
    };
}

/// The operator's snapshot in the partner guild.
pub(super) const PARTNER_SNAPSHOT: CommittedQuery = CommittedQuery {
    name: "partner_snapshot",
    sql: concat!(
        "SELECT ",
        snapshot_columns!(),
        " FROM discord_membership_snapshots WHERE discord_id = :'discord_id' \
         AND guild_id = :'partner_guild_id'"
    ),
    parameters: &["discord_id", "partner_guild_id"],
};

/// The operator's snapshot in the main guild.
pub(super) const MAIN_SNAPSHOT: CommittedQuery = CommittedQuery {
    name: "main_snapshot",
    sql: concat!(
        "SELECT ",
        snapshot_columns!(),
        " FROM discord_membership_snapshots WHERE discord_id = :'discord_id' \
         AND guild_id <> :'partner_guild_id'"
    ),
    parameters: &["discord_id", "partner_guild_id"],
};

/// The newest undeleted partner event named `title` with a partner group of the partner guild:
/// `event_id|group_created_ms`.
pub(super) const PARTNER_EVENT: CommittedQuery = CommittedQuery {
    name: "partner_event",
    sql: "SELECT e.id, floor(extract(epoch FROM g.created_at) * 1000)::bigint \
          FROM event_groups g JOIN events e ON e.id = g.event_id \
          WHERE e.name_override = :'title' AND e.deleted_at IS NULL AND g.deleted_at IS NULL \
          AND g.source->>'kind' = 'partner_guild' AND g.source->>'guild_id' = :'partner_guild_id' \
          ORDER BY g.created_at DESC LIMIT 1",
    parameters: &["title", "partner_guild_id"],
};

/// The operator's newest registration on the event:
/// `registration_id|reservation_state|release_reason|withdrawn_ms`.
pub(super) const OPERATOR_REGISTRATION: CommittedQuery = CommittedQuery {
    name: "operator_registration",
    sql: "SELECT r.id, r.reservation_state, coalesce(r.release_reason, ''), \
          coalesce(floor(extract(epoch FROM r.withdrawn_at) * 1000)::bigint, 0) \
          FROM event_registrations r JOIN event_missions m ON m.id = r.event_mission_id \
          WHERE m.event_id = :'event_id'::uuid AND r.discord_id = :'discord_id' \
          ORDER BY r.registered_at DESC LIMIT 1",
    parameters: &["event_id", "discord_id"],
};

/// Audit rows of one action and target from a Unix millisecond on: `count|newest_ms`.
pub(super) const AUDIT_ROWS_SINCE: CommittedQuery = CommittedQuery {
    name: "audit_rows_since",
    sql: "SELECT count(*), coalesce(floor(extract(epoch FROM max(created_at)) * 1000)::bigint, 0) \
          FROM audit_logs WHERE action = :'action' AND target_type = :'target_type' \
          AND target_id = :'target_id' \
          AND created_at >= to_timestamp(:'since_ms'::bigint / 1000.0)",
    parameters: &["action", "target_type", "target_id", "since_ms"],
};

/// The operator's grace override: `created_ms|expires_ms|authorized_by`.
pub(super) const GRACE_OVERRIDE: CommittedQuery = CommittedQuery {
    name: "grace_override",
    sql: "SELECT floor(extract(epoch FROM created_at) * 1000)::bigint, \
          floor(extract(epoch FROM expires_at) * 1000)::bigint, authorized_by \
          FROM discord_membership_grace_overrides WHERE discord_id = :'discord_id' \
          AND guild_id <> :'partner_guild_id'",
    parameters: &["discord_id", "partner_guild_id"],
};

/// Whether the operator holds the partner role in the platform's copy: `count`.
pub(super) const PARTNER_ROLE_ROWS: CommittedQuery = CommittedQuery {
    name: "partner_role_rows",
    sql: "SELECT count(*) FROM user_discord_roles WHERE discord_id = :'discord_id' \
          AND guild_id = :'partner_guild_id' AND discord_role_id = :'role_id'",
    parameters: &["discord_id", "partner_guild_id", "role_id"],
};

/// The event's deletion time: `deleted_ms` (0 while it exists).
pub(super) const EVENT_DELETION: CommittedQuery = CommittedQuery {
    name: "event_deletion",
    sql: "SELECT coalesce(floor(extract(epoch FROM deleted_at) * 1000)::bigint, 0) \
          FROM events WHERE id = :'event_id'::uuid",
    parameters: &["event_id"],
};

/// The first row of `psql -A -t` output.
pub(super) fn first_row(output: &str) -> Option<Vec<String>> {
    database_reader::rows(output).into_iter().next()
}

/// A Unix millisecond column; 0 means null.
pub(super) fn millis(field: Option<&String>) -> Option<u64> {
    field.and_then(|value| value.trim().parse().ok())
}

/// One snapshot row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SnapshotRow {
    pub status: String,
    /// 0 when never verified.
    pub verified_ms: u64,
    pub last_error: String,
    pub next_refresh_ms: u64,
    /// The database's clock when it answered.
    pub now_ms: u64,
}

/// The snapshot row of `output`, when there is one.
pub(super) fn snapshot(output: &str) -> Option<SnapshotRow> {
    let row = first_row(output)?;
    Some(SnapshotRow {
        status: row.first()?.clone(),
        verified_ms: millis(row.get(1))?,
        last_error: row.get(2)?.clone(),
        next_refresh_ms: millis(row.get(3))?,
        now_ms: millis(row.get(4))?,
    })
}
