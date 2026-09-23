//! Compile a mission version into an immutable artifact, or return the artifact identical inputs
//! already produced, and read artifacts back.

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use sqlx::PgConnection;
use uuid::Uuid;
use website_map_engine::data::scenario::COMPILER_PACKAGE_VERSION;
use website_map_engine::data::scenario::flatten::unsupported_authored_data;

use super::artifact_inputs::{
    canonical_json, compiled_metadata, load_catalog_snapshot, sha256_hex,
};
use crate::core::error_handling::api_error::ApiError;
use crate::core::wire_format::rfc3339_utc;
use crate::missions::contract::schema_validators::validate_mission_document;
use crate::missions::models::mission::Mission;
use crate::missions::services::mission_compile::{
    CompileError, flatten_to_mod_document_with_catalog,
};

/// An artifact's provenance as operators and reviewers see it; the document bytes are served
/// separately.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct MissionArtifact {
    pub id: Uuid,
    pub mission_id: Uuid,
    pub mission_version_id: Uuid,
    pub version_payload_sha256: String,
    pub metadata: sqlx::types::Json<Value>,
    pub metadata_sha256: String,
    pub catalog_sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modpack_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub modpack_version: Option<String>,
    pub compiler_version: String,
    pub schema_version: String,
    pub terrain: String,
    pub document_sha256: String,
    pub document_bytes: i32,
    pub diagnostics: sqlx::types::Json<Value>,
    pub artifact_digest: String,
    pub created_by: String,
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
}

pub(crate) const ARTIFACT_COLUMNS: &str = "id, mission_id, mission_version_id, version_payload_sha256, \
     metadata, metadata_sha256, catalog_sha256, modpack_id, modpack_version, compiler_version, \
     schema_version, terrain, document_sha256, document_bytes, diagnostics, artifact_digest, \
     created_by, created_at";

/// Cap on the findings a rejection echoes. Every constraint in `mission.schema.json` under
/// `slots[]` is per slot, so one systematic defect on a large mission yields one finding per
/// slot; the full count always ships and the full list reaches the log.
pub(crate) const MAX_REPORTED_FINDINGS: usize = 20;

/// The distinct findings in first-reported order, and how many there are. The schema reaches
/// `slots[]` through two paths, so every slot finding arrives twice.
pub(crate) fn reported_findings(findings: Vec<String>) -> (usize, Vec<String>) {
    let mut seen = std::collections::HashSet::with_capacity(findings.len());
    let unique: Vec<String> = findings
        .into_iter()
        .filter(|finding| seen.insert(finding.clone()))
        .collect();
    (unique.len(), unique)
}

fn rejection(code: &str, message: String, findings: Vec<String>) -> ApiError {
    let (count, unique) = reported_findings(findings);
    if !unique.is_empty() {
        tracing::warn!(code, findings = count, detail = %unique.join("; "), "artifact compilation refused");
    }
    let shown: Vec<&String> = unique.iter().take(MAX_REPORTED_FINDINGS).collect();
    ApiError::with_details(
        axum::http::StatusCode::UNPROCESSABLE_ENTITY,
        message,
        serde_json::json!({
            "code": code,
            "schema": "mission.schema.json",
            "finding_count": count,
            "findings": shown,
        }),
    )
}

