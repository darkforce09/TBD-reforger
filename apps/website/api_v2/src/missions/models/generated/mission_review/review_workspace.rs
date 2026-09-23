// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/mission-review.schema.json — regenerate with: cargo xtask ci schema-codegen

use super::{MissionArtifact, MissionVersion};

///GET /api/v1/missions/:id/artifacts/:artifactId/workspace (the mission's author or an administrator): the artifact and exactly the version it compiled from, verified against the payload digest the artifact recorded, for the editor to open read-only.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct ReviewWorkspace {
    pub artifact: MissionArtifact,
    pub version: MissionVersion,
}
