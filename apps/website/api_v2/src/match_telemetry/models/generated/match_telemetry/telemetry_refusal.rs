// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/match-telemetry.schema.json — regenerate with: cargo xtask ci schema-codegen

///details of a 400 or 409 telemetry refusal. index names the offending entry of the submitted array where there is one.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct TelemetryRefusal {
    pub code: TelemetryRefusalCode,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub event_id: ::std::option::Option<::std::string::String>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub field: ::std::option::Option<::std::string::String>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub index: ::std::option::Option<u64>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub match_id: ::std::option::Option<::uuid::Uuid>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub report_sha256: ::std::option::Option<TelemetryRefusalReportSha256>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub revision: ::std::option::Option<u64>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub sequence: ::std::option::Option<::std::num::NonZeroU64>,
}
///`TelemetryRefusalCode`
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
pub enum TelemetryRefusalCode {
    #[serde(rename = "MATCH_NOT_REGISTERED")]
    MatchNotRegistered,
    #[serde(rename = "REGISTRATION_CONFLICT")]
    RegistrationConflict,
    #[serde(rename = "STALE_REVISION")]
    StaleRevision,
    #[serde(rename = "REVISION_CONFLICT")]
    RevisionConflict,
    #[serde(rename = "MATCH_FINALIZED")]
    MatchFinalized,
    #[serde(rename = "EVENT_CONFLICT")]
    EventConflict,
    #[serde(rename = "EVENT_SEQUENCE_CONFLICT")]
    EventSequenceConflict,
    #[serde(rename = "INVALID_MATCH_RESULTS")]
    InvalidMatchResults,
    #[serde(rename = "INVALID_EVENT")]
    InvalidEvent,
    #[serde(rename = "EVENT_BATCH_TOO_LARGE")]
    EventBatchTooLarge,
}
impl ::std::fmt::Display for TelemetryRefusalCode {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::MatchNotRegistered => f.write_str("MATCH_NOT_REGISTERED"),
            Self::RegistrationConflict => f.write_str("REGISTRATION_CONFLICT"),
            Self::StaleRevision => f.write_str("STALE_REVISION"),
            Self::RevisionConflict => f.write_str("REVISION_CONFLICT"),
            Self::MatchFinalized => f.write_str("MATCH_FINALIZED"),
            Self::EventConflict => f.write_str("EVENT_CONFLICT"),
            Self::EventSequenceConflict => f.write_str("EVENT_SEQUENCE_CONFLICT"),
            Self::InvalidMatchResults => f.write_str("INVALID_MATCH_RESULTS"),
            Self::InvalidEvent => f.write_str("INVALID_EVENT"),
            Self::EventBatchTooLarge => f.write_str("EVENT_BATCH_TOO_LARGE"),
        }
    }
}
impl ::std::str::FromStr for TelemetryRefusalCode {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "MATCH_NOT_REGISTERED" => Ok(Self::MatchNotRegistered),
            "REGISTRATION_CONFLICT" => Ok(Self::RegistrationConflict),
            "STALE_REVISION" => Ok(Self::StaleRevision),
            "REVISION_CONFLICT" => Ok(Self::RevisionConflict),
            "MATCH_FINALIZED" => Ok(Self::MatchFinalized),
            "EVENT_CONFLICT" => Ok(Self::EventConflict),
            "EVENT_SEQUENCE_CONFLICT" => Ok(Self::EventSequenceConflict),
            "INVALID_MATCH_RESULTS" => Ok(Self::InvalidMatchResults),
            "INVALID_EVENT" => Ok(Self::InvalidEvent),
            "EVENT_BATCH_TOO_LARGE" => Ok(Self::EventBatchTooLarge),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for TelemetryRefusalCode {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for TelemetryRefusalCode {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for TelemetryRefusalCode {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
///`TelemetryRefusalReportSha256`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct TelemetryRefusalReportSha256(::std::string::String);
impl ::std::ops::Deref for TelemetryRefusalReportSha256 {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<TelemetryRefusalReportSha256> for ::std::string::String {
    fn from(value: TelemetryRefusalReportSha256) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for TelemetryRefusalReportSha256 {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        static PATTERN: ::std::sync::LazyLock<::regress::Regex> =
            ::std::sync::LazyLock::new(|| ::regress::Regex::new("^[0-9a-f]{64}$").unwrap());
        if PATTERN.find(value).is_none() {
            return Err("doesn't match pattern \"^[0-9a-f]{64}$\"".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for TelemetryRefusalReportSha256 {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for TelemetryRefusalReportSha256 {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for TelemetryRefusalReportSha256 {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for TelemetryRefusalReportSha256 {
    fn deserialize<D>(deserializer: D) -> ::std::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        ::std::string::String::deserialize(deserializer)?
            .parse()
            .map_err(|e: super::error::ConversionError| {
                <D::Error as ::serde::de::Error>::custom(e.to_string())
            })
    }
}
