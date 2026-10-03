//! Specific policies replace broader policies; mandatory constraints are checked independently.

use api_identifiers::{DiscordUserId, EventGroupId};
use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde::Serialize;

use crate::models::event_access_policy::{EventAccessCondition, EventAccessPolicy};
use crate::models::reservation_quota::ReservationQuotaKind;

/// Only membership snapshots accepted by the freshness/override rules populate these sets.
/// Partner guild roles grant event eligibility, never a website role.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventAccessSubject {
    /// Discord account the decision is about; empty means no valid session.
    pub discord_id: DiscordUserId,
    /// Whether the account holds an accepted membership snapshot of the TBD guild.
    pub tbd_member: bool,
    /// Role ids held per Discord guild id, from accepted membership snapshots only.
    pub guild_roles: BTreeMap<String, BTreeSet<String>>,
    /// Event groups of this event that list the account as a member.
    pub event_groups: BTreeSet<EventGroupId>,
}

/// Which policy level decided access: the event default, a squad override or a slot override.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicySource {
    /// The event-wide default policy.
    Event,
    /// A squad-level policy override (mission, faction, squad).
    Squad,
    /// A slot-level policy override, the most specific level.
    Slot,
}

/// Why access is refused; serialized as a snake_case variant name, the quota case as an object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessDenial {
    /// No valid session or no Discord account on the subject.
    InvalidSession,
    /// The account is banned or deleted.
    AccountUnavailable,
    /// The effective policy admits none of the subject's verified grants.
    Policy,
    /// Current authority lacks a grant that stale or pending membership evidence may still supply.
    MembershipVerificationRequired,
    /// Registration for the event is closed.
    RegistrationClosed,
    /// The subject's reservation pool has not opened yet.
    QuotaNotOpen {
        /// Reservation pool that applies to the subject.
        quota_kind: ReservationQuotaKind,
        /// Instant (UTC) at which that pool opens.
        opens_at: DateTime<Utc>,
    },
    /// The subject does not meet the deployment requirements of the seat.
    DeploymentRequirements,
    /// No free capacity remains.
    Capacity,
}

/// The participant's reservation pool, or its open-pool fallback, has not reached its opening time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaOpeningGate {
    /// The pool, or its open-pool fallback, already accepts reservations.
    Open,
    /// The pool opens later.
    NotYetOpen {
        /// Reservation pool that applies to the participant.
        quota_kind: ReservationQuotaKind,
        /// Instant (UTC) at which that pool opens.
        opens_at: DateTime<Utc>,
    },
}

/// Visibility checks use session/account constraints and policy alone. Full or future operations
/// remain discoverable; reservation and deployment writers additionally check their own gates.
#[derive(Debug, Clone, Copy)]
pub struct MandatoryAccessConstraints {
    /// The caller's session is valid.
    pub session_valid: bool,
    /// The account is available (neither banned nor deleted).
    pub account_available: bool,
    /// Registration for the event is open.
    pub registration_open: bool,
    /// Whether the participant's reservation pool has opened.
    pub quota_opening: QuotaOpeningGate,
    /// The participant meets the deployment requirements.
    pub deployment_eligible: bool,
    /// Free capacity remains for the requested seat.
    pub capacity_available: bool,
}

impl MandatoryAccessConstraints {
    /// Every mandatory gate passes; used when only the policy decision is being examined.
    pub const SATISFIED: Self = Self {
        session_valid: true,
        account_available: true,
        registration_open: true,
        quota_opening: QuotaOpeningGate::Open,
        deployment_eligible: true,
        capacity_available: true,
    };
}

/// Outcome of one access evaluation: the deciding policy level and the first failed gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct EventAccessDecision {
    /// Policy level whose policy was evaluated.
    pub policy_source: PolicySource,
    /// First gate that refused access; `None` means access is granted.
    pub denial: Option<AccessDenial>,
}

