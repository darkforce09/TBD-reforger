// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts/definitions/match-telemetry.schema.json — regenerate with: cargo xtask ci schema-codegen

///Match-level fields of a results revision. A present field replaces the stored value, an absent one keeps it; outcome is always present.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct MatchReport {
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub aar_replay_url: ::std::option::Option<::std::string::String>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub ended_at: ::std::option::Option<::chrono::DateTime<::chrono::offset::Utc>>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub event_id: ::std::option::Option<::uuid::Uuid>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub mission_id: ::std::option::Option<::uuid::Uuid>,
    pub outcome: MatchReportOutcome,
    pub source_match_id: MatchReportSourceMatchId,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub started_at: ::std::option::Option<::chrono::DateTime<::chrono::offset::Utc>>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub terrain: ::std::option::Option<MatchReportTerrain>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub winning_faction: ::std::option::Option<::std::string::String>,
}
///`MatchReportOutcome`
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
pub enum MatchReportOutcome {
    #[serde(rename = "success")]
    Success,
    #[serde(rename = "failure")]
    Failure,
    #[serde(rename = "aborted")]
    Aborted,
    #[serde(rename = "pending")]
    Pending,
}
impl ::std::fmt::Display for MatchReportOutcome {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Success => f.write_str("success"),
            Self::Failure => f.write_str("failure"),
            Self::Aborted => f.write_str("aborted"),
            Self::Pending => f.write_str("pending"),
        }
    }
}
impl ::std::str::FromStr for MatchReportOutcome {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "success" => Ok(Self::Success),
            "failure" => Ok(Self::Failure),
            "aborted" => Ok(Self::Aborted),
            "pending" => Ok(Self::Pending),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for MatchReportOutcome {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for MatchReportOutcome {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for MatchReportOutcome {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
///`MatchReportSourceMatchId`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct MatchReportSourceMatchId(::std::string::String);
impl ::std::ops::Deref for MatchReportSourceMatchId {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<MatchReportSourceMatchId> for ::std::string::String {
    fn from(value: MatchReportSourceMatchId) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for MatchReportSourceMatchId {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() > 128usize {
            return Err("longer than 128 characters".into());
        }
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for MatchReportSourceMatchId {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for MatchReportSourceMatchId {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for MatchReportSourceMatchId {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for MatchReportSourceMatchId {
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
///`MatchReportTerrain`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct MatchReportTerrain(::std::string::String);
impl ::std::ops::Deref for MatchReportTerrain {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<MatchReportTerrain> for ::std::string::String {
    fn from(value: MatchReportTerrain) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for MatchReportTerrain {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for MatchReportTerrain {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for MatchReportTerrain {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for MatchReportTerrain {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for MatchReportTerrain {
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
