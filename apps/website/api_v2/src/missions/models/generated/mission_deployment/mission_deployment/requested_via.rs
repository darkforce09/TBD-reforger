// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/mission-deployment.schema.json — regenerate with: cargo xtask ci schema-codegen

///`MissionDeploymentRequestedVia`
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
pub enum MissionDeploymentRequestedVia {
    #[serde(rename = "web")]
    Web,
    #[serde(rename = "game_runtime")]
    GameRuntime,
}
impl ::std::fmt::Display for MissionDeploymentRequestedVia {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Web => f.write_str("web"),
            Self::GameRuntime => f.write_str("game_runtime"),
        }
    }
}
impl ::std::str::FromStr for MissionDeploymentRequestedVia {
    type Err = super::super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::super::error::ConversionError> {
        match value {
            "web" => Ok(Self::Web),
            "game_runtime" => Ok(Self::GameRuntime),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for MissionDeploymentRequestedVia {
    type Error = super::super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for MissionDeploymentRequestedVia {
    type Error = super::super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for MissionDeploymentRequestedVia {
    type Error = super::super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::super::error::ConversionError> {
        value.parse()
    }
}
