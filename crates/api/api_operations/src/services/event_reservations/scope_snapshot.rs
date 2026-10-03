//! One projection of a locked event scope: seats, reservations, waiting candidates, allocations,
//! quota settings, policies and the membership facts of every locked account.

use api_identifiers::{
    DiscordGuildId, DiscordUserId, EventMissionId, EventRegistrationId, OrbatSlotId,
};
use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use sqlx::PgConnection;

use super::participant_allocations::{load_active_allocations, load_quota_settings};
use super::reservation_planning::{
    ActiveReservation, EligibilityOracle, PlanningSeat, QueuedRegistration, QuotaSettings,
    ReservationPlan,
};
use super::reservation_scope::ReservationScope;
use crate::models::RegistrationState;
use crate::models::participant_allocation::ParticipantAllocationKind;
use crate::services::event_access::context::EventAccessContext;
use crate::services::event_access::slot_eligibility::{PolicySlot, load_policy_slots};
use crate::services::event_access::subject_loading::AccountEligibilityFacts;
use api_foundation::error_handling::api_error::ApiError;

/// Eligibility of locked accounts for the scope's seats. Unknown accounts and seats never pass.
pub struct ScopeEligibility {
    /// The event's access policies, groups and guild settings.
    pub context: EventAccessContext,
    /// The current and last verified membership facts of every locked account.
    pub facts: BTreeMap<DiscordUserId, AccountEligibilityFacts>,
    /// The scope's seats with the part that selects their policy, keyed by seat.
    pub slots: BTreeMap<OrbatSlotId, PolicySlot>,
    /// The community guild whose membership evidence the pending checks read.
    pub main_guild: DiscordGuildId,
}

impl ScopeEligibility {
    fn admits(&self, account: &str, seat: OrbatSlotId, current: bool) -> bool {
        match (self.facts.get(account), self.slots.get(&seat)) {
            (Some(facts), Some(slot)) => self.context.slot_admits(
                slot,
                if current {
                    &facts.current
                } else {
                    &facts.last_verified
                },
            ),
            _ => false,
        }
    }
}

impl EligibilityOracle for ScopeEligibility {
    fn available(&self, account: &str) -> bool {
        self.facts.get(account).is_some_and(|facts| facts.available)
    }

    fn current_member(&self, account: &str) -> bool {
        self.facts
            .get(account)
            .is_some_and(|facts| facts.current.tbd_member)
    }

    fn current_admits(&self, account: &str, seat: OrbatSlotId) -> bool {
        self.admits(account, seat, true)
    }

    fn verified_admits(&self, account: &str, seat: OrbatSlotId) -> bool {
        self.admits(account, seat, false)
    }
}

#[derive(sqlx::FromRow)]
struct RegistrationRow {
    id: EventRegistrationId,
    discord_id: String,
    event_mission_id: EventMissionId,
    queue_entered_at: DateTime<Utc>,
    slot_id: Option<OrbatSlotId>,
    reservation_state: RegistrationState,
}

/// Everything the reservation planner reads about one locked scope, loaded once per transaction.
pub struct ScopeSnapshot {
    /// The eligibility oracle over the locked accounts and seats.
    pub eligibility: ScopeEligibility,
    /// The seats of the operational attachments, in allocation order, with their occupants.
    pub seats: Vec<PlanningSeat>,
    /// The registered reservations, with the seat each holds when the seat records it.
    pub reservations: Vec<ActiveReservation>,
    /// The waiting participants.
    pub waiting: Vec<QueuedRegistration>,
    /// The unreleased allocation of every participant, keyed by Discord account.
    pub allocations: BTreeMap<String, ParticipantAllocationKind>,
    /// The event's participant limit and reservation pools.
    pub settings: QuotaSettings,
}

