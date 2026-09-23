// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/mission-deployment.schema.json — regenerate with: cargo xtask ci schema-codegen

///POST /api/v1/game-runtime/deployments body (mod_runtime credential): an in-game administrator's selection. The Arma identity must be linked to an unbanned platform administrator (403 IDENTITY_NOT_LINKED or NOT_AN_ADMINISTRATOR), who becomes the requester; the selection is validated as DeploymentRequest is. Answered 202 with the MissionDeployment.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct RelayedDeploymentRequest {
    pub artifact_id: ::uuid::Uuid,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub event_mission_id: ::std::option::Option<::uuid::Uuid>,
    pub mission_id: ::uuid::Uuid,
    pub requested_by_arma_id: RelayedDeploymentRequestRequestedByArmaId,
}
///`RelayedDeploymentRequestRequestedByArmaId`
#[derive(::serde::Serialize, Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct RelayedDeploymentRequestRequestedByArmaId(::std::string::String);
impl ::std::ops::Deref for RelayedDeploymentRequestRequestedByArmaId {
    type Target = ::std::string::String;
    fn deref(&self) -> &::std::string::String {
        &self.0
    }
}
impl ::std::convert::From<RelayedDeploymentRequestRequestedByArmaId> for ::std::string::String {
    fn from(value: RelayedDeploymentRequestRequestedByArmaId) -> Self {
        value.0
    }
}
impl ::std::str::FromStr for RelayedDeploymentRequestRequestedByArmaId {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        if value.chars().count() < 1usize {
            return Err("shorter than 1 characters".into());
        }
        Ok(Self(value.to_string()))
    }
}
impl ::std::convert::TryFrom<&str> for RelayedDeploymentRequestRequestedByArmaId {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for RelayedDeploymentRequestRequestedByArmaId {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for RelayedDeploymentRequestRequestedByArmaId {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl<'de> ::serde::Deserialize<'de> for RelayedDeploymentRequestRequestedByArmaId {
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
