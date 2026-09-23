// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/event-hub.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::ArmoryFaction;

///`EventMissionDossier`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct EventMissionDossier {
    pub armory_by_faction: ::std::vec::Vec<ArmoryFaction>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub briefing: ::std::option::Option<::std::string::String>,
    pub event_mission_id: ::uuid::Uuid,
    pub factions: ::std::vec::Vec<::std::string::String>,
    pub filled: u64,
    pub game_mode: ::std::string::String,
    pub mission_id: ::uuid::Uuid,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub my_attendance_state: ::std::option::Option<EventMissionDossierMyAttendanceState>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub my_release_reason: ::std::option::Option<EventMissionDossierMyReleaseReason>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub my_reservation_state: ::std::option::Option<EventMissionDossierMyReservationState>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub my_slot_id: ::std::option::Option<::uuid::Uuid>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub my_state: ::std::option::Option<EventMissionDossierMyState>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub my_waiting_position: ::std::option::Option<::std::num::NonZeroU64>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub my_withdrawn_at: ::std::option::Option<::chrono::DateTime<::chrono::offset::Utc>>,
    pub start_time: ::chrono::DateTime<::chrono::offset::Utc>,
    pub terrain: ::std::string::String,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub thumbnail_url: ::std::option::Option<::std::string::String>,
    pub title: ::std::string::String,
    pub total: u64,
    ///Some seat of this mission admits the viewer under current membership authority.
    pub viewer_eligible: bool,
}
///`EventMissionDossierMyAttendanceState`
#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub enum EventMissionDossierMyAttendanceState {
    #[serde(rename = "attended")]
    Attended,
    #[serde(rename = "no_show")]
    NoShow,
}
impl ::std::fmt::Display for EventMissionDossierMyAttendanceState {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Attended => f.write_str("attended"),
            Self::NoShow => f.write_str("no_show"),
        }
    }
}
impl ::std::str::FromStr for EventMissionDossierMyAttendanceState {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "attended" => Ok(Self::Attended),
            "no_show" => Ok(Self::NoShow),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for EventMissionDossierMyAttendanceState {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for EventMissionDossierMyAttendanceState {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for EventMissionDossierMyAttendanceState {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
///`EventMissionDossierMyReleaseReason`
#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub enum EventMissionDossierMyReleaseReason {
    #[serde(rename = "participant_withdrew")]
    ParticipantWithdrew,
    #[serde(rename = "mission_removed")]
    MissionRemoved,
    #[serde(rename = "event_cancelled")]
    EventCancelled,
    #[serde(rename = "event_deleted")]
    EventDeleted,
    #[serde(rename = "eligibility_lost")]
    EligibilityLost,
    #[serde(rename = "access_policy_changed")]
    AccessPolicyChanged,
    #[serde(rename = "account_unavailable")]
    AccountUnavailable,
    #[serde(rename = "seat_cleared")]
    SeatCleared,
}
impl ::std::fmt::Display for EventMissionDossierMyReleaseReason {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::ParticipantWithdrew => f.write_str("participant_withdrew"),
            Self::MissionRemoved => f.write_str("mission_removed"),
            Self::EventCancelled => f.write_str("event_cancelled"),
            Self::EventDeleted => f.write_str("event_deleted"),
            Self::EligibilityLost => f.write_str("eligibility_lost"),
            Self::AccessPolicyChanged => f.write_str("access_policy_changed"),
            Self::AccountUnavailable => f.write_str("account_unavailable"),
            Self::SeatCleared => f.write_str("seat_cleared"),
        }
    }
}
impl ::std::str::FromStr for EventMissionDossierMyReleaseReason {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "participant_withdrew" => Ok(Self::ParticipantWithdrew),
            "mission_removed" => Ok(Self::MissionRemoved),
            "event_cancelled" => Ok(Self::EventCancelled),
            "event_deleted" => Ok(Self::EventDeleted),
            "eligibility_lost" => Ok(Self::EligibilityLost),
            "access_policy_changed" => Ok(Self::AccessPolicyChanged),
            "account_unavailable" => Ok(Self::AccountUnavailable),
            "seat_cleared" => Ok(Self::SeatCleared),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for EventMissionDossierMyReleaseReason {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for EventMissionDossierMyReleaseReason {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for EventMissionDossierMyReleaseReason {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
///`EventMissionDossierMyReservationState`
#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub enum EventMissionDossierMyReservationState {
    #[serde(rename = "registered")]
    Registered,
    #[serde(rename = "waitlisted")]
    Waitlisted,
    #[serde(rename = "withdrawn")]
    Withdrawn,
    #[serde(rename = "legacy_unknown")]
    LegacyUnknown,
}
impl ::std::fmt::Display for EventMissionDossierMyReservationState {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Registered => f.write_str("registered"),
            Self::Waitlisted => f.write_str("waitlisted"),
            Self::Withdrawn => f.write_str("withdrawn"),
            Self::LegacyUnknown => f.write_str("legacy_unknown"),
        }
    }
}
impl ::std::str::FromStr for EventMissionDossierMyReservationState {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "registered" => Ok(Self::Registered),
            "waitlisted" => Ok(Self::Waitlisted),
            "withdrawn" => Ok(Self::Withdrawn),
            "legacy_unknown" => Ok(Self::LegacyUnknown),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for EventMissionDossierMyReservationState {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for EventMissionDossierMyReservationState {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for EventMissionDossierMyReservationState {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
///`EventMissionDossierMyState`
#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub enum EventMissionDossierMyState {
    #[serde(rename = "registered")]
    Registered,
    #[serde(rename = "waitlisted")]
    Waitlisted,
    #[serde(rename = "withdrawn")]
    Withdrawn,
    #[serde(rename = "legacy_unknown")]
    LegacyUnknown,
    #[serde(rename = "attended")]
    Attended,
    #[serde(rename = "no_show")]
    NoShow,
}
impl ::std::fmt::Display for EventMissionDossierMyState {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Registered => f.write_str("registered"),
            Self::Waitlisted => f.write_str("waitlisted"),
            Self::Withdrawn => f.write_str("withdrawn"),
            Self::LegacyUnknown => f.write_str("legacy_unknown"),
            Self::Attended => f.write_str("attended"),
            Self::NoShow => f.write_str("no_show"),
        }
    }
}
impl ::std::str::FromStr for EventMissionDossierMyState {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "registered" => Ok(Self::Registered),
            "waitlisted" => Ok(Self::Waitlisted),
            "withdrawn" => Ok(Self::Withdrawn),
            "legacy_unknown" => Ok(Self::LegacyUnknown),
            "attended" => Ok(Self::Attended),
            "no_show" => Ok(Self::NoShow),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for EventMissionDossierMyState {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for EventMissionDossierMyState {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for EventMissionDossierMyState {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
