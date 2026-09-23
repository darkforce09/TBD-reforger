// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/mission-deployment.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::MissionDeployment;

///GET /api/v1/servers/:id/deployments (administrator): the server's deployments, newest first. POST .../deployments/:deploymentId/cancel answers the cancelled MissionDeployment while its command is queued, else 409.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct MissionDeploymentPage {
    pub items: ::std::vec::Vec<MissionDeployment>,
    pub limit: ::std::num::NonZeroU64,
    pub offset: u64,
    pub total: u64,
}
