//! What the viewing account may see and reserve in one operation.
//!
//! **Role:** the viewer half of the operation dossier — how much of the operation the viewer sees,
//! which reservation pool a new place comes from first, and how many places each pool still has.
//! **Position:** deserialised as part of the operation dossier and handed to the pages that render
//! it; re-serialised unchanged by the round-trip tests.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** the enumerated values travel as strings, as the reservation states do, so a value
//! the backend adds cannot make the app reject the whole operation; every reader maps a value it
//! does not know to its most conservative meaning. A pool's limit, remaining count and closed
//! reason cross the wire as explicit nulls when they have no value, so none of them is skipped when
//! serialising.

use serde::{Deserialize, Serialize};

/// How much of the operation the viewer sees, and which pool a new place comes from first.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventViewerAccess {
    /// `full` when the operation's own policy admits the viewer; `partial` when only squad or slot
    /// policies do, in which case the dossier carries only the admitted missions and seats and no
    /// operation briefing.
    pub visibility: String,
    /// `member` for a verified community member, otherwise `guest`: the pool a new place comes from
    /// first. Either overflows to the `open` pool once that pool opens.
    pub quota_class: String,
    /// Discord verification is pending or stale for a guild this operation's policies rely on.
    pub membership_verification_pending: bool,
}

/// One reservation pool, as the viewer sees it.
///
/// The operation dossier carries exactly three of these, in member, guest, open order.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReservationQuotaAvailability {
    /// `member`, `guest` or `open`.
    pub quota_kind: String,
    /// The pool's limit, or null when the pool is uncapped; the operation-wide limit still applies.
    pub seat_limit: Option<i64>,
    /// Places the pool has granted.
    pub allocated: i64,
    /// Places left in the pool, or null when the pool is uncapped.
    pub remaining: Option<i64>,
    /// When the pool starts granting places, as an RFC 3339 UTC instant.
    pub opens_at: String,
    /// Whether the pool grants a place right now.
    pub open: bool,
    /// Why the pool grants no place right now — `not_yet_open`, `no_places` (its limit is zero) or
    /// `full` — and null while it is open.
    pub closed_reason: Option<String>,
}
