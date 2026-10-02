//! Effective policy selection for concrete ORBAT seats: slot, then squad, then event.

use std::cmp::Ordering;

use sqlx::PgConnection;
use uuid::Uuid;

use super::context::EventAccessContext;
use super::evaluation::{EventAccessSubject, PolicySource, effective_policy, policy_admits};
use crate::core::error_handling::api_error::ApiError;
use crate::operations::models::event_access_policy::EventAccessPolicy;

/// The part of an ORBAT seat that selects its policy, its allocation order and its occupant.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct PolicySlot {
    pub id: Uuid,
    pub event_mission_id: Uuid,
    pub faction: String,
    pub squad: String,
    pub slot_index: i64,
    pub assigned_to: Option<String>,
}

impl PolicySlot {
    /// Deterministic seat order: faction and squad by byte value, then slot index and identity.
    pub fn allocation_order(&self, other: &Self) -> Ordering {
        self.faction
            .as_bytes()
            .cmp(other.faction.as_bytes())
            .then_with(|| self.squad.as_bytes().cmp(other.squad.as_bytes()))
            .then_with(|| self.slot_index.cmp(&other.slot_index))
            .then_with(|| self.id.cmp(&other.id))
    }
}

/// Seats of the given attachments in allocation order. Callers mutating seats hold the
/// attachment locks, so this projection is stable for the rest of their transaction.
pub async fn load_policy_slots(
    connection: &mut PgConnection,
    event_missions: &[Uuid],
) -> Result<Vec<PolicySlot>, ApiError> {
    Ok(sqlx::query_as(
        "SELECT id, event_mission_id, faction, squad, slot_index, assigned_to FROM orbat_slots
         WHERE event_mission_id = ANY($1)
         ORDER BY faction COLLATE \"C\", squad COLLATE \"C\", slot_index, id",
    )
    .bind(event_missions)
    .fetch_all(connection)
    .await?)
}

impl EventAccessContext {
    /// The event policy and any explicit squad and slot policies that apply to `slot`.
    pub fn slot_policy_chain(
        &self,
        slot: &PolicySlot,
    ) -> (
        &EventAccessPolicy,
        Option<&EventAccessPolicy>,
        Option<&EventAccessPolicy>,
    ) {
        (
            &self.policy,
            self.squad_policies.get(&(
                slot.event_mission_id,
                slot.faction.clone(),
                slot.squad.clone(),
            )),
            self.slot_policies.get(&slot.id),
        )
    }

    pub fn effective_slot_policy(&self, slot: &PolicySlot) -> (&EventAccessPolicy, PolicySource) {
        let (event, squad, seat) = self.slot_policy_chain(slot);
        effective_policy(event, squad, seat)
    }

    /// Policy alone; mandatory session, account, opening and capacity gates are checked apart.
    pub fn slot_admits(&self, slot: &PolicySlot, subject: &EventAccessSubject) -> bool {
        policy_admits(self.effective_slot_policy(slot).0, subject)
    }

    /// A viewer may discover an event through its own policy or through any squad or slot policy.
    pub fn admits_any(&self, slots: &[PolicySlot], subject: &EventAccessSubject) -> bool {
        policy_admits(&self.policy, subject)
            || slots.iter().any(|slot| self.slot_admits(slot, subject))
    }
}
