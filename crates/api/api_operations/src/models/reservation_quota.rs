//! Event-wide participant quotas. Zero closes a pool; explicit null leaves its count uncapped.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

/// One of an event's three reservation pools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReservationQuotaKind {
    /// Places verified TBD members draw from first.
    Member,
    /// Places other admitted accounts draw from first.
    Guest,
    /// Places any admitted account falls back to once its own pool cannot grant one.
    Open,
}

impl ReservationQuotaKind {
    /// The Postgres/JSON wire string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Member => "member",
            Self::Guest => "guest",
            Self::Open => "open",
        }
    }
}

/// One pool's place limit and opening time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReservationQuotaPool {
    /// Place limit: `0` closes the pool and an explicit `null` leaves it uncapped.
    // An absent limit must not silently authorize unlimited reservations.
    #[serde(deserialize_with = "required_seat_limit")]
    pub seats: Option<u32>,
    /// When the pool starts granting places, as an RFC 3339 UTC timestamp.
    #[serde(with = "fleet_wire_contract::rfc3339_timestamps::rfc3339_utc")]
    pub opens_at: DateTime<Utc>,
}

fn required_seat_limit<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u32>, D::Error> {
    Option::<u32>::deserialize(d)
}

/// The event's member, guest and open pools; every pool must be present.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReservationQuotas {
    /// The pool verified TBD members draw from first.
    pub member: ReservationQuotaPool,
    /// The pool other admitted accounts draw from first.
    pub guest: ReservationQuotaPool,
    /// The fallback pool any admitted account may draw from.
    pub open: ReservationQuotaPool,
}

impl ReservationQuotas {
    /// The pool of the given kind.
    pub fn pool(&self, kind: ReservationQuotaKind) -> &ReservationQuotaPool {
        match kind {
            ReservationQuotaKind::Member => &self.member,
            ReservationQuotaKind::Guest => &self.guest,
            ReservationQuotaKind::Open => &self.open,
        }
    }
}
