//! Where a squad's or a slot's access policy comes from: its own, its squad's, or the operation's.
//!
//! **Role:** resolves the policy that decides a squad or a slot from the access view's explicit
//! policies, and words that resolution for the policy lists.
//! **Position:** read by the policy lists of the access panel for every squad and slot row.
//! **Signals & state:** none; pure over the access view.
//! **Invariants:** a slot's own policy wins, then its squad's, then the operation's — the order the
//! backend evaluates in. An own policy with no grants admits nobody, and is worded as such: it is a
//! decision, where the absence of an own policy is inheritance.

use super::policy_draft::policy_summary;
use crate::v2::core::api::dto::{EventAccessAdministration, EventAccessPolicy};

/// Where the policy deciding a squad or a slot comes from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum PolicyOrigin<'a> {
    /// The squad or slot has a policy of its own.
    Own(&'a EventAccessPolicy),
    /// A slot without one follows its squad's.
    Squad(&'a EventAccessPolicy),
    /// A squad or slot without one, whose squad has none either, follows the operation's.
    Operation(&'a EventAccessPolicy),
}

impl<'a> PolicyOrigin<'a> {
    /// The policy that decides.
    pub(super) fn policy(self) -> &'a EventAccessPolicy {
        match self {
            PolicyOrigin::Own(p) | PolicyOrigin::Squad(p) | PolicyOrigin::Operation(p) => p,
        }
    }

    /// Whether the squad or slot has a policy of its own, which it can be told to drop.
    pub(super) fn is_own(self) -> bool {
        matches!(self, PolicyOrigin::Own(_))
    }

    /// The resolution in words, with the deciding policy summarised.
    pub(super) fn describe(self, access: &EventAccessAdministration) -> String {
        let summary = policy_summary(self.policy(), &access.groups);
        match self {
            PolicyOrigin::Own(policy) if policy.grants.is_empty() => {
                "Own policy with no grants: admits nobody".to_string()
            }
            PolicyOrigin::Own(_) => format!("Own policy. {summary}"),
            PolicyOrigin::Squad(_) => format!("Follows its squad's policy. {summary}"),
            PolicyOrigin::Operation(_) => format!("Follows the operation's policy. {summary}"),
        }
    }
}

/// A squad's explicit policy, when it has one.
pub(super) fn squad_policy<'a>(
    access: &'a EventAccessAdministration,
    event_mission_id: &str,
    faction: &str,
    squad: &str,
) -> Option<&'a EventAccessPolicy> {
    access
        .squad_policies
        .iter()
        .find(|p| {
            p.event_mission_id == event_mission_id && p.faction == faction && p.squad == squad
        })
        .map(|p| &p.policy)
}

/// Where a squad's policy comes from.
pub(super) fn squad_origin<'a>(
    access: &'a EventAccessAdministration,
    event_mission_id: &str,
    faction: &str,
    squad: &str,
) -> PolicyOrigin<'a> {
    match squad_policy(access, event_mission_id, faction, squad) {
        Some(own) => PolicyOrigin::Own(own),
        None => PolicyOrigin::Operation(&access.event_policy),
    }
}

/// Where a slot's policy comes from.
pub(super) fn slot_origin<'a>(
    access: &'a EventAccessAdministration,
    event_mission_id: &str,
    faction: &str,
    squad: &str,
    slot_id: &str,
) -> PolicyOrigin<'a> {
    if let Some(own) = access
        .slot_policies
        .iter()
        .find(|p| p.slot_id == slot_id)
        .map(|p| &p.policy)
    {
        return PolicyOrigin::Own(own);
    }
    match squad_policy(access, event_mission_id, faction, squad) {
        Some(squad_policy) => PolicyOrigin::Squad(squad_policy),
        None => PolicyOrigin::Operation(&access.event_policy),
    }
}
