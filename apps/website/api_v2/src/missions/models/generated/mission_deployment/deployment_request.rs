// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/mission-deployment.schema.json — regenerate with: cargo xtask ci schema-codegen

///POST /api/v1/servers/:id/deployments body. Refusals persist nothing: 409 SERVER_INACTIVE, DEPLOYMENT_IN_PROGRESS, ARTIFACT_NOT_APPROVED or EVENT_MISSION_NOT_ON_SERVER; 422 MODPACK_MISMATCH, TERRAIN_NOT_RUNNABLE or ORBAT_ARTIFACT_MISMATCH (details name every unbound seat and unseated slot).
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct DeploymentRequest {
    pub artifact_id: ::uuid::Uuid,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub event_mission_id: ::std::option::Option<::uuid::Uuid>,
    pub mission_id: ::uuid::Uuid,
}
