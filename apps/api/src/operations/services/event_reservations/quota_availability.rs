//! Pool availability as a viewer sees it: limit, allocated places, remaining places, opening
//! time and why a pool cannot grant a place right now.

use chrono::{DateTime, Utc};

use super::quota_selection::ReservationQuotaUsage;
use crate::operations::models::event_viewer_access::{
    QuotaClosedReason, ReservationQuotaAvailability,
};
use crate::operations::models::reservation_quota::{ReservationQuotaKind, ReservationQuotas};

pub fn quota_availability(
    quotas: &ReservationQuotas,
    usage: ReservationQuotaUsage,
    now: DateTime<Utc>,
) -> Vec<ReservationQuotaAvailability> {
    [
        ReservationQuotaKind::Member,
        ReservationQuotaKind::Guest,
        ReservationQuotaKind::Open,
    ]
    .into_iter()
    .map(|quota_kind| {
        let pool = quotas.pool(quota_kind);
        let allocated = usage.count(quota_kind);
        let remaining = pool
            .seats
            .map(|limit| u64::from(limit).saturating_sub(allocated));
        let closed_reason = if pool.seats == Some(0) {
            Some(QuotaClosedReason::NoPlaces)
        } else if now < pool.opens_at {
            Some(QuotaClosedReason::NotYetOpen)
        } else if remaining == Some(0) {
            Some(QuotaClosedReason::Full)
        } else {
            None
        };
        ReservationQuotaAvailability {
            quota_kind,
            seat_limit: pool.seats,
            allocated,
            remaining,
            opens_at: pool.opens_at,
            open: closed_reason.is_none(),
            closed_reason,
        }
    })
    .collect()
}

#[cfg(test)]
#[path = "tests/quota_availability.rs"]
mod tests;
