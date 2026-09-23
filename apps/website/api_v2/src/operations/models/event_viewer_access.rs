//! What the viewing account may see and reserve in one event: its visibility, its pool class,
//! pool availability, and per-seat eligibility. Other participants' eligibility is never shown.

use chrono::{DateTime, Utc};
use serde::Serialize;

use super::reservation_quota::ReservationQuotaKind;
use crate::core::wire_format::rfc3339_utc;
use crate::operations::services::event_access::evaluation::PolicySource;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventVisibilityLevel {
    /// The event policy admits the viewer.
    Full,
    /// Only squad or slot policies admit the viewer; only those seats are shown.
    Partial,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuotaClosedReason {
    NotYetOpen,
    /// The pool's limit is zero.
    NoPlaces,
    /// Every place of the pool is allocated.
    Full,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReservationQuotaAvailability {
    pub quota_kind: ReservationQuotaKind,
    /// `None` leaves the pool uncapped; the event-wide limit still applies.
    pub seat_limit: Option<u32>,
    pub allocated: u64,
    pub remaining: Option<u64>,
    #[serde(with = "rfc3339_utc")]
    pub opens_at: DateTime<Utc>,
    pub open: bool,
    pub closed_reason: Option<QuotaClosedReason>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EventViewerAccess {
    pub visibility: EventVisibilityLevel,
    /// The pool a new place comes from first: member for verified TBD members, otherwise guest.
    pub quota_class: ReservationQuotaKind,
    /// Discord verification is pending or stale for a guild this event's policies rely on.
    pub membership_verification_pending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotViewerAccess {
    Eligible,
    Restricted,
}

/// The viewer's standing for one seat of the ORBAT.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct SlotViewerEligibility {
    pub viewer_access: SlotViewerAccess,
    pub policy_source: PolicySource,
}
