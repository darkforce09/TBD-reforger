//! Mission reviews: the review history and its thread, the decisions a reviewer sends, and the
//! immutable artifact a review decides.
//!
//! **Role:** the review history of `GET /missions/:id/reviews`, one review and one thread comment,
//! the comment and decision bodies, an artifact's provenance with the findings its compile
//! reported, and the review workspace — the artifact together with exactly the version it compiled
//! from.
//! **Position:** deserialised straight from the backend's JSON and handed to the approvals drawer,
//! the mission hub's review record and the editor's review workspace; re-serialised unchanged by
//! the round-trip tests.
//! **Signals & state:** none — these are plain data.
//! **Invariants:** review states, comment kinds and finding severities are carried as the strings
//! the backend sends, so a value added there still lists here instead of failing the whole read.
//! A decision always names the artifact it decides: the backend refuses one whose artifact is no
//! longer the one under review. A finding's `subject_id` is sent as an explicit `null` when the
//! finding concerns no single entity, so it is never skipped when serialising.

use serde::{Deserialize, Serialize};

use super::missions::MissionVersion;

/// One review: the artifact it decides, the version that artifact compiled from, and its outcome.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MissionReview {
    pub id: String,
    pub mission_id: String,
    pub artifact_id: String,
    /// The artifact's SHA-256 digest, lowercase hex.
    pub artifact_digest: String,
    pub mission_version_id: String,
    /// The version number the artifact compiled from.
    pub semver: String,
    pub submitted_by: String,
    pub submitted_at: String,
    /// `pending`, `approved`, `approved_with_conditions`, `rejected` or `superseded` — a later
    /// submission replaced a pending review before anyone decided it.
    pub state: String,
    /// Absent until the review is decided; a superseded review is never decided by anyone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decided_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decided_at: Option<String>,
}

/// One comment of a mission's review thread, with the review, version and artifact it concerns.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReviewComment {
    pub id: String,
    pub mission_id: String,
    /// The review a decision comment belongs to; absent on a thread comment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mission_version_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<String>,
    pub author_id: String,
    /// The author's display name; empty when their account row is gone.
    pub author_name: String,
    /// `comment`, `rejection` (a rejection's reason) or `approval_conditions` (the conditions an
    /// approval carries).
    pub kind: String,
    pub body: String,
    pub created_at: String,
}

/// `GET /missions/:id/reviews`: every review of the mission, newest first, and the thread in the
/// order it was written.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MissionReviewHistory {
    pub reviews: Vec<MissionReview>,
    pub comments: Vec<ReviewComment>,
}

/// `POST /missions/:id/review-comments` body: the comment, and the artifact it is about when it is
/// about one.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReviewCommentRequest {
    /// Trimmed and not blank; at most 8000 bytes of UTF-8.
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<String>,
}

/// `POST /approvals/:id/approve` body: the artifact under review, and any conditions the approval
/// carries. Non-blank conditions make the decision `approved_with_conditions`.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApprovalDecision {
    pub artifact_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conditions: Option<String>,
}

/// `POST /approvals/:id/reject` body: the artifact under review, and the reason the author is
/// given, which becomes the review's rejection comment.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RejectionDecision {
    pub artifact_id: String,
    pub reason: String,
}

/// One finding the compiler reported while producing an artifact.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArtifactDiagnostic {
    pub rule_id: String,
    /// `error`, `warning` or `info`.
    pub severity: String,
    /// The kind of thing the finding is about, such as a slot or a vehicle.
    pub subject: String,
    /// The entity the finding names, or null when it names none. Always sent.
    pub subject_id: Option<String>,
    pub message: String,
}

/// The mission fields the compiler read, in the canonical form the metadata digest covers.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    pub id: String,
    pub title: String,
    /// The author's account id, as the compiled document carries it.
    pub author: String,
    pub terrain: String,
    /// Empty for a built-in terrain.
    pub custom_terrain_name: String,
    pub game_mode: String,
    pub max_players: i64,
    pub time_of_day: String,
    pub weather: String,
}

impl ArtifactMetadata {
    /// The row fields the shared compiler reads, exactly as this artifact's compile read them —
    /// so a compile of the reviewed version in the browser runs over the same inputs.
    pub fn compiled_meta(&self) -> website_map_engine::data::scenario::flatten::MissionMeta {
        website_map_engine::data::scenario::flatten::MissionMeta {
            id: self.id.clone(),
            title: self.title.clone(),
            author: self.author.clone(),
            terrain: self.terrain.clone(),
            custom_terrain_name: self.custom_terrain_name.clone(),
            max_players: self.max_players,
            time_of_day: self.time_of_day.clone(),
            weather_preset: self.weather.clone(),
        }
    }
}

/// `GET /missions/:id/artifacts/:artifactId`: an immutable artifact's provenance. Its exact bytes
/// are `GET …/artifacts/:artifactId/document`, served with their SHA-256 as the entity tag.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MissionArtifact {
    pub id: String,
    pub mission_id: String,
    pub mission_version_id: String,
    /// SHA-256 of the authored version payload the artifact compiled from.
    pub version_payload_sha256: String,
    pub metadata: ArtifactMetadata,
    pub metadata_sha256: String,
    /// SHA-256 of the catalog snapshot the compile read.
    pub catalog_sha256: String,
    /// The modpack current when the artifact compiled; absent when there was none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modpack_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modpack_version: Option<String>,
    pub compiler_version: String,
    pub schema_version: String,
    /// The terrain key of the compiled document.
    pub terrain: String,
    /// SHA-256 of the compiled document's bytes.
    pub document_sha256: String,
    /// The compiled document's size in bytes.
    pub document_bytes: i64,
    pub diagnostics: Vec<ArtifactDiagnostic>,
    /// SHA-256 over every input and output digest above: the artifact's identity.
    pub artifact_digest: String,
    pub created_by: String,
    pub created_at: String,
}

/// `GET /missions/:id/artifacts/:artifactId/workspace`: the artifact and exactly the version it
/// compiled from, verified by the backend against the payload digest the artifact recorded.
#[allow(dead_code)]
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewWorkspace {
    pub artifact: MissionArtifact,
    pub version: MissionVersion,
}
