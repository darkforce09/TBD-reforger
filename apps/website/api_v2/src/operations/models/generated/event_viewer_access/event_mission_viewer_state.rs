// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/event-viewer-access.schema.json — regenerate with: cargo xtask ci schema-codegen

///Viewer fields of each hub mission: viewer_eligible (some seat admits the viewer now), my_reservation_state, my_attendance_state, my_slot_id, my_release_reason and my_withdrawn_at (a released signup keeps its tombstone), my_waiting_position (one-based queue position while waitlisted). Absent fields have no value.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
pub struct EventMissionViewerState {
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub my_attendance_state: ::std::option::Option<EventMissionViewerStateMyAttendanceState>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub my_release_reason: ::std::option::Option<EventMissionViewerStateMyReleaseReason>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub my_reservation_state: ::std::option::Option<EventMissionViewerStateMyReservationState>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub my_slot_id: ::std::option::Option<::uuid::Uuid>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub my_waiting_position: ::std::option::Option<::std::num::NonZeroU64>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub my_withdrawn_at: ::std::option::Option<::chrono::DateTime<::chrono::offset::Utc>>,
    pub viewer_eligible: bool,
}
///`EventMissionViewerStateMyAttendanceState`
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
pub enum EventMissionViewerStateMyAttendanceState {
    #[serde(rename = "attended")]
    Attended,
    #[serde(rename = "no_show")]
    NoShow,
}
impl ::std::fmt::Display for EventMissionViewerStateMyAttendanceState {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Attended => f.write_str("attended"),
            Self::NoShow => f.write_str("no_show"),
        }
    }
}
impl ::std::str::FromStr for EventMissionViewerStateMyAttendanceState {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "attended" => Ok(Self::Attended),
            "no_show" => Ok(Self::NoShow),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for EventMissionViewerStateMyAttendanceState {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for EventMissionViewerStateMyAttendanceState {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for EventMissionViewerStateMyAttendanceState {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
///`EventMissionViewerStateMyReleaseReason`
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
pub enum EventMissionViewerStateMyReleaseReason {
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
impl ::std::fmt::Display for EventMissionViewerStateMyReleaseReason {
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
impl ::std::str::FromStr for EventMissionViewerStateMyReleaseReason {
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
impl ::std::convert::TryFrom<&str> for EventMissionViewerStateMyReleaseReason {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for EventMissionViewerStateMyReleaseReason {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for EventMissionViewerStateMyReleaseReason {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
///`EventMissionViewerStateMyReservationState`
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
pub enum EventMissionViewerStateMyReservationState {
    #[serde(rename = "registered")]
    Registered,
    #[serde(rename = "waitlisted")]
    Waitlisted,
    #[serde(rename = "withdrawn")]
    Withdrawn,
    #[serde(rename = "legacy_unknown")]
    LegacyUnknown,
}
impl ::std::fmt::Display for EventMissionViewerStateMyReservationState {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Registered => f.write_str("registered"),
            Self::Waitlisted => f.write_str("waitlisted"),
            Self::Withdrawn => f.write_str("withdrawn"),
            Self::LegacyUnknown => f.write_str("legacy_unknown"),
        }
    }
}
impl ::std::str::FromStr for EventMissionViewerStateMyReservationState {
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
impl ::std::convert::TryFrom<&str> for EventMissionViewerStateMyReservationState {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for EventMissionViewerStateMyReservationState {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for EventMissionViewerStateMyReservationState {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
