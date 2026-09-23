// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/mission-deployment.schema.json — regenerate with: cargo xtask ci schema-codegen

///scenario_restart: the runtime already runs the artifact's terrain and restarts its scenario in-process (fleet action load_mission). host_restart: the host agent restarts the server on the terrain's registered scenario (fleet action restart_with_mission).
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
pub enum DeploymentTransition {
    #[serde(rename = "scenario_restart")]
    ScenarioRestart,
    #[serde(rename = "host_restart")]
    HostRestart,
}
impl ::std::fmt::Display for DeploymentTransition {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::ScenarioRestart => f.write_str("scenario_restart"),
            Self::HostRestart => f.write_str("host_restart"),
        }
    }
}
impl ::std::str::FromStr for DeploymentTransition {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "scenario_restart" => Ok(Self::ScenarioRestart),
            "host_restart" => Ok(Self::HostRestart),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for DeploymentTransition {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for DeploymentTransition {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for DeploymentTransition {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
