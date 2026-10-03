//! Membership facts for many accounts at once, under both evidence standards eligibility uses.
//!
//! Current authority admits only snapshots that are fresh, inside the 48-hour grace period, or
//! extended by an audited override. Last verified facts keep the most recent verified member
//! observation regardless of age: they decide whether eligibility loss is confirmed, so stale
//! data never evicts a reservation. Unverified guilds never grant access under either standard.

use api_identifiers::{DiscordGuildId, DiscordUserId, EventGroupId, EventId};
use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use sqlx::PgConnection;

use super::context::EventAccessContext;
use super::evaluation::{EventAccessSubject, policy_admits};
use crate::models::event_access_policy::{EventAccessCondition, EventAccessPolicy};
use crate::models::event_group::{EventGroup, EventGroupSource};
use api_caller_identity::UserRole;
use api_caller_identity::cached_membership_permissions::evaluate_cached_membership_permissions;
use api_foundation::error_handling::api_error::ApiError;

/// Which membership observations may supply grants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MembershipEvidence {
    /// Only fresh, in-grace or overridden snapshots grant; decides admission.
    CurrentAuthority,
    /// The latest verified member observation of any age grants; confirms eligibility loss.
    LastVerifiedFacts,
}

/// One guild observation as recorded by bot-authenticated REST reconciliation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildEvidence {
    /// Observed status: `unknown`, `member` or `nonmember`.
    pub membership_status: String,
    /// When the observation was verified; `None` while it is still `unknown`.
    pub verified_at: Option<DateTime<Utc>>,
    /// Expiry of an audited grace override for this guild, if one exists.
    pub override_until: Option<DateTime<Utc>>,
    /// The observation currently authorizes grants (fresh, within grace, or overridden).
    pub current: bool,
}

/// A manager-maintained roster entry, authored by a manager or a named system transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterEvidence {
    /// Discord id of the manager who added the entry; `None` for a system transition.
    pub added_by: Option<String>,
    /// Name of the system transition that added the entry; `None` for a manager entry.
    pub system_origin: Option<String>,
    /// When the account joined the roster.
    pub added_at: DateTime<Utc>,
}

/// One account's membership facts for one event, under both evidence standards.
#[derive(Debug, Clone)]
pub struct AccountEligibilityFacts {
    /// Account the facts describe.
    pub discord_id: DiscordUserId,
    /// Not banned and not deleted, read after the caller's account lock.
    pub available: bool,
    /// Grants under current authority (`MembershipEvidence::CurrentAuthority`).
    pub current: EventAccessSubject,
    /// Grants under last verified facts (`MembershipEvidence::LastVerifiedFacts`).
    pub last_verified: EventAccessSubject,
    /// Applicable guilds with no verified observation yet; pending checks may still grant them.
    pub unverified_guilds: BTreeSet<String>,
    /// Per applicable guild id, the account's recorded observation.
    pub guilds: BTreeMap<String, GuildEvidence>,
    /// Managed-roster groups of the event listing the account, with each entry's provenance.
    pub roster_groups: BTreeMap<EventGroupId, RosterEvidence>,
    /// Partner groups whose guild has not been verified for this account yet.
    pub pending_partner_groups: BTreeSet<EventGroupId>,
    /// Database clock instant the facts were read at.
    pub observed_at: DateTime<Utc>,
}

impl AccountEligibilityFacts {
    /// The subject that supplies grants under `evidence`.
    pub fn subject(&self, evidence: MembershipEvidence) -> &EventAccessSubject {
        match evidence {
            MembershipEvidence::CurrentAuthority => &self.current,
            MembershipEvidence::LastVerifiedFacts => &self.last_verified,
        }
    }

    /// Stale or still-pending observations could satisfy `policy`. This is used to ask for
    /// verification instead of reporting a plain denial; it never grants access by itself.
    pub fn pending_evidence_admits(
        &self,
        policy: &EventAccessPolicy,
        main_guild: &DiscordGuildId,
    ) -> bool {
        let mut optimistic = self.last_verified.clone();
        if self.unverified_guilds.contains(main_guild.as_str()) {
            optimistic.tbd_member = true;
        }
        for condition in policy.grants.iter().flat_map(|grant| &grant.conditions) {
            if let EventAccessCondition::DiscordRole { guild_id, role_id } = condition
                && self.unverified_guilds.contains(guild_id)
            {
                optimistic
                    .guild_roles
                    .entry(guild_id.clone())
                    .or_default()
                    .insert(role_id.clone());
            }
        }
        optimistic
            .event_groups
            .extend(self.pending_partner_groups.iter().copied());
        policy_admits(policy, &optimistic)
    }
}

