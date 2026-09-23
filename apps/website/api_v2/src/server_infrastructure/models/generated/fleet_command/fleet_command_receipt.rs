// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/fleet-command.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::{ExecutorKind, FleetAction};

///One fleet command as operators observe it.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct FleetCommandReceipt {
    pub action: FleetAction,
    pub arguments: ::serde_json::Map<::std::string::String, ::serde_json::Value>,
    pub attempts: u64,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub claimed_at: ::std::option::Option<::chrono::DateTime<::chrono::offset::Utc>>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub executing_at: ::std::option::Option<::chrono::DateTime<::chrono::offset::Utc>>,
    pub executor_kind: ExecutorKind,
    pub expires_at: ::chrono::DateTime<::chrono::offset::Utc>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub failure_reason: ::std::option::Option<FleetCommandReceiptFailureReason>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub finished_at: ::std::option::Option<::chrono::DateTime<::chrono::offset::Utc>>,
    pub id: ::uuid::Uuid,
    #[serde(default, skip_serializing_if = "::serde_json::Map::is_empty")]
    pub outcome: ::serde_json::Map<::std::string::String, ::serde_json::Value>,
    pub requested_at: ::chrono::DateTime<::chrono::offset::Utc>,
    pub requested_by: ::std::string::String,
    pub server_id: ::uuid::Uuid,
    pub state: FleetCommandReceiptState,
}
///`FleetCommandReceiptFailureReason`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct FleetCommandReceiptFailureReason(::std::string::String);
impl ::std::ops::Deref for FleetCommandReceiptFailureReason {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<FleetCommandReceiptFailureReason> for ::std::string::String {
    fn from(value: FleetCommandReceiptFailureReason) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for FleetCommandReceiptFailureReason {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for FleetCommandReceiptFailureReason {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for FleetCommandReceiptFailureReason {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for FleetCommandReceiptFailureReason {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for FleetCommandReceiptFailureReason {
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
///`FleetCommandReceiptState`
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
pub enum FleetCommandReceiptState {
    #[serde(rename = "queued")]
    Queued,
    #[serde(rename = "claimed")]
    Claimed,
    #[serde(rename = "executing")]
    Executing,
    #[serde(rename = "succeeded")]
    Succeeded,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "expired")]
    Expired,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "indeterminate")]
    Indeterminate,
}
impl ::std::fmt::Display for FleetCommandReceiptState {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Queued => f.write_str("queued"),
            Self::Claimed => f.write_str("claimed"),
            Self::Executing => f.write_str("executing"),
            Self::Succeeded => f.write_str("succeeded"),
            Self::Failed => f.write_str("failed"),
            Self::Expired => f.write_str("expired"),
            Self::Cancelled => f.write_str("cancelled"),
            Self::Indeterminate => f.write_str("indeterminate"),
        }
    }
}
impl ::std::str::FromStr for FleetCommandReceiptState {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "queued" => Ok(Self::Queued),
            "claimed" => Ok(Self::Claimed),
            "executing" => Ok(Self::Executing),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "expired" => Ok(Self::Expired),
            "cancelled" => Ok(Self::Cancelled),
            "indeterminate" => Ok(Self::Indeterminate),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for FleetCommandReceiptState {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for FleetCommandReceiptState {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for FleetCommandReceiptState {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
