// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/mission-deployment.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::MissionDeployment;

///Mission deployments, what a game runtime reads to run one, and the fleet scenario registry. The root is one deployment.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(transparent)]
pub struct MissionDeploymentContract(pub MissionDeployment);
impl ::std::ops::Deref for MissionDeploymentContract {
    type Target = MissionDeployment;
    fn deref(&self) -> &MissionDeployment {
        &self.0
    }
}
impl ::std::convert::From<MissionDeployment> for MissionDeploymentContract {
    fn from(value: MissionDeployment) -> Self {
        Self(value)
    }
}
