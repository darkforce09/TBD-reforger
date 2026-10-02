//! Why a seat, place or promotion is refused, and the stable wire form of each reason.
//!
//! Every refusal carries `details.code` so clients branch on a machine-readable value. Messages
//! never name another participant: a seat needed by an existing holder is reported as such
//! without identifying the holder.

use axum::http::StatusCode;
use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::json;

use crate::core::error_handling::api_error::ApiError;
use crate::operations::models::reservation_quota::ReservationQuotaKind;
use crate::operations::services::event_access::evaluation::{AccessDenial, PolicySource};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaimRefusal {
    /// A mandatory gate or the effective access policy refused the account.
    Denied {
        denial: AccessDenial,
        policy_source: PolicySource,
    },
    /// The event-wide participant limit or every applicable pool is exhausted.
    EventFull,
    /// Every physical seat of the attachment is allocated.
    MissionFull,
    /// Taking the seat would leave a participant who already holds a place without any seat.
    SeatNeededByHolder,
    SeatTaken,
    SquadHeldByLeader,
    /// The attachment has no ORBAT seats, so there is nothing to reserve.
    NoSeats,
}

/// The refusal asks for Discord membership verification that has not completed yet.
pub fn awaits_membership_verification(error: &ApiError) -> bool {
    error
        .details
        .as_ref()
        .and_then(|details| details.get("code"))
        .and_then(serde_json::Value::as_str)
        == Some("MEMBERSHIP_VERIFICATION_REQUIRED")
}

fn opening(quota_kind: ReservationQuotaKind, opens_at: DateTime<Utc>) -> ApiError {
    let opens_at = opens_at.to_rfc3339_opts(SecondsFormat::Micros, true);
    ApiError::with_details(
        StatusCode::CONFLICT,
        format!("{} reservations open at {opens_at}", quota_kind.as_str()),
        json!({"code": "QUOTA_NOT_OPEN", "quota_kind": quota_kind.as_str(), "opens_at": opens_at}),
    )
}

impl From<ClaimRefusal> for ApiError {
    fn from(refusal: ClaimRefusal) -> Self {
        let coded = |status, message: &str, code: &str| {
            ApiError::with_details(status, message, json!({ "code": code }))
        };
        match refusal {
            ClaimRefusal::Denied {
                denial,
                policy_source,
            } => match denial {
                AccessDenial::InvalidSession => {
                    ApiError::unauthorized("expired or revoked session")
                }
                AccessDenial::AccountUnavailable => ApiError::with_details(
                    StatusCode::FORBIDDEN,
                    "account is unavailable",
                    json!({"code": "ACCOUNT_UNAVAILABLE"}),
                ),
                AccessDenial::Policy => ApiError::with_details(
                    StatusCode::FORBIDDEN,
                    "this operation's access policy does not admit you here",
                    json!({"code": "ACCESS_POLICY", "policy_source": policy_source}),
                ),
                AccessDenial::MembershipVerificationRequired => ApiError::with_details(
                    StatusCode::FORBIDDEN,
                    "your Discord membership is being verified; try again once verification completes",
                    json!({"code": "MEMBERSHIP_VERIFICATION_REQUIRED", "policy_source": policy_source}),
                ),
                AccessDenial::RegistrationClosed => coded(
                    StatusCode::CONFLICT,
                    "registration is closed for this operation",
                    "REGISTRATION_CLOSED",
                ),
                AccessDenial::QuotaNotOpen {
                    quota_kind,
                    opens_at,
                } => opening(quota_kind, opens_at),
                AccessDenial::DeploymentRequirements => coded(
                    StatusCode::CONFLICT,
                    "deployment requirements are not met",
                    "DEPLOYMENT_REQUIREMENTS",
                ),
                AccessDenial::Capacity => ClaimRefusal::EventFull.into(),
            },
            ClaimRefusal::EventFull => coded(
                StatusCode::CONFLICT,
                "this operation is full — its slot cap has been reached",
                "EVENT_FULL",
            ),
            ClaimRefusal::MissionFull => coded(
                StatusCode::CONFLICT,
                "this mission has no unallocated capacity",
                "MISSION_FULL",
            ),
            ClaimRefusal::SeatNeededByHolder => coded(
                StatusCode::CONFLICT,
                "this seat is the last one a participant already holding a place can take",
                "SEAT_NEEDED_BY_HOLDER",
            ),
            ClaimRefusal::SeatTaken => {
                coded(StatusCode::CONFLICT, "slot already taken", "SEAT_TAKEN")
            }
            ClaimRefusal::SquadHeldByLeader => coded(
                StatusCode::CONFLICT,
                "squad is reserved by a leader",
                "SQUAD_HELD",
            ),
            ClaimRefusal::NoSeats => coded(
                StatusCode::CONFLICT,
                "this operation has no ORBAT slots, so there is nothing to register for",
                "NO_SEATS",
            ),
        }
    }
}
