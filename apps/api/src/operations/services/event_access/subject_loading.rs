//! Membership facts for many accounts at once, under both evidence standards eligibility uses.
//!
//! Current authority admits only snapshots that are fresh, inside the 48-hour grace period, or
//! extended by an audited override. Last verified facts keep the most recent verified member
//! observation regardless of age: they decide whether eligibility loss is confirmed, so stale
//! data never evicts a reservation. Unverified guilds never grant access under either standard.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use uuid::Uuid;

use super::context::EventAccessContext;
use super::evaluation::{EventAccessSubject, policy_admits};
use crate::core::error_handling::api_error::ApiError;
use crate::identity_and_access::models::user_account::UserRole;
use crate::identity_and_access::services::cached_membership_permissions::evaluate_cached_membership_permissions;
use crate::operations::models::event_access_policy::{EventAccessCondition, EventAccessPolicy};
use crate::operations::models::event_group::{EventGroup, EventGroupSource};

/// Which membership observations may supply grants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MembershipEvidence {
    CurrentAuthority,
    LastVerifiedFacts,
}

/// One guild observation as recorded by bot-authenticated REST reconciliation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildEvidence {
    pub membership_status: String,
    pub verified_at: Option<DateTime<Utc>>,
    pub override_until: Option<DateTime<Utc>>,
    /// The observation currently authorizes grants (fresh, within grace, or overridden).
    pub current: bool,
}

/// A manager-maintained roster entry, authored by a manager or a named system transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RosterEvidence {
    pub added_by: Option<String>,
    pub system_origin: Option<String>,
    pub added_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AccountEligibilityFacts {
    pub discord_id: String,
    /// Not banned and not deleted, read after the caller's account lock.
    pub available: bool,
    pub current: EventAccessSubject,
    pub last_verified: EventAccessSubject,
    /// Applicable guilds with no verified observation yet; pending checks may still grant them.
    pub unverified_guilds: BTreeSet<String>,
    pub guilds: BTreeMap<String, GuildEvidence>,
    pub roster_groups: BTreeMap<Uuid, RosterEvidence>,
    /// Partner groups whose guild has not been verified for this account yet.
    pub pending_partner_groups: BTreeSet<Uuid>,
    pub observed_at: DateTime<Utc>,
}

impl AccountEligibilityFacts {
    pub fn subject(&self, evidence: MembershipEvidence) -> &EventAccessSubject {
        match evidence {
            MembershipEvidence::CurrentAuthority => &self.current,
            MembershipEvidence::LastVerifiedFacts => &self.last_verified,
        }
    }

    /// Stale or still-pending observations could satisfy `policy`. This is used to ask for
    /// verification instead of reporting a plain denial; it never grants access by itself.
    pub fn pending_evidence_admits(&self, policy: &EventAccessPolicy, main_guild: &str) -> bool {
        let mut optimistic = self.last_verified.clone();
        if self.unverified_guilds.contains(main_guild) {
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
    pub discord_id: String,
    pub guild_id: String,
    pub membership_status: String,
    pub verified_at: Option<DateTime<Utc>>,
    pub override_until: Option<DateTime<Utc>>,
    pub roles: Vec<String>,
}

/// An active managed-roster entry and its provenance.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RosterEntryRow {
    pub event_id: Uuid,
    pub discord_id: String,
    pub group_id: Uuid,
    pub added_by: Option<String>,
    pub system_origin: Option<String>,
    pub added_at: DateTime<Utc>,
}

/// Snapshots of `accounts` for `guilds`, with roles in byte order.
pub async fn load_membership_snapshots(
    connection: &mut PgConnection,
    accounts: &[String],
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
    events: &[Uuid],
    accounts: &[String],
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
    pub groups: &'a [EventGroup],
    pub applicable_guilds: &'a BTreeSet<String>,
    pub main_guild: &'a str,
}

/// Pure construction of one account's facts for one event's groups and applicable guilds.
pub fn build_account_facts<'a>(
    discord_id: &str,
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
    for snapshot in snapshots.filter(|row| applicable_guilds.contains(&row.guild_id)) {
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
                .insert(snapshot.guild_id.clone(), roles.clone());
        }
        if member && snapshot.verified_at.is_some() {
            last_verified
                .guild_roles
                .insert(snapshot.guild_id.clone(), roles);
        }
        account_guilds.insert(
            snapshot.guild_id.clone(),
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
    let roster_groups: BTreeMap<Uuid, RosterEvidence> = roster
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
    current.tbd_member = current.guild_roles.contains_key(main_guild);
    last_verified.tbd_member = last_verified.guild_roles.contains_key(main_guild);
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
        accounts: &[String],
        main_guild: &str,
    ) -> Result<BTreeMap<String, AccountEligibilityFacts>, ApiError> {
        let rows: Vec<(String, bool, DateTime<Utc>)> = sqlx::query_as(
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