/// One verified-or-pending guild observation with its roles and any audited override.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MembershipSnapshotRow {
    /// Account the observation is about.
    pub discord_id: DiscordUserId,
    /// Guild the observation is about.
    pub guild_id: DiscordGuildId,
    /// Observed status: `unknown`, `member` or `nonmember`.
    pub membership_status: String,
    /// When the observation was verified; `None` while it is still `unknown`.
    pub verified_at: Option<DateTime<Utc>>,
    /// Expiry of an audited grace override (column alias `override_until`), if one exists.
    pub override_until: Option<DateTime<Utc>>,
    /// Role ids the account holds in the guild, in byte order.
    pub roles: Vec<String>,
}

/// An active managed-roster entry and its provenance.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RosterEntryRow {
    /// Event that owns the group.
    pub event_id: EventId,
    /// Account listed on the roster.
    pub discord_id: DiscordUserId,
    /// Managed-roster group the entry belongs to.
    pub group_id: EventGroupId,
    /// Discord id of the manager who added the entry; `None` for a system transition.
    pub added_by: Option<String>,
    /// Name of the system transition that added the entry; `None` for a manager entry.
    pub system_origin: Option<String>,
    /// When the account joined the roster.
    pub added_at: DateTime<Utc>,
}

/// Snapshots of `accounts` for `guilds`, with roles in byte order.
pub async fn load_membership_snapshots(
    connection: &mut PgConnection,
    accounts: &[DiscordUserId],
    guilds: &[String],
) -> Result<Vec<MembershipSnapshotRow>, ApiError> {
    Ok(sqlx::query_as(
        "SELECT s.discord_id, s.guild_id, s.membership_status, s.verified_at, o.expires_at AS override_until,
            ARRAY(SELECT r.discord_role_id FROM user_discord_roles r
                WHERE r.discord_id = s.discord_id AND r.guild_id = s.guild_id
                ORDER BY r.discord_role_id COLLATE \"C\") AS roles
         FROM discord_membership_snapshots s LEFT JOIN discord_membership_grace_overrides o
            ON o.discord_id = s.discord_id AND o.guild_id = s.guild_id
         WHERE s.discord_id = ANY($1) AND s.guild_id = ANY($2)",
    )
    .bind(accounts)
    .bind(guilds)
    .fetch_all(connection)
    .await?)
}

/// Active roster entries of `accounts` in the non-deleted groups of `events`.
pub async fn load_roster_entries(
    connection: &mut PgConnection,
    events: &[EventId],
    accounts: &[DiscordUserId],
) -> Result<Vec<RosterEntryRow>, ApiError> {
    Ok(sqlx::query_as(
        "SELECT g.event_id, r.discord_id, r.group_id, r.added_by, r.system_origin, r.added_at
         FROM event_group_roster r JOIN event_groups g ON g.id = r.group_id
         WHERE g.event_id = ANY($1) AND g.deleted_at IS NULL AND r.discord_id = ANY($2)
            AND r.removed_at IS NULL",
    )
    .bind(events)
    .bind(accounts)
    .fetch_all(connection)
    .await?)
}

/// The groups and guilds one event's policies can rely on.
pub struct EventMembershipScope<'a> {
    /// The event's non-deleted event groups.
    pub groups: &'a [EventGroup],
    /// The TBD guild plus every partner guild the event's groups reference.
    pub applicable_guilds: &'a BTreeSet<String>,
    /// The TBD guild id; membership there makes the account a TBD member.
    pub main_guild: &'a DiscordGuildId,
}