impl ScopeSnapshot {
    /// Read after the scope's locks, so no other writer can change these rows meanwhile.
    pub async fn load(
        connection: &mut PgConnection,
        scope: &ReservationScope,
        main_guild: &DiscordGuildId,
    ) -> Result<Self, ApiError> {
        let event_id = scope.event.id;
        let context = EventAccessContext::load(connection, event_id).await?;
        let facts = context
            .account_facts(connection, &scope.accounts, main_guild)
            .await?;
        let policy_slots = load_policy_slots(connection, &scope.active_missions).await?;
        let rows: Vec<RegistrationRow> = sqlx::query_as(
            "SELECT id, discord_id, event_mission_id, queue_entered_at, slot_id, reservation_state
             FROM event_registrations WHERE event_mission_id = ANY($1)
                AND reservation_state IN ('registered', 'legacy_unknown', 'waitlisted')",
        )
        .bind(&scope.active_missions)
        .fetch_all(&mut *connection)
        .await?;
        let seats: Vec<PlanningSeat> = policy_slots
            .iter()
            .map(|slot| PlanningSeat {
                id: slot.id,
                mission: slot.event_mission_id,
                faction: slot.faction.clone(),
                squad: slot.squad.clone(),
                slot_index: slot.slot_index,
                occupant: slot.assigned_to.clone().map(String::from),
            })
            .collect();
        let occupant_of: BTreeMap<OrbatSlotId, Option<&DiscordUserId>> = policy_slots
            .iter()
            .map(|slot| (slot.id, slot.assigned_to.as_ref()))
            .collect();
        let mut reservations = Vec::new();
        let mut waiting = Vec::new();
        for row in rows {
            let queued = QueuedRegistration {
                registration: row.id,
                account: row.discord_id,
                mission: row.event_mission_id,
                queue_entered_at: row.queue_entered_at,
            };
            if row.reservation_state == RegistrationState::Waitlisted {
                waiting.push(queued);
                continue;
            }
            // A claim the seat itself does not record is treated as a seatless place hold.
            let seat = row.slot_id.filter(|slot| {
                occupant_of.get(slot).is_some_and(|occupant| {
                    occupant.is_some_and(|occupant| occupant == queued.account.as_str())
                })
            });
            reservations.push(ActiveReservation { queued, seat });
        }
        let allocations = load_active_allocations(connection, event_id).await?;
        let settings = load_quota_settings(connection, event_id, scope.event.max_slots).await?;
        Ok(Self {
            eligibility: ScopeEligibility {
                context,
                facts,
                slots: policy_slots
                    .into_iter()
                    .map(|slot| (slot.id, slot))
                    .collect(),
                main_guild: main_guild.clone(),
            },
            seats,
            reservations,
            waiting,
            allocations,
            settings,
        })
    }

    /// The reservation plan over this snapshot; a broken planner invariant is an internal error.
    pub fn plan(&self) -> Result<ReservationPlan, ApiError> {
        ReservationPlan::new(
            self.seats.clone(),
            &self.reservations,
            self.allocations.clone(),
            self.settings.clone(),
        )
        .map_err(ApiError::internal)
    }

    /// The membership facts of the locked `account`, or `None` when it was not locked.
    pub fn facts(&self, account: &str) -> Option<&AccountEligibilityFacts> {
        self.eligibility.facts.get(account)
    }

    /// The policy-selecting part of `seat`, or `None` when it is outside the scope.
    pub fn slot(&self, seat: OrbatSlotId) -> Option<&PolicySlot> {
        self.eligibility.slots.get(&seat)
    }

    /// The account's active reservation in `mission`, if any.
    pub fn reservation(
        &self,
        mission: EventMissionId,
        account: &str,
    ) -> Option<&ActiveReservation> {
        self.reservations
            .iter()
            .find(|r| r.queued.mission == mission && r.queued.account == account)
    }

    /// Seats in `mission`, in allocation order.
    pub fn mission_slots(&self, mission: EventMissionId) -> impl Iterator<Item = &PolicySlot> {
        self.seats
            .iter()
            .filter(move |seat| seat.mission == mission)
            .filter_map(|seat| self.eligibility.slots.get(&seat.id))
    }
}
