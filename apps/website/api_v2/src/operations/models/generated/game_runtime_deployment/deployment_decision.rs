// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/game-runtime-deployment.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::{DeploymentAllowed, DeploymentDenied};

///POST /api/v1/game-runtime/sessions/:sessionId/deployments response (mod_runtime machine credential). A refusal is a decision (200 with decision denied), not an error. An allowed decision is recorded under the runtime's player_life_id, so a retried request returns it unchanged.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(untagged)]
pub enum DeploymentDecision {
    Allowed(DeploymentAllowed),
    Denied(DeploymentDenied),
}
impl ::std::convert::From<DeploymentAllowed> for DeploymentDecision {
    fn from(value: DeploymentAllowed) -> Self {
        Self::Allowed(value)
    }
}
impl ::std::convert::From<DeploymentDenied> for DeploymentDecision {
    fn from(value: DeploymentDenied) -> Self {
        Self::Denied(value)
    }
}