/// Pure construction of one account's facts for one event's groups and applicable guilds.
pub fn build_account_facts<'a>(
    discord_id: &DiscordUserId,
    available: bool,
    now: DateTime<Utc>,
    snapshots: impl Iterator<Item = &'a MembershipSnapshotRow>,
    roster: impl Iterator<Item = &'a RosterEntryRow>,
    scope: &EventMembershipScope<'_>,
) -> AccountEligibilityFacts {
    let (groups, applicable_guilds, main_guild) =
        (scope.groups, scope.applicable_guilds, scope.main_guild);
    let mut current = EventAccessSubject {
        discord_id: discord_id.to_owned(),
        ..Default::default()
    };
    let mut last_verified = current.clone();
    let mut account_guilds = BTreeMap::new();
    for snapshot in snapshots.filter(|row| applicable_guilds.contains(row.guild_id.as_str())) {
        let member = snapshot.membership_status == "member";
        let decision = evaluate_cached_membership_permissions(
            now,
            snapshot.verified_at,
            UserRole::Enlisted,
            !member,
            snapshot.override_until,
            false,
        );
        let is_current = decision.effective_role == Some(UserRole::Enlisted);
        let roles: BTreeSet<String> = snapshot.roles.iter().cloned().collect();
        if is_current {
            current
                .guild_roles
                .insert(snapshot.guild_id.to_string(), roles.clone());
        }
        if member && snapshot.verified_at.is_some() {
            last_verified
                .guild_roles
                .insert(snapshot.guild_id.to_string(), roles);
        }
        account_guilds.insert(
            snapshot.guild_id.to_string(),
            GuildEvidence {
                membership_status: snapshot.membership_status.clone(),
                verified_at: snapshot.verified_at,
                override_until: snapshot.override_until,
                current: is_current,
            },
        );
    }
    let unverified_guilds: BTreeSet<String> = applicable_guilds
        .iter()
        .filter(|guild| {
            account_guilds
                .get(*guild)
                .is_none_or(|evidence: &GuildEvidence| evidence.verified_at.is_none())
        })
        .cloned()
        .collect();
    let roster_groups: BTreeMap<EventGroupId, RosterEvidence> = roster
        .map(|row| {
            (
                row.group_id,
                RosterEvidence {
                    added_by: row.added_by.clone(),
                    system_origin: row.system_origin.clone(),
                    added_at: row.added_at,
                },
            )
        })
        .collect();
    current.tbd_member = current.guild_roles.contains_key(main_guild.as_str());
    last_verified.tbd_member = last_verified.guild_roles.contains_key(main_guild.as_str());
    let mut pending_partner_groups = BTreeSet::new();
    for group in groups {
        let (in_current, in_verified) = match &group.source {
            EventGroupSource::ManagedRoster {} => {
                let listed = roster_groups.contains_key(&group.id);
                (listed, listed)
            }
            EventGroupSource::PartnerGuild {
                guild_id,
                required_role_ids,
            } => {
                if unverified_guilds.contains(guild_id) {
                    pending_partner_groups.insert(group.id);
                }
                let holds = |subject: &EventAccessSubject| {
                    subject.guild_roles.get(guild_id).is_some_and(|roles| {
                        required_role_ids.iter().all(|role| roles.contains(role))
                    })
                };
                (holds(&current), holds(&last_verified))
            }
        };
        if in_current {
            current.event_groups.insert(group.id);
        }
        if in_verified {
            last_verified.event_groups.insert(group.id);
        }
    }
    AccountEligibilityFacts {
        discord_id: discord_id.to_owned(),
        available,
        current,
        last_verified,
        unverified_guilds,
        guilds: account_guilds,
        roster_groups,
        pending_partner_groups,
        observed_at: now,
    }
}

impl EventAccessContext {
    /// Batch-load facts for `accounts`. Callers performing mutations hold every account lock.
    /// Accounts absent from `users` are absent from the result.
    pub async fn account_facts(
        &self,
        connection: &mut PgConnection,
        accounts: &[DiscordUserId],
        main_guild: &DiscordGuildId,
    ) -> Result<BTreeMap<DiscordUserId, AccountEligibilityFacts>, ApiError> {
        let rows: Vec<(DiscordUserId, bool, DateTime<Utc>)> = sqlx::query_as(
            "SELECT discord_id, NOT is_banned AND deleted_at IS NULL, clock_timestamp()
             FROM users WHERE discord_id = ANY($1)",
        )
        .bind(accounts)
        .fetch_all(&mut *connection)
        .await?;
        let applicable = self.applicable_guilds(main_guild);
        let guilds: Vec<String> = applicable.iter().cloned().collect();
        let snapshots = load_membership_snapshots(connection, accounts, &guilds).await?;
        let roster = load_roster_entries(connection, &[self.event_id], accounts).await?;
        Ok(rows
            .into_iter()
            .map(|(discord_id, available, now)| {
                let facts = build_account_facts(
                    &discord_id,
                    available,
                    now,
                    snapshots.iter().filter(|row| row.discord_id == discord_id),
                    roster.iter().filter(|row| row.discord_id == discord_id),
                    &EventMembershipScope {
                        groups: &self.groups,
                        applicable_guilds: &applicable,
                        main_guild,
                    },
                );
                (discord_id, facts)
            })
            .collect())
    }
}
