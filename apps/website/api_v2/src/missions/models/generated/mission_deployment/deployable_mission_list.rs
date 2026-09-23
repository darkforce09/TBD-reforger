// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/mission-deployment.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::DeployableMission;

///GET /api/v1/game-runtime/missions (mod_runtime credential): every live mission whose approved artifact the server can run.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct DeployableMissionList {
    pub missions: ::std::vec::Vec<DeployableMission>,
}
