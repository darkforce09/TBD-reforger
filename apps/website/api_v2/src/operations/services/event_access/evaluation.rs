//! Specific policies replace broader policies; mandatory constraints are checked independently.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::operations::models::event_access_policy::{EventAccessCondition, EventAccessPolicy};
use crate::operations::models::reservation_quota::ReservationQuotaKind;

/// Only membership snapshots accepted by the freshness/override rules populate these sets.
/// Partner guild roles grant event eligibility, never a website role.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EventAccessSubject {
    pub discord_id: String,
    pub tbd_member: bool,
    pub guild_roles: BTreeMap<String, BTreeSet<String>>,
    pub event_groups: BTreeSet<Uuid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicySource {
    Event,
    Squad,
    Slot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessDenial {
    InvalidSession,
    AccountUnavailable,
    Policy,
    /// Current authority lacks a grant that stale or pending membership evidence may still supply.
    MembershipVerificationRequired,
    RegistrationClosed,
    QuotaNotOpen {
        quota_kind: ReservationQuotaKind,
        opens_at: DateTime<Utc>,
    },
    DeploymentRequirements,
    Capacity,
}

/// The participant's reservation pool, or its open-pool fallback, has not reached its opening time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaOpeningGate {
    Open,
    NotYetOpen {
        quota_kind: ReservationQuotaKind,
        opens_at: DateTime<Utc>,
    },
}

/// Visibility checks use session/account constraints and policy alone. Full or future operations
/// remain discoverable; reservation and deployment writers additionally check their own gates.
#[derive(Debug, Clone, Copy)]
pub struct MandatoryAccessConstraints {
    pub session_valid: bool,
    pub account_available: bool,
    pub registration_open: bool,
    pub quota_opening: QuotaOpeningGate,
    pub deployment_eligible: bool,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct EventAccessDecision {
    pub policy_source: PolicySource,
    pub denial: Option<AccessDenial>,
}

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
        EventAccessCondition::NamedAccount { discord_id } => *discord_id == subject.discord_id,
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