/// The most specific policy present (slot, then squad, then event) and the level it came from.
pub fn effective_policy<'a>(
    event: &'a EventAccessPolicy,
    squad: Option<&'a EventAccessPolicy>,
    slot: Option<&'a EventAccessPolicy>,
) -> (&'a EventAccessPolicy, PolicySource) {
    if let Some(policy) = slot {
        (policy, PolicySource::Slot)
    } else if let Some(policy) = squad {
        (policy, PolicySource::Squad)
    } else {
        (event, PolicySource::Event)
    }
}

fn condition_holds(condition: &EventAccessCondition, subject: &EventAccessSubject) -> bool {
    match condition {
        EventAccessCondition::Authenticated {} => true,
        EventAccessCondition::TbdMember {} => subject.tbd_member,
        EventAccessCondition::NamedAccount { discord_id } => {
            *discord_id == subject.discord_id.as_str()
        }
        EventAccessCondition::DiscordRole { guild_id, role_id } => subject
            .guild_roles
            .get(guild_id)
            .is_some_and(|roles| roles.contains(role_id)),
        EventAccessCondition::EventGroup { group_id } => subject.event_groups.contains(group_id),
    }
}

/// Alternative grants are OR; the conditions inside one grant are AND. The caller validates.
pub fn policy_admits(policy: &EventAccessPolicy, subject: &EventAccessSubject) -> bool {
    !admitting_grants(policy, subject).is_empty()
}

/// Positions of the grants whose every condition holds for `subject`; managers see which
/// alternative admitted a participant.
pub fn admitting_grants(policy: &EventAccessPolicy, subject: &EventAccessSubject) -> Vec<usize> {
    if subject.discord_id.is_empty() {
        return Vec::new();
    }
    policy
        .grants
        .iter()
        .enumerate()
        .filter(|(_, grant)| {
            grant
                .conditions
                .iter()
                .all(|condition| condition_holds(condition, subject))
        })
        .map(|(index, _)| index)
        .collect()
}

/// Evaluates the effective policy and every mandatory gate in a fixed order, returning the first
/// denial; `Err` carries the validation message of a corrupt stored policy.
pub fn evaluate_access(
    event: &EventAccessPolicy,
    squad: Option<&EventAccessPolicy>,
    slot: Option<&EventAccessPolicy>,
    subject: &EventAccessSubject,
    constraints: MandatoryAccessConstraints,
) -> Result<EventAccessDecision, &'static str> {
    let (policy, source) = effective_policy(event, squad, slot);
    // A corrupt policy cannot become an implicit open grant, even when no conditions are present.
    policy.validate()?;
    let denial = if !constraints.session_valid || subject.discord_id.is_empty() {
        Some(AccessDenial::InvalidSession)
    } else if !constraints.account_available {
        Some(AccessDenial::AccountUnavailable)
    } else if !policy_admits(policy, subject) {
        Some(AccessDenial::Policy)
    } else if !constraints.registration_open {
        Some(AccessDenial::RegistrationClosed)
    } else if let QuotaOpeningGate::NotYetOpen {
        quota_kind,
        opens_at,
    } = constraints.quota_opening
    {
        Some(AccessDenial::QuotaNotOpen {
            quota_kind,
            opens_at,
        })
    } else if !constraints.deployment_eligible {
        Some(AccessDenial::DeploymentRequirements)
    } else if !constraints.capacity_available {
        Some(AccessDenial::Capacity)
    } else {
        None
    };
    Ok(EventAccessDecision {
        policy_source: source,
        denial,
    })
}

/// A policy denial under current authority becomes a verification request when stale or pending
/// membership evidence could still satisfy the same policy. Such evidence never grants access.
pub fn evaluate_access_with_pending_evidence(
    event: &EventAccessPolicy,
    squad: Option<&EventAccessPolicy>,
    slot: Option<&EventAccessPolicy>,
    current: &EventAccessSubject,
    pending_evidence_admits: bool,
    constraints: MandatoryAccessConstraints,
) -> Result<EventAccessDecision, &'static str> {
    let mut decision = evaluate_access(event, squad, slot, current, constraints)?;
    if decision.denial == Some(AccessDenial::Policy) && pending_evidence_admits {
        decision.denial = Some(AccessDenial::MembershipVerificationRequired);
    }
    Ok(decision)
}

#[cfg(test)]
#[path = "tests/evaluation.rs"]
mod tests;
