// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/reservation-response.schema.json — regenerate with: cargo xtask ci schema-codegen

///POST /api/v1/event-missions/:id/register. state is the compatibility attendance-or-reservation projection; consumers use reservation_state for actions.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ReservationResponse {
    pub attendance_state: ::std::option::Option<ReservationResponseAttendanceState>,
    pub reservation_state: ReservationResponseReservationState,
    pub slot_id: ::std::option::Option<::uuid::Uuid>,
    pub state: ReservationResponseState,
}
///`ReservationResponseAttendanceState`
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
pub enum ReservationResponseAttendanceState {
    #[serde(rename = "attended")]
    Attended,
    #[serde(rename = "no_show")]
    NoShow,
}
impl ::std::fmt::Display for ReservationResponseAttendanceState {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Attended => f.write_str("attended"),
            Self::NoShow => f.write_str("no_show"),
        }
    }
}
impl ::std::str::FromStr for ReservationResponseAttendanceState {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "attended" => Ok(Self::Attended),
            "no_show" => Ok(Self::NoShow),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for ReservationResponseAttendanceState {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ReservationResponseAttendanceState {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ReservationResponseAttendanceState {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
///`ReservationResponseReservationState`
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
pub enum ReservationResponseReservationState {
    #[serde(rename = "registered")]
    Registered,
    #[serde(rename = "waitlisted")]
    Waitlisted,
    #[serde(rename = "withdrawn")]
    Withdrawn,
    #[serde(rename = "legacy_unknown")]
    LegacyUnknown,
}
impl ::std::fmt::Display for ReservationResponseReservationState {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Registered => f.write_str("registered"),
            Self::Waitlisted => f.write_str("waitlisted"),
            Self::Withdrawn => f.write_str("withdrawn"),
            Self::LegacyUnknown => f.write_str("legacy_unknown"),
        }
    }
}
impl ::std::str::FromStr for ReservationResponseReservationState {
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
impl ::std::convert::TryFrom<&str> for ReservationResponseReservationState {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ReservationResponseReservationState {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ReservationResponseReservationState {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
///`ReservationResponseState`
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
pub enum ReservationResponseState {
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
impl ::std::fmt::Display for ReservationResponseState {
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
impl ::std::str::FromStr for ReservationResponseState {
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
impl ::std::convert::TryFrom<&str> for ReservationResponseState {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ReservationResponseState {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ReservationResponseState {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
