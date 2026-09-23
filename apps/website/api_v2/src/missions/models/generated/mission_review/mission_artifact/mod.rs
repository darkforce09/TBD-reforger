// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts_v2/definitions/mission-review.schema.json — regenerate with: cargo xtask ci schema-codegen

//! The `MissionArtifact` definition and the types typify derives from it.

mod artifact_digest;
pub use artifact_digest::*;
mod catalog_sha256;
pub use catalog_sha256::*;
mod compiler_version;
pub use compiler_version::*;
mod created_by;
pub use created_by::*;
mod document_sha256;
pub use document_sha256::*;
mod metadata_sha256;
pub use metadata_sha256::*;
mod schema_version;
pub use schema_version::*;
mod terrain;
pub use terrain::*;
mod version_payload_sha256;
pub use version_payload_sha256::*;

use super::{ArtifactDiagnostic, ArtifactMetadata};

///GET /api/v1/missions/:id/artifacts/:artifactId (the mission's author or an administrator): an immutable artifact's provenance. The exact compiled bytes are served by GET .../artifacts/:artifactId/document with their SHA-256 as a strong entity tag.
#[derive(::serde::Deserialize, ::serde::Serialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct MissionArtifact {
    pub artifact_digest: MissionArtifactArtifactDigest,
    pub catalog_sha256: MissionArtifactCatalogSha256,
    pub compiler_version: MissionArtifactCompilerVersion,
    pub created_at: ::chrono::DateTime<::chrono::offset::Utc>,
    pub created_by: MissionArtifactCreatedBy,
    pub diagnostics: ::std::vec::Vec<ArtifactDiagnostic>,
    pub document_bytes: ::std::num::NonZeroU64,
    pub document_sha256: MissionArtifactDocumentSha256,
    pub id: ::uuid::Uuid,
    pub metadata: ArtifactMetadata,
    pub metadata_sha256: MissionArtifactMetadataSha256,
    pub mission_id: ::uuid::Uuid,
    pub mission_version_id: ::uuid::Uuid,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub modpack_id: ::std::option::Option<::uuid::Uuid>,
    #[serde(default, skip_serializing_if = "::std::option::Option::is_none")]
    pub modpack_version: ::std::option::Option<::std::string::String>,
    pub schema_version: MissionArtifactSchemaVersion,
    pub terrain: MissionArtifactTerrain,
    pub version_payload_sha256: MissionArtifactVersionPayloadSha256,
}
