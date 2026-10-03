//! The recorded quota an event participant consumed. One active allocation per participant is
//! shared by every mission reservation of that participant in the event.

use serde::{Deserialize, Serialize};

use super::reservation_quota::ReservationQuotaKind;

/// The pool a participant's allocation was recorded against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticipantAllocationKind {
    /// Recorded against the member pool.
    Member,
    /// Recorded against the guest pool.
    Guest,
    /// Recorded against the open pool.
    Open,
    /// Recorded before pools existed; counts toward the event total and toward no pool.
    LegacyUnclassified,
}

impl ParticipantAllocationKind {
    /// The Postgres/JSON wire string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Member => "member",
            Self::Guest => "guest",
            Self::Open => "open",
            Self::LegacyUnclassified => "legacy_unclassified",
        }
    }

    /// Parses a stored wire string; `None` for an unknown value.
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "member" => Some(Self::Member),
            "guest" => Some(Self::Guest),
            "open" => Some(Self::Open),
            "legacy_unclassified" => Some(Self::LegacyUnclassified),
            _ => None,
        }
    }

    /// The pool this allocation consumed, if it was recorded against one.
    pub fn pool(self) -> Option<ReservationQuotaKind> {
        match self {
            Self::Member => Some(ReservationQuotaKind::Member),
            Self::Guest => Some(ReservationQuotaKind::Guest),
            Self::Open => Some(ReservationQuotaKind::Open),
            Self::LegacyUnclassified => None,
        }
    }
}

impl From<ReservationQuotaKind> for ParticipantAllocationKind {
    fn from(kind: ReservationQuotaKind) -> Self {
        match kind {
            ReservationQuotaKind::Member => Self::Member,
            ReservationQuotaKind::Guest => Self::Guest,
            ReservationQuotaKind::Open => Self::Open,
        }
    }
}