/// Compile `version` of `mission` against the current catalog into an artifact authored by
/// `actor`. Identical inputs return the artifact they already produced. Authored gameplay data
/// the document cannot carry is refused with every authored path it concerns, so an artifact
/// always plays as authored.
pub async fn compile_artifact(
    connection: &mut PgConnection,
    mission: &Mission,
    version: Uuid,
    actor: &str,
) -> Result<MissionArtifact, ApiError> {
    let payload: String = sqlx::query_scalar(
        "SELECT json_payload::text FROM mission_versions WHERE id = $1 AND mission_id = $2",
    )
    .bind(version)
    .bind(mission.id)
    .fetch_optional(&mut *connection)
    .await?
    .ok_or_else(|| ApiError::not_found("mission version not found"))?;
    let snapshot = load_catalog_snapshot(connection).await?;
    let document = match flatten_to_mod_document_with_catalog(
        mission,
        payload.as_bytes(),
        &snapshot.catalog,
    ) {
        Ok(document) => document,
        Err(CompileError::NoSlots) => {
            return Err(rejection(
                "NO_PLACED_SLOTS",
                "the version has no placed slots".into(),
                Vec::new(),
            ));
        }
        Err(CompileError::Parse(detail)) => {
            return Err(rejection(
                "UNCOMPILABLE_VERSION",
                "the version does not compile".into(),
                vec![detail],
            ));
        }
    };
    let authored: Value = serde_json::from_str(&payload).unwrap_or(Value::Null);
    let unsupported = unsupported_authored_data(&document, &authored);
    if !unsupported.is_empty() {
        return Err(rejection(
            "UNSUPPORTED_AUTHORED_DATA",
            "the version authors gameplay data the mission document cannot carry".into(),
            unsupported,
        ));
    }
    let diagnostics = serde_json::to_value(
        document
            .diagnostics
            .iter()
            .map(|finding| {
                serde_json::json!({
                    "rule_id": finding.rule_id,
                    "severity": finding.severity.as_str(),
                    "subject": finding.subject,
                    "subject_id": finding.subject_id,
                    "message": finding.message,
                })
            })
            .collect::<Vec<_>>(),
    )
    .unwrap_or_else(|_| Value::Array(Vec::new()));
    let bytes = serde_json::to_vec(&document)
        .map_err(|_| ApiError::internal("the compiled document could not be serialized"))?;
    let findings = validate_mission_document(&bytes)
        .map_err(|_| ApiError::internal("mission validation unavailable"))?;
    if !findings.is_empty() {
        return Err(rejection(
            "DOCUMENT_CONTRACT_VIOLATION",
            "the compiled document violates the mission contract".into(),
            findings,
        ));
    }
    let metadata = compiled_metadata(mission);
    let metadata_sha256 = sha256_hex(canonical_json(&metadata).as_bytes());
    let version_payload_sha256 = sha256_hex(payload.as_bytes());
    let document_sha256 = sha256_hex(&bytes);
    let (modpack_id, modpack_version) = snapshot.modpack.clone().unzip();
    let digest_input = canonical_json(&serde_json::json!({
        "compiler_version": COMPILER_PACKAGE_VERSION,
        "schema_version": document.schema_version,
        "mission_version_id": version,
        "version_payload_sha256": version_payload_sha256,
        "metadata_sha256": metadata_sha256,
        "catalog_sha256": snapshot.sha256,
        "modpack_id": modpack_id,
        "modpack_version": modpack_version,
        "document_sha256": document_sha256,
    }));
    let artifact_digest = sha256_hex(digest_input.as_bytes());
    let inserted: Option<MissionArtifact> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "INSERT INTO mission_artifacts (mission_id, mission_version_id, version_payload_sha256,
             metadata, metadata_sha256, catalog_sha256, modpack_id, modpack_version, compiler_version,
             schema_version, terrain, document, document_sha256, document_bytes, diagnostics,
             artifact_digest, created_by)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
         ON CONFLICT (artifact_digest) DO NOTHING
         RETURNING {ARTIFACT_COLUMNS}"
    )))
    .bind(mission.id)
    .bind(version)
    .bind(&version_payload_sha256)
    .bind(sqlx::types::Json(&metadata))
    .bind(&metadata_sha256)
    .bind(&snapshot.sha256)
    .bind(modpack_id)
    .bind(&modpack_version)
    .bind(COMPILER_PACKAGE_VERSION)
    .bind(&document.schema_version)
    .bind(&document.meta.terrain)
    .bind(&bytes)
    .bind(&document_sha256)
    .bind(bytes.len() as i32)
    .bind(sqlx::types::Json(&diagnostics))
    .bind(&artifact_digest)
    .bind(actor)
    .fetch_optional(&mut *connection)
    .await?;
    match inserted {
        Some(artifact) => {
            for finding in &document.diagnostics {
                tracing::warn!(
                    artifact = %artifact.id,
                    rule = %finding.rule_id,
                    severity = %finding.severity.as_str(),
                    subject = %finding.subject,
                    subject_id = %finding.subject_id.as_deref().unwrap_or(""),
                    detail = %finding.message,
                    "compile diagnostic",
                );
            }
            Ok(artifact)
        }
        None => load_artifact_by_digest(connection, &artifact_digest).await,
    }
}

async fn load_artifact_by_digest(
    connection: &mut PgConnection,
    digest: &str,
) -> Result<MissionArtifact, ApiError> {
    Ok(sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {ARTIFACT_COLUMNS} FROM mission_artifacts WHERE artifact_digest = $1"
    )))
    .bind(digest)
    .fetch_one(connection)
    .await?)
}

pub async fn load_artifact(
    connection: &mut PgConnection,
    mission: Uuid,
    artifact: Uuid,
) -> Result<MissionArtifact, ApiError> {
    sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT {ARTIFACT_COLUMNS} FROM mission_artifacts WHERE id = $1 AND mission_id = $2"
    )))
    .bind(artifact)
    .bind(mission)
    .fetch_optional(connection)
    .await?
    .ok_or_else(|| ApiError::not_found("artifact not found"))
}

/// An artifact's exact compiled bytes, their SHA-256, and the compile's findings.
pub struct ArtifactDocument {
    pub bytes: Vec<u8>,
    pub sha256: String,
    pub diagnostics: Value,
}

pub async fn load_artifact_document(
    connection: &mut PgConnection,
    artifact: Uuid,
) -> Result<ArtifactDocument, ApiError> {
    let row: Option<(Vec<u8>, String, sqlx::types::Json<Value>)> = sqlx::query_as(
        "SELECT document, document_sha256, diagnostics FROM mission_artifacts WHERE id = $1",
    )
    .bind(artifact)
    .fetch_optional(connection)
    .await?;
    let (bytes, sha256, diagnostics) =
        row.ok_or_else(|| ApiError::not_found("artifact not found"))?;
    Ok(ArtifactDocument {
        bytes,
        sha256,
        diagnostics: diagnostics.0,
    })
}

#[cfg(test)]
#[path = "tests/artifact_store.rs"]
mod tests;
