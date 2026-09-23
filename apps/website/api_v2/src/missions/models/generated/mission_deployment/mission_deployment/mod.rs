// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/mission-deployment.schema.json — regenerate with: cargo xtask ci schema-codegen

//! The `MissionDeployment` definition and the types typify derives from it.

mod artifact_digest;
pub use artifact_digest::*;
mod artifact_sha256;
pub use artifact_sha256::*;
mod failure_reason;
pub use failure_reason::*;
mod fleet_command_state;
pub use fleet_command_state::*;
mod requested_by;
pub use requested_by::*;
mod requested_via;
pub use requested_via::*;
mod scenario_id;
pub use scenario_id::*;
mod terrain_key;
pub use terrain_key::*;

use super::{DeploymentState, DeploymentTransition, MissionDeploymentContract};

///POST /api/v1/servers/:id/deployments (administrator, answered 202) and GET /api/v1/servers/:id/deployments/:deploymentId: an approved artifact selected for a server, the transition that runs it, and its outcome. A deployment is confirmed only by a runtime session of the server, started after the request, that reports its artifact with the exact document SHA-256.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct MissionDeployment {
    pub artifact_digest: MissionDeploymentArtifactDigest,
    pub artifact_id: ::uuid::Uuid,
    pub artifact_sha256: MissionDeploymentArtifactSha256,
    pub bound_slots: u64,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub confirmed_runtime_session_id: ::std::option::Option<::uuid::Uuid>,
    pub deadline_at: ::chrono::DateTime<::chrono::offset::Utc>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub event_mission_id: ::std::option::Option<::uuid::Uuid>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub failure_reason: ::std::option::Option<MissionDeploymentFailureReason>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub finished_at: ::std::option::Option<::chrono::DateTime<::chrono::offset::Utc>>,
    pub fleet_command_id: ::uuid::Uuid,
    pub fleet_command_state: MissionDeploymentFleetCommandState,
    pub id: ::uuid::Uuid,
    pub mission_id: ::uuid::Uuid,
    pub mission_title: ::std::string::String,
    pub requested_at: ::chrono::DateTime<::chrono::offset::Utc>,
    pub requested_by: MissionDeploymentRequestedBy,
    pub requested_via: MissionDeploymentRequestedVia,
    pub scenario_id: MissionDeploymentScenarioId,
    pub server_id: ::uuid::Uuid,
    pub state: DeploymentState,
    pub terrain_key: MissionDeploymentTerrainKey,
    pub transition: DeploymentTransition,
}
impl ::std::convert::From<MissionDeploymentContract> for MissionDeployment {
    fn from(value: MissionDeploymentContract) -> Self {
        value.0
    }
}
