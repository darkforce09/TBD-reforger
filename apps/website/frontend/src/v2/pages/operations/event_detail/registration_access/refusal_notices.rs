//! What the viewer is told when a registration is refused.
//!
//! **Role:** reads the reason a refused registration names into one of the known refusals, and
//! words each one — which policy refused, when a pool opens, whether the waiting list is the way
//! forward — instead of a generic failure.
//! **Position:** read by the slotting footer after a registration or waiting-list request fails,
//! and by the waiting-list promotion control.
//! **Signals & state:** none; pure over the refusal.
//! **Invariants:** a reason this build does not know falls back to the backend's own sentence, so an
//! unfamiliar refusal still says something true. An opening time is shown in the viewer's own zone
//! with its UTC time beside it; the local rendering is handed in, which keeps every sentence here
//! testable without a browser clock. No sentence names another participant.

use crate::v2::core::api::client::ApiRefusal;
use crate::v2::core::utils::utc_timestamp::utc_label;

/// A registration refusal, by the reason the backend names.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RegistrationRefusal {
    /// The seat's effective policy does not admit the viewer; `policy_source` names which one.
    AccessPolicy {
        policy_source: Option<String>,
    },
    /// A Discord verification the policy relies on has not completed yet.
    MembershipVerificationRequired,
    /// The viewer's pools have places, but none has opened yet.
    QuotaNotOpen {
        quota_kind: Option<String>,
        opens_at: Option<String>,
    },
    /// The operation-wide limit, or every pool the viewer draws from, is exhausted.
    EventFull,
    /// Every seat of the mission is allocated.
    MissionFull,
    /// The seat is the last one a participant already holding a place can take.
    SeatNeededByHolder,
    SeatTaken,
    SquadHeld,
    NoSeats,
    RegistrationClosed,
    AccountUnavailable,
    DeploymentRequirements,
    /// A refusal without a known reason, carrying the sentence to show.
    Other(String),
}

impl RegistrationRefusal {
    /// Read a refused request; `fallback` is shown when the backend sent no sentence at all.
    pub(crate) fn from_refusal(refusal: &ApiRefusal, fallback: &str) -> Self {
        let text = |key: &str| refusal.detail(key).map(str::to_string);
        match refusal.code() {
            Some("ACCESS_POLICY") => Self::AccessPolicy {
                policy_source: text("policy_source"),
            },
            Some("MEMBERSHIP_VERIFICATION_REQUIRED") => Self::MembershipVerificationRequired,
            Some("QUOTA_NOT_OPEN") => Self::QuotaNotOpen {
                quota_kind: text("quota_kind"),
                opens_at: text("opens_at"),
            },
            Some("EVENT_FULL") => Self::EventFull,
            Some("MISSION_FULL") => Self::MissionFull,
            Some("SEAT_NEEDED_BY_HOLDER") => Self::SeatNeededByHolder,
            Some("SEAT_TAKEN") => Self::SeatTaken,
            Some("SQUAD_HELD") => Self::SquadHeld,
            Some("NO_SEATS") => Self::NoSeats,
            Some("REGISTRATION_CLOSED") => Self::RegistrationClosed,
            Some("ACCOUNT_UNAVAILABLE") => Self::AccountUnavailable,
            Some("DEPLOYMENT_REQUIREMENTS") => Self::DeploymentRequirements,
            _ => Self::Other(refusal.message_or(fallback)),
        }
    }

    /// Whether joining the waiting list is the way forward: there is no place or seat to take now,
    /// and a waiting entry is seated when one frees up.
    pub(crate) fn suggests_waiting_list(&self) -> bool {
        matches!(
            self,
            Self::EventFull | Self::MissionFull | Self::SeatNeededByHolder
        )
    }

    /// The sentence the viewer is shown. `local_time` renders an instant in the viewer's own zone.
    pub(crate) fn sentence(&self, local_time: impl Fn(&str) -> String) -> String {
        match self {
            Self::AccessPolicy { policy_source } => format!(
                "{} does not admit you to this seat.",
                policy_owner(policy_source.as_deref())
            ),
            Self::MembershipVerificationRequired => {
                "Your Discord membership is still being verified for this operation. Try again \
                 once verification completes."
                    .to_string()
            }
            Self::QuotaNotOpen {
                quota_kind,
                opens_at,
            } => {
                let pool = pool_name(quota_kind.as_deref());
                match opens_at {
                    Some(at) => format!("{pool} open {} ({}).", local_time(at), utc_label(at)),
                    None => format!("{pool} are not open yet."),
                }
            }
            Self::EventFull => "This operation is full: no place is free for you. Join the \
                                waiting list to be seated when one frees up."
                .to_string(),
            Self::MissionFull => "This mission is full. Join the waiting list to be seated when \
                                  a seat frees up."
                .to_string(),
            Self::SeatNeededByHolder => "This seat is the last one a participant already holding \
                                         a place can take. Pick another seat or join the \
                                         waiting list."
                .to_string(),
            Self::SeatTaken => "Someone took this seat first. Pick another one.".to_string(),
            Self::SquadHeld => "A leader has reserved this squad; only they or an administrator \
                                can seat it."
                .to_string(),
            Self::NoSeats => "This mission has no seats to register for.".to_string(),
            Self::RegistrationClosed => "Registration is closed for this operation.".to_string(),
            Self::AccountUnavailable => {
                "Your account is unavailable, so it cannot register.".to_string()
            }
            Self::DeploymentRequirements => {
                "You do not meet this operation's deployment requirements.".to_string()
            }
            Self::Other(sentence) => sentence.clone(),
        }
    }
}

/// The policy a refusal names, as the subject of a sentence.
pub(crate) fn policy_owner(policy_source: Option<&str>) -> &'static str {
    match policy_source {
        Some("slot") => "This seat's access policy",
        Some("squad") => "This squad's access policy",
        Some("event") => "The operation's access policy",
        _ => "An access policy",
    }
}

/// A pool's places, as the subject of a sentence: `Guest places`.
pub(crate) fn pool_name(quota_kind: Option<&str>) -> String {
    match quota_kind {
        Some("member") => "Member places".to_string(),
        Some("guest") => "Guest places".to_string(),
        Some("open") => "Open places".to_string(),
        Some(other) => {
            let words = other.replace('_', " ");
            let mut letters = words.chars();
            match letters.next() {
                Some(first) => format!("{}{} places", first.to_uppercase(), letters.as_str()),
                None => "Places".to_string(),
            }
        }
        None => "Places".to_string(),
    }
}
