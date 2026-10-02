//! Checked quota accounting. One event participant owns one allocation shared by their missions.

use chrono::{DateTime, Utc};

use crate::operations::models::reservation_quota::{ReservationQuotaKind, ReservationQuotas};

/// Active allocations by recorded quota kind. Allocations recorded before pools existed are
/// legacy_unclassified: they count toward the event-wide total and toward no pool.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReservationQuotaUsage {
    pub member: u64,
    pub guest: u64,
    pub open: u64,
    pub legacy_unclassified: u64,
}

/// The outcome of asking for one new participant place, with the reason when none is granted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaDecision {
    Reserved(ReservationQuotaKind),
    /// A pool with remaining places exists but has not reached its opening time yet.
    NotYetOpen {
        quota_kind: ReservationQuotaKind,
        opens_at: DateTime<Utc>,
    },
    /// The event-wide limit or every applicable pool is full or closed.
    Exhausted,
}

impl ReservationQuotaUsage {
    pub fn count(self, kind: ReservationQuotaKind) -> u64 {
        match kind {
            ReservationQuotaKind::Member => self.member,
            ReservationQuotaKind::Guest => self.guest,
            ReservationQuotaKind::Open => self.open,
        }
    }

    pub fn total(self) -> Result<u64, &'static str> {
        self.member
            .checked_add(self.guest)
            .and_then(|n| n.checked_add(self.open))
            .and_then(|n| n.checked_add(self.legacy_unclassified))
            .ok_or("quota usage overflow")
    }

    /// Existing allocations are retained by identity, never passed here as new participants.
    /// A pool opening is inclusive. The event's historical max_slots=0 means no event-wide cap.
    pub fn reserve(
        &mut self,
        quotas: &ReservationQuotas,
        tbd_member: bool,
        now: DateTime<Utc>,
        event_max_slots: u32,
    ) -> Result<Option<ReservationQuotaKind>, &'static str> {
        Ok(
            match self.decide(quotas, tbd_member, now, event_max_slots)? {
                QuotaDecision::Reserved(kind) => Some(kind),
                QuotaDecision::NotYetOpen { .. } | QuotaDecision::Exhausted => None,
            },
        )
    }

    /// Reserve like [`Self::reserve`], explaining a refusal. When pools with remaining places
    /// have not opened, the earliest opening is reported; the participant's own pool wins ties.
    pub fn decide(
        &mut self,
        quotas: &ReservationQuotas,
        tbd_member: bool,
        now: DateTime<Utc>,
        event_max_slots: u32,
    ) -> Result<QuotaDecision, &'static str> {
        let total = self.total()?;
        if event_max_slots != 0 && total >= u64::from(event_max_slots) {
            return Ok(QuotaDecision::Exhausted);
        }
        let preferred = if tbd_member {
            ReservationQuotaKind::Member
        } else {
            ReservationQuotaKind::Guest
        };
        let mut earliest_closed: Option<(ReservationQuotaKind, DateTime<Utc>)> = None;
        for kind in [preferred, ReservationQuotaKind::Open] {
            let pool = quotas.pool(kind);
            let used = self.count(kind);
            if pool.seats.is_some_and(|limit| used >= u64::from(limit)) {
                continue;
            }
            if now >= pool.opens_at {
                // Check both the pool and total before changing either observable count.
                total.checked_add(1).ok_or("quota usage overflow")?;
                let new_count = used.checked_add(1).ok_or("quota usage overflow")?;
                *self.count_mut(kind) = new_count;
                return Ok(QuotaDecision::Reserved(kind));
            }
            if earliest_closed.is_none_or(|(_, opens_at)| pool.opens_at < opens_at) {
                earliest_closed = Some((kind, pool.opens_at));
            }
        }
        Ok(match earliest_closed {
            Some((quota_kind, opens_at)) => QuotaDecision::NotYetOpen {
                quota_kind,
                opens_at,
            },
            None => QuotaDecision::Exhausted,
        })
    }

    pub fn release(&mut self, kind: ReservationQuotaKind) -> Result<(), &'static str> {
        let count = self.count_mut(kind);
        *count = count.checked_sub(1).ok_or("quota usage underflow")?;
        Ok(())
    }

    /// Releasing an unclassified historical allocation returns a place to the event total only.
    pub fn release_legacy_unclassified(&mut self) -> Result<(), &'static str> {
        self.legacy_unclassified = self
            .legacy_unclassified
            .checked_sub(1)
            .ok_or("quota usage underflow")?;
        Ok(())
    }

    /// Manager edits cannot reduce any limit below current allocations, even if other pools have room.
    pub fn validate_limits(
        self,
        quotas: &ReservationQuotas,
        event_max_slots: u32,
    ) -> Result<(), &'static str> {
        let total = self.total()?;
        if event_max_slots != 0 && total > u64::from(event_max_slots) {
            return Err("event capacity is below current allocation");
        }
        for kind in [
            ReservationQuotaKind::Member,
            ReservationQuotaKind::Guest,
            ReservationQuotaKind::Open,
        ] {
            if quotas
                .pool(kind)
                .seats
                .is_some_and(|limit| self.count(kind) > u64::from(limit))
            {
                return Err("quota capacity is below current allocation");
            }
        }
        Ok(())
    }

    fn count_mut(&mut self, kind: ReservationQuotaKind) -> &mut u64 {
        match kind {
            ReservationQuotaKind::Member => &mut self.member,
            ReservationQuotaKind::Guest => &mut self.guest,
            ReservationQuotaKind::Open => &mut self.open,
        }
    }
}

#[cfg(test)]
#[path = "tests/quota_selection.rs"]
mod tests;
