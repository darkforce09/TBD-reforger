// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/event-hub.schema.json — regenerate with: cargo xtask ci schema-codegen

///`ReservationQuotaAvailability`
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ReservationQuotaAvailability {
    pub allocated: u64,
    pub closed_reason: ::std::option::Option<ReservationQuotaAvailabilityClosedReason>,
    pub open: bool,
    pub opens_at: ::chrono::DateTime<::chrono::offset::Utc>,
    pub quota_kind: ReservationQuotaAvailabilityQuotaKind,
    pub remaining: ::std::option::Option<u64>,
    pub seat_limit: ::std::option::Option<u64>,
}
///`ReservationQuotaAvailabilityClosedReason`
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
pub enum ReservationQuotaAvailabilityClosedReason {
    #[serde(rename = "not_yet_open")]
    NotYetOpen,
    #[serde(rename = "no_places")]
    NoPlaces,
    #[serde(rename = "full")]
    Full,
}
impl ::std::fmt::Display for ReservationQuotaAvailabilityClosedReason {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::NotYetOpen => f.write_str("not_yet_open"),
            Self::NoPlaces => f.write_str("no_places"),
            Self::Full => f.write_str("full"),
        }
    }
}
impl ::std::str::FromStr for ReservationQuotaAvailabilityClosedReason {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "not_yet_open" => Ok(Self::NotYetOpen),
            "no_places" => Ok(Self::NoPlaces),
            "full" => Ok(Self::Full),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for ReservationQuotaAvailabilityClosedReason {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ReservationQuotaAvailabilityClosedReason {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ReservationQuotaAvailabilityClosedReason {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
///`ReservationQuotaAvailabilityQuotaKind`
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
pub enum ReservationQuotaAvailabilityQuotaKind {
    #[serde(rename = "member")]
    Member,
    #[serde(rename = "guest")]
    Guest,
    #[serde(rename = "open")]
    Open,
}
impl ::std::fmt::Display for ReservationQuotaAvailabilityQuotaKind {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Member => f.write_str("member"),
            Self::Guest => f.write_str("guest"),
            Self::Open => f.write_str("open"),
        }
    }
}
impl ::std::str::FromStr for ReservationQuotaAvailabilityQuotaKind {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "member" => Ok(Self::Member),
            "guest" => Ok(Self::Guest),
            "open" => Ok(Self::Open),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for ReservationQuotaAvailabilityQuotaKind {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for ReservationQuotaAvailabilityQuotaKind {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for ReservationQuotaAvailabilityQuotaKind {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
