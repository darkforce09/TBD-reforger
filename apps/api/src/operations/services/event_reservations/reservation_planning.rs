//! Pure planning for seat claims, waitlist promotion and eligibility re-evaluation.
//!
//! The transaction layer loads one locked event scope into a [`ReservationPlan`], asks for a
//! plan, applies it, and records history and audit. Planning never touches the database, so the
//! same rules are exercised by property tests and by every production transaction.
//!
//! Existing seatless holders are judged by last verified membership facts, so stale data never
//! takes their place away. New claims and promotions require current authority.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use uuid::Uuid;

use super::quota_selection::{QuotaDecision, ReservationQuotaUsage};
use super::seat_matching::SeatEligibility;
use crate::operations::models::participant_allocation::ParticipantAllocationKind;
use crate::operations::models::reservation_quota::{ReservationQuotaKind, ReservationQuotas};

/// Eligibility answers for accounts inside the locked scope.
pub trait EligibilityOracle {
    /// Not banned and not deleted.
    fn available(&self, account: &str) -> bool;
    /// Verified TBD membership under current authority selects the member pool.
    fn current_member(&self, account: &str) -> bool;
    /// Current authority admits the account to the seat (new claims and promotions).
    fn current_admits(&self, account: &str, seat: Uuid) -> bool;
    /// Last verified facts admit the account to the seat (existing reservations).
    fn verified_admits(&self, account: &str, seat: Uuid) -> bool;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanningSeat {
    pub id: Uuid,
    pub mission: Uuid,
    pub faction: String,
    pub squad: String,
    pub slot_index: i64,
    pub occupant: Option<String>,
}

impl PlanningSeat {
    fn allocation_order(&self, other: &Self) -> Ordering {
        self.faction
            .as_bytes()
            .cmp(other.faction.as_bytes())
            .then_with(|| self.squad.as_bytes().cmp(other.squad.as_bytes()))
            .then_with(|| self.slot_index.cmp(&other.slot_index))
            .then_with(|| self.id.cmp(&other.id))
    }
}

/// A registration in the queue order `(queue_entered_at, registration)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueuedRegistration {
    pub registration: Uuid,
    pub account: String,
    pub mission: Uuid,
    pub queue_entered_at: DateTime<Utc>,
}

impl QueuedRegistration {
    fn queue_order(&self, other: &Self) -> Ordering {
        self.queue_entered_at
            .cmp(&other.queue_entered_at)
            .then_with(|| self.registration.cmp(&other.registration))
    }
}

