//! Missions through the platform's publication flow: create, submit (the platform compiles the
//! current version into an immutable artifact and opens a review), approve, and read the
//! artifact's exact document bytes.

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::api_client::{ApiClient, text_at};
use super::http_exchange::encode_component;

/// The approved artifact a deployment can run, with what a tool needs to stage it.
#[derive(Debug, Clone, PartialEq)]
pub struct ApprovedArtifact {
    pub mission_id: String,
    pub artifact_id: String,
    pub terrain: String,
    pub modpack_id: Option<String>,
}

/// An artifact's document: the exact bytes the platform serves, and their SHA-256 in hex.
#[derive(Debug, Clone, PartialEq)]
pub struct ArtifactDocument {
    pub bytes: Vec<u8>,
    pub sha256: String,
}

/// `POST /api/v1/missions` with `body` (an inline `payload` becomes version 1); the new id.
pub fn create_mission(client: &ApiClient<'_>, body: &Value) -> Result<String> {
    let created = client.expect_json("POST", "/api/v1/missions", Some(body), 201)?;
    text_at(&created, "/id", "POST /api/v1/missions")
}

/// `GET /api/v1/missions/{id}`.
pub fn mission(client: &ApiClient<'_>, mission_id: &str) -> Result<Value> {
    client.expect_json(
        "GET",
        &format!("/api/v1/missions/{}", encode_component(mission_id)),
        None,
        200,
    )
}

/// Submit the mission's current version; the artifact the opened review decides.
pub fn submit_mission(client: &ApiClient<'_>, mission_id: &str) -> Result<String> {
    client.expect(
        "POST",
        &format!("/api/v1/missions/{}/submit", encode_component(mission_id)),
        None,
        200,
    )?;
    pending_review_artifact(client, mission_id)
}

/// The artifact of the mission's pending review.
pub fn pending_review_artifact(client: &ApiClient<'_>, mission_id: &str) -> Result<String> {
    let history = client.expect_json(
        "GET",
        &format!("/api/v1/missions/{}/reviews", encode_component(mission_id)),
        None,
        200,
    )?;
    history["reviews"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|review| review["state"] == "pending")
        .and_then(|review| review["artifact_id"].as_str())
        .map(str::to_string)
        .with_context(|| format!("mission {mission_id} has no pending review: {history}"))
}

/// Approve the pending review's artifact.
pub fn approve(client: &ApiClient<'_>, mission_id: &str, artifact_id: &str) -> Result<()> {
    client.expect(
        "POST",
        &format!("/api/v1/approvals/{}/approve", encode_component(mission_id)),
        Some(&json!({ "artifact_id": artifact_id })),
        200,
    )?;
    Ok(())
}

/// The mission's approved artifact, submitting and approving the current version first when
/// the mission has none: a live mission keeps the artifact its approval decided, a mission under
/// review has its pending artifact approved, and any other mission is submitted and approved.
/// Approving needs an administrator's token.
pub fn approved_artifact(client: &ApiClient<'_>, mission_id: &str) -> Result<ApprovedArtifact> {
    let current = mission(client, mission_id)?;
    let status = current["status"].as_str().unwrap_or_default();
    let approved = current["approved_artifact_id"].as_str().map(str::to_string);
    let artifact_id = match (status, approved) {
        ("live", Some(artifact)) => artifact,
        ("pending_approval", _) => {
            let artifact = pending_review_artifact(client, mission_id)?;
            approve(client, mission_id, &artifact)?;
            artifact
        }
        ("archived", _) => bail!("mission {mission_id} is archived; restore it before deploying"),
        _ => {
            let artifact = submit_mission(client, mission_id)?;
            approve(client, mission_id, &artifact)?;
            artifact
        }
    };
    let provenance = artifact_provenance(client, mission_id, &artifact_id)?;
    Ok(ApprovedArtifact {
        mission_id: mission_id.to_string(),
        terrain: text_at(&provenance, "/terrain", "artifact provenance")?,
        modpack_id: provenance["modpack_id"].as_str().map(str::to_string),
        artifact_id,
    })
}

/// `GET /api/v1/missions/{id}/artifacts/{artifact}`: the artifact's provenance.
pub fn artifact_provenance(
    client: &ApiClient<'_>,
    mission_id: &str,
    artifact_id: &str,
) -> Result<Value> {
    client.expect_json(
        "GET",
        &format!(
            "/api/v1/missions/{}/artifacts/{}",
            encode_component(mission_id),
            encode_component(artifact_id)
        ),
        None,
        200,
    )
}

/// The artifact's document. Its SHA-256 must equal the entity tag the platform sends with it.
pub fn artifact_document(
    client: &ApiClient<'_>,
    mission_id: &str,
    artifact_id: &str,
) -> Result<ArtifactDocument> {
    let answer = client.expect(
        "GET",
        &format!(
            "/api/v1/missions/{}/artifacts/{}/document",
            encode_component(mission_id),
            encode_component(artifact_id)
        ),
        None,
        200,
    )?;
    let sha256 = sha256_hex(&answer.body);
    let tag = answer
        .header("etag")
        .map(|tag| tag.trim_matches('"').to_string())
        .context("the artifact document came without an entity tag")?;
    if tag != sha256 {
        bail!("artifact {artifact_id}: the document hashes to {sha256}, its entity tag is {tag}");
    }
    Ok(ArtifactDocument {
        bytes: answer.body,
        sha256,
    })
}

/// The ids of the caller's own missions with exactly this title.
pub fn own_missions_titled(client: &ApiClient<'_>, title: &str) -> Result<Vec<String>> {
    let page = client.expect_json(
        "GET",
        &format!(
            "/api/v1/missions?scope=mine&limit=100&q={}",
            encode_component(title)
        ),
        None,
        200,
    )?;
    Ok(page["data"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|row| row["title"] == title)
        .filter_map(|row| row["id"].as_str().map(str::to_string))
        .collect())
}

/// `DELETE /api/v1/missions/{id}` (a soft delete).
pub fn delete_mission(client: &ApiClient<'_>, mission_id: &str) -> Result<()> {
    let path = format!("/api/v1/missions/{}", encode_component(mission_id));
    let answer = client.call("DELETE", &path, None)?;
    if !matches!(answer.status, 200 | 204) {
        bail!(
            "DELETE {path} answered {}: {}",
            answer.status,
            answer.excerpt()
        );
    }
    Ok(())
}

/// Lowercase hex SHA-256.
pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
