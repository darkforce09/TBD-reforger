//! What the viewing account may see and reserve in one event: its visibility, its pool class,
//! pool availability, and per-seat eligibility. Other participants' eligibility is never shown.

use chrono::{DateTime, Utc};
use serde::Serialize;

use super::reservation_quota::ReservationQuotaKind;
use crate::services::event_access::evaluation::PolicySource;
use fleet_wire_contract::rfc3339_timestamps::rfc3339_utc;

/// How much of an event's ORBAT the viewer may see.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventVisibilityLevel {
    /// The event policy admits the viewer.
    Full,
    /// Only squad or slot policies admit the viewer; only those seats are shown.
    Partial,
}

/// Why a pool cannot grant a place right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QuotaClosedReason {
    /// The pool's opening time is still ahead.
    NotYetOpen,
    /// The pool's limit is zero.
    NoPlaces,
    /// Every place of the pool is allocated.
    Full,
}

/// One pool's availability as the viewer sees it.
#[derive(Debug, Clone, Serialize)]
pub struct ReservationQuotaAvailability {
    /// The pool this entry describes.
    pub quota_kind: ReservationQuotaKind,
    /// `None` leaves the pool uncapped; the event-wide limit still applies.
    pub seat_limit: Option<u32>,
    /// Places of the pool already allocated.
    pub allocated: u64,
    /// Places left under the limit; `None` when the pool is uncapped.
    pub remaining: Option<u64>,
    /// When the pool starts granting places, as an RFC 3339 UTC timestamp.
    #[serde(with = "rfc3339_utc")]
    pub opens_at: DateTime<Utc>,
    /// The pool can grant a place now.
    pub open: bool,
    /// Why the pool cannot grant a place now; absent while it is open.
    pub closed_reason: Option<QuotaClosedReason>,
}

/// The viewer's standing in one event.
#[derive(Debug, Clone, Serialize)]
pub struct EventViewerAccess {
    /// How much of the event's ORBAT the viewer may see.
    pub visibility: EventVisibilityLevel,
    /// The pool a new place comes from first: member for verified TBD members, otherwise guest.
    pub quota_class: ReservationQuotaKind,
    /// Discord verification is pending or stale for a guild this event's policies rely on.
    pub membership_verification_pending: bool,
}

/// Whether the viewer may reserve one seat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotViewerAccess {
    /// The seat's effective policy admits the viewer.
    Eligible,
    /// The seat's effective policy does not admit the viewer.
    Restricted,
}

/// The viewer's standing for one seat of the ORBAT.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct SlotViewerEligibility {
    /// Whether the viewer may reserve the seat.
    pub viewer_access: SlotViewerAccess,
    /// Level of the policy that decides the seat: event, squad or slot.
    pub policy_source: PolicySource,
}