/// An active reservation: seated when `seat` is present, otherwise a seatless place hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveReservation {
    pub queued: QueuedRegistration,
    pub seat: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedPromotion {
    pub registration: Uuid,
    pub account: String,
    pub mission: Uuid,
    pub seat: Uuid,
    /// The pool a new allocation consumes; `None` reuses the participant's event allocation.
    pub new_allocation: Option<ReservationQuotaKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseCause {
    AccountUnavailable,
    PolicyDenied,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedRelease {
    pub registration: Uuid,
    pub account: String,
    pub mission: Uuid,
    pub seat: Option<Uuid>,
    pub cause: ReleaseCause,
    /// The participant has no remaining reservation or seat in the event.
    pub releases_allocation: bool,
}

/// Quota configuration and the current instant for one event.
#[derive(Debug, Clone)]
pub struct QuotaSettings {
    pub quotas: ReservationQuotas,
    pub max_slots: u32,
    pub now: DateTime<Utc>,
}

/// One locked event scope: seats, seatless holders, per-mission participants and quota state.
#[derive(Debug, Clone)]
pub struct ReservationPlan {
    seats: Vec<PlanningSeat>,
    holders: Vec<QueuedRegistration>,
    /// Distinct active registrants and seat occupants per attachment.
    participants: BTreeMap<Uuid, BTreeSet<String>>,
    /// Active event allocations by account.
    allocations: BTreeMap<String, ParticipantAllocationKind>,
    /// Active reservations plus seats occupied without a reservation, per account.
    presence: BTreeMap<String, usize>,
    usage: ReservationQuotaUsage,
    settings: QuotaSettings,
}

impl ReservationPlan {
    /// Inputs may arrive in any order; planning uses allocation and queue order. Usage is
    /// derived from `allocations`, so the plan's quota state cannot disagree with its holders.
    pub fn new(
        mut seats: Vec<PlanningSeat>,
        reservations: &[ActiveReservation],
        mut allocations: BTreeMap<String, ParticipantAllocationKind>,
        settings: QuotaSettings,
    ) -> Result<Self, &'static str> {
        seats.sort_by(PlanningSeat::allocation_order);
        // A seat occupied without a recorded allocation still holds an event place.
        for occupant in seats.iter().filter_map(|seat| seat.occupant.as_ref()) {
            allocations
                .entry(occupant.clone())
                .or_insert(ParticipantAllocationKind::LegacyUnclassified);
        }
        let reserved_seats: BTreeSet<Uuid> = reservations.iter().filter_map(|r| r.seat).collect();
        let mut participants: BTreeMap<Uuid, BTreeSet<String>> = BTreeMap::new();
        let mut presence: BTreeMap<String, usize> = BTreeMap::new();
        for seat in &seats {
            participants.entry(seat.mission).or_default();
            if let Some(occupant) = &seat.occupant {
                participants
                    .entry(seat.mission)
                    .or_default()
                    .insert(occupant.clone());
                if !reserved_seats.contains(&seat.id) {
                    *presence.entry(occupant.clone()).or_default() += 1;
                }
            }
        }
        let mut holders = Vec::new();
        for reservation in reservations {
            participants
                .entry(reservation.queued.mission)
                .or_default()
                .insert(reservation.queued.account.clone());
            *presence
                .entry(reservation.queued.account.clone())
                .or_default() += 1;
            if reservation.seat.is_none() {
                holders.push(reservation.queued.clone());
            }
        }
        holders.sort_by(QueuedRegistration::queue_order);
        let mut usage = ReservationQuotaUsage::default();
        for kind in allocations.values() {
            let count = match kind {
                ParticipantAllocationKind::Member => &mut usage.member,
                ParticipantAllocationKind::Guest => &mut usage.guest,
                ParticipantAllocationKind::Open => &mut usage.open,
                ParticipantAllocationKind::LegacyUnclassified => &mut usage.legacy_unclassified,
            };
            *count = count.checked_add(1).ok_or("quota usage overflow")?;
        }
        Ok(Self {
            seats,
            holders,
            participants,
            allocations,
            presence,
            usage,
            settings,
        })
    }

    pub fn usage(&self) -> ReservationQuotaUsage {
        self.usage
    }

    pub fn seats(&self) -> &[PlanningSeat] {
        &self.seats
    }

    fn seat_count(&self, mission: Uuid) -> usize {
        self.seats
            .iter()
            .filter(|seat| seat.mission == mission)
            .count()
    }

    fn free_seats(&self, mission: Uuid, taken: Option<Uuid>, vacated: Option<Uuid>) -> Vec<Uuid> {
        self.seats
            .iter()
            .filter(|seat| seat.mission == mission)
            .filter(|seat| Some(seat.id) != taken)
            .filter(|seat| seat.occupant.is_none() || Some(seat.id) == vacated)
            .map(|seat| seat.id)
            .collect()
    }

    /// The mission's seatless holders remain seatable after a change. `leaving` names a holder
    /// that stops being seatless, `joining` a new holder judged by current authority, `taken` a
    /// free seat that becomes occupied and `vacated` an occupied seat that becomes free.
    pub fn holders_remain_seatable(
        &self,
        oracle: &impl EligibilityOracle,
        mission: Uuid,
        leaving: Option<Uuid>,
        joining: Option<&str>,
        taken: Option<Uuid>,
        vacated: Option<Uuid>,
    ) -> bool {
        let free = self.free_seats(mission, taken, vacated);
        let mut matrix = SeatEligibility::new(free.len());
        for holder in self
            .holders
            .iter()
            .filter(|holder| holder.mission == mission && Some(holder.registration) != leaving)
        {
            matrix.push_holder(|index| oracle.verified_admits(&holder.account, free[index]));
        }
        if let Some(account) = joining {
            matrix.push_holder(|index| oracle.current_admits(account, free[index]));
        }
        matrix.all_holders_seatable()
    }

    /// A new participant may enter the attachment without exceeding its physical seats.
    pub fn mission_has_room(&self, mission: Uuid, account: &str) -> bool {
        let participants = self.participants.get(&mission);
        participants.is_some_and(|set| set.contains(account))
            || participants.map_or(0, BTreeSet::len) < self.seat_count(mission)
    }

    pub fn holds_allocation(&self, account: &str) -> bool {
        self.allocations.contains_key(account)
    }

    /// Quota for one new participant, evaluated without changing the plan.
    pub fn quota_decision(&self, member: bool) -> Result<QuotaDecision, &'static str> {
        let mut trial = self.usage;
        trial.decide(
            &self.settings.quotas,
            member,
            self.settings.now,
            self.settings.max_slots,
        )
    }

    /// Promote waiting participants in queue order. Each promotion takes the first free seat,
    /// in allocation order, that current authority admits and that keeps holders seatable.
    pub fn plan_promotions(
        &mut self,
        oracle: &impl EligibilityOracle,
        waiting: &[QueuedRegistration],
    ) -> Result<Vec<PlannedPromotion>, &'static str> {
        let mut queue = waiting.to_vec();
        queue.sort_by(QueuedRegistration::queue_order);
        let mut promotions = Vec::new();
        for candidate in queue {
            if !oracle.available(&candidate.account)
                || !self.mission_has_room(candidate.mission, &candidate.account)
            {
                continue;
            }
            let new_allocation = if self.holds_allocation(&candidate.account) {
                None
            } else {
                match self.quota_decision(oracle.current_member(&candidate.account))? {
                    QuotaDecision::Reserved(kind) => Some(kind),
                    QuotaDecision::NotYetOpen { .. } | QuotaDecision::Exhausted => continue,
                }
            };
            let Some(seat) = self
                .free_seats(candidate.mission, None, None)
                .into_iter()
                .find(|seat| {
                    oracle.current_admits(&candidate.account, *seat)
                        && self.holders_remain_seatable(
                            oracle,
                            candidate.mission,
                            None,
                            None,
                            Some(*seat),
                            None,
                        )
                })
            else {
                continue;
            };
            if let Some(kind) = new_allocation {
                let decision = self.usage.decide(
                    &self.settings.quotas,
                    oracle.current_member(&candidate.account),
                    self.settings.now,
                    self.settings.max_slots,
                )?;
                if decision != QuotaDecision::Reserved(kind) {
                    return Err("quota decision changed within one plan");
                }
                self.allocations
                    .insert(candidate.account.clone(), kind.into());
            }
            *self.presence.entry(candidate.account.clone()).or_default() += 1;
            self.occupy(seat, &candidate.account);
            self.participants
                .entry(candidate.mission)
                .or_default()
                .insert(candidate.account.clone());
            promotions.push(PlannedPromotion {
                registration: candidate.registration,
                account: candidate.account,
                mission: candidate.mission,
                seat,
                new_allocation,
            });
        }
        Ok(promotions)
    }

    /// Release reservations whose eligibility loss is confirmed by last verified facts: bans and
    /// deletions, seats their policy no longer admits, holds no seat in the attachment admits,
    /// and then the latest holders a matching of earlier holders cannot seat.
    pub fn plan_eligibility_releases(
        &mut self,
        oracle: &impl EligibilityOracle,
        reservations: &[ActiveReservation],
    ) -> Result<Vec<PlannedRelease>, &'static str> {
        let mut ordered = reservations.to_vec();
        ordered.sort_by(|left, right| left.queued.queue_order(&right.queued));
        let mut releases = Vec::new();
        for reservation in &ordered {
            let queued = &reservation.queued;
            let cause = if !oracle.available(&queued.account) {
                Some(ReleaseCause::AccountUnavailable)
            } else {
                let admitted = match reservation.seat {
                    Some(seat) => oracle.verified_admits(&queued.account, seat),
                    None => self
                        .seats
                        .iter()
                        .filter(|seat| seat.mission == queued.mission)
                        .any(|seat| oracle.verified_admits(&queued.account, seat.id)),
                };
                (!admitted).then_some(ReleaseCause::PolicyDenied)
            };
            if let Some(cause) = cause {
                releases.push(self.release(reservation, cause)?);
            }
        }
        let missions: BTreeSet<Uuid> = self.holders.iter().map(|holder| holder.mission).collect();
        for mission in missions {
            let free = self.free_seats(mission, None, None);
            let holders: Vec<QueuedRegistration> = self
                .holders
                .iter()
                .filter(|holder| holder.mission == mission)
                .cloned()
                .collect();
            let mut matrix = SeatEligibility::new(free.len());
            for holder in &holders {
                matrix.push_holder(|index| oracle.verified_admits(&holder.account, free[index]));
            }
            for index in matrix.unseatable_holders() {
                let reservation = ActiveReservation {
                    queued: holders[index].clone(),
                    seat: None,
                };
                releases.push(self.release(&reservation, ReleaseCause::PolicyDenied)?);
            }
        }
        Ok(releases)
    }

    fn occupy(&mut self, seat: Uuid, account: &str) {
        if let Some(target) = self.seats.iter_mut().find(|candidate| candidate.id == seat) {
            target.occupant = Some(account.to_owned());
        }
    }

    fn release(
        &mut self,
        reservation: &ActiveReservation,
        cause: ReleaseCause,
    ) -> Result<PlannedRelease, &'static str> {
        let queued = &reservation.queued;
        if let Some(seat) = reservation.seat
            && let Some(target) = self.seats.iter_mut().find(|candidate| candidate.id == seat)
        {
            target.occupant = None;
        }
        self.holders
            .retain(|holder| holder.registration != queued.registration);
        if let Some(set) = self.participants.get_mut(&queued.mission) {
            set.remove(&queued.account);
        }
        let remaining = self
            .presence
            .get_mut(&queued.account)
            .ok_or("released reservation had no recorded presence")?;
        *remaining = remaining
            .checked_sub(1)
            .ok_or("released reservation had no recorded presence")?;
        let releases_allocation = *remaining == 0;
        if releases_allocation {
            self.presence.remove(&queued.account);
            match self.allocations.remove(&queued.account) {
                Some(ParticipantAllocationKind::LegacyUnclassified) => {
                    self.usage.release_legacy_unclassified()?
                }
                Some(kind) => self
                    .usage
                    .release(kind.pool().ok_or("allocation kind has no pool")?)?,
                None => return Err("a released participant had no active allocation"),
            }
        }
        Ok(PlannedRelease {
            registration: queued.registration,
            account: queued.account.clone(),
            mission: queued.mission,
            seat: reservation.seat,
            cause,
            releases_allocation,
        })
    }
}

#[cfg(test)]
#[path = "tests/reservation_planning.rs"]
mod tests;
