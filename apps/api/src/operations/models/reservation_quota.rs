//! Event-wide participant quotas. Zero closes a pool; explicit null leaves its count uncapped.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReservationQuotaKind {
    Member,
    Guest,
    Open,
}

impl ReservationQuotaKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Member => "member",
            Self::Guest => "guest",
            Self::Open => "open",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReservationQuotaPool {
    // An absent limit must not silently authorize unlimited reservations.
    #[serde(deserialize_with = "required_seat_limit")]
    pub seats: Option<u32>,
    #[serde(with = "crate::core::wire_format::rfc3339_utc")]
    pub opens_at: DateTime<Utc>,
}

fn required_seat_limit<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u32>, D::Error> {
    Option::<u32>::deserialize(d)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReservationQuotas {
    pub member: ReservationQuotaPool,
    pub guest: ReservationQuotaPool,
    pub open: ReservationQuotaPool,
}

impl ReservationQuotas {
    pub fn pool(&self, kind: ReservationQuotaKind) -> &ReservationQuotaPool {
        match kind {
            ReservationQuotaKind::Member => &self.member,
            ReservationQuotaKind::Guest => &self.guest,
            ReservationQuotaKind::Open => &self.open,
        }
    }
}
