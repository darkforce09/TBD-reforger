//! Effective policy selection for concrete ORBAT seats: slot, then squad, then event.

use api_identifiers::{DiscordUserId, EventMissionId, OrbatSlotId};
use std::cmp::Ordering;

use sqlx::PgConnection;

use super::context::EventAccessContext;
use super::evaluation::{EventAccessSubject, PolicySource, effective_policy, policy_admits};
use crate::models::event_access_policy::EventAccessPolicy;
use api_foundation::error_handling::api_error::ApiError;

/// The part of an ORBAT seat that selects its policy, its allocation order and its occupant.
#[derive(Debug, Clone, PartialEq, Eq, sqlx::FromRow)]
pub struct PolicySlot {
    /// The seat.
    pub id: OrbatSlotId,
    /// The event mission (attachment) the seat belongs to.
    pub event_mission_id: EventMissionId,
    /// The faction of the seat's squad, which keys the squad policy.
    pub faction: String,
    /// The seat's squad, which keys the squad policy.
    pub squad: String,
    /// The seat's position in its squad.
    pub slot_index: i64,
    /// The Discord account occupying the seat, or `None` when it is free.
    pub assigned_to: Option<DiscordUserId>,
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
    event_missions: &[EventMissionId],
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

    /// The policy that decides `slot` (slot, then squad, then event) and the level it came from.
    pub fn effective_slot_policy(&self, slot: &PolicySlot) -> (&EventAccessPolicy, PolicySource) {
        let (event, squad, seat) = self.slot_policy_chain(slot);
        effective_policy(event, squad, seat)
    }

    /// Policy alone; mandatory session, account, opening and capacity gates are checked apart.
    pub fn slot_admits(&self, slot: &PolicySlot, subject: &EventAccessSubject) -> bool {
        policy_admits(self.effective_slot_policy(slot).0, subject)
    }
}
