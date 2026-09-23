// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/mission-deployment.schema.json — regenerate with: cargo xtask ci schema-codegen

///`MissionDeploymentFleetCommandState`
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
pub enum MissionDeploymentFleetCommandState {
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
impl ::std::fmt::Display for MissionDeploymentFleetCommandState {
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
impl ::std::str::FromStr for MissionDeploymentFleetCommandState {
    type Err = super::super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::super::error::ConversionError> {
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
impl ::std::convert::TryFrom<&str> for MissionDeploymentFleetCommandState {
    type Error = super::super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for MissionDeploymentFleetCommandState {
    type Error = super::super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for MissionDeploymentFleetCommandState {
    type Error = super::super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::super::error::ConversionError> {
        value.parse()
    }
}
