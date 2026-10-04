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

use super::identifiers::{
    DiscordUserId, MissionArtifactId, MissionId, MissionReviewCommentId, MissionReviewId,
    MissionVersionId, ModpackId, ValidationRuleId, ValidationSubjectId,
};
use super::missions::MissionVersion;

/// One review: the artifact it decides, the version that artifact compiled from, and its outcome.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MissionReview {
    /// The review's id.
    pub id: MissionReviewId,
    /// The reviewed mission.
    pub mission_id: MissionId,
    /// The immutable artifact the review decides.
    pub artifact_id: MissionArtifactId,
    /// The artifact's SHA-256 digest, lowercase hex.
    pub artifact_digest: String,
    /// The mission version the artifact was compiled from.
    pub mission_version_id: MissionVersionId,
    /// The version number the artifact compiled from.
    pub semver: String,
    /// Discord id of the user who submitted the mission for review.
    pub submitted_by: String,
    /// When the review was opened (RFC 3339, UTC).
    pub submitted_at: String,
    /// `pending`, `approved`, `approved_with_conditions`, `rejected` or `superseded` — a later
    /// submission replaced a pending review before anyone decided it.
    pub state: String,
    /// Absent until the review is decided; a superseded review is never decided by anyone.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decided_by: Option<String>,
    /// When the review left `pending` (RFC 3339, UTC); `None` while pending.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decided_at: Option<String>,
}

/// One comment of a mission's review thread, with the review, version and artifact it concerns.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReviewComment {
    /// The comment's id.
    pub id: MissionReviewCommentId,
    /// The mission whose thread the comment belongs to.
    pub mission_id: MissionId,
    /// The review a decision comment belongs to; absent on a thread comment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review_id: Option<MissionReviewId>,
    /// The mission version the comment concerns, when it names one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mission_version_id: Option<MissionVersionId>,
    /// The artifact the comment concerns, when it names one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<MissionArtifactId>,
    /// Discord id of the comment's author.
    pub author_id: DiscordUserId,
    /// The author's display name; empty when their account row is gone.
    pub author_name: String,
    /// `comment`, `rejection` (a rejection's reason) or `approval_conditions` (the conditions an
    /// approval carries).
    pub kind: String,
    /// The comment text (trimmed, at most 8000 bytes).
    pub body: String,
    /// When the comment was posted (RFC 3339, UTC).
    pub created_at: String,
}

/// `GET /missions/:id/reviews`: every review of the mission, newest first, and the thread in the
/// order it was written.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MissionReviewHistory {
    /// The mission's reviews.
    pub reviews: Vec<MissionReview>,
    /// The mission's review thread.
    pub comments: Vec<ReviewComment>,
}

/// `POST /missions/:id/review-comments` body: the comment, and the artifact it is about when it is
/// about one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ReviewCommentRequest {
    /// Trimmed and not blank; at most 8000 bytes of UTF-8.
    pub body: String,
    /// The artifact the comment concerns; `None` for a comment on the mission as a whole.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<MissionArtifactId>,
}

/// `POST /approvals/:id/approve` body: the artifact under review, and any conditions the approval
/// carries. Non-blank conditions make the decision `approved_with_conditions`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApprovalDecision {
    /// The artifact the reviewer approves; must be the pending review's artifact.
    pub artifact_id: MissionArtifactId,
    /// Conditions attached to the approval; `Some` approves with conditions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conditions: Option<String>,
}

/// `POST /approvals/:id/reject` body: the artifact under review, and the reason the author is
/// given, which becomes the review's rejection comment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RejectionDecision {
    /// The artifact the reviewer rejects; must be the pending review's artifact.
    pub artifact_id: MissionArtifactId,
    /// Why the mission is rejected; stored as the review's rejection comment.
    pub reason: String,
}

/// One finding the compiler reported while producing an artifact.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArtifactDiagnostic {
    /// The validation rule the finding comes from.
    pub rule_id: ValidationRuleId,
    /// `error`, `warning` or `info`.
    pub severity: String,
    /// The kind of thing the finding is about, such as a slot or a vehicle.
    pub subject: String,
    /// The entity the finding names, or null when it names none. Always sent.
    pub subject_id: Option<ValidationSubjectId>,
    /// The finding's text.
    pub message: String,
}

/// The mission fields the compiler read, in the canonical form the metadata digest covers.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    /// Primary key (uuid).
    pub id: MissionId,
    /// Display title shown in the library.
    pub title: String,
    /// The author's account id, as the compiled document carries it.
    pub author: String,
    /// Terrain the mission plays on.
    pub terrain: String,
    /// Empty for a built-in terrain.
    pub custom_terrain_name: String,
    /// Game mode the mission is played as.
    pub game_mode: String,
    /// Player capacity of the mission.
    pub max_players: i64,
    /// The in-game time of day the mission starts at, as `HH:MM:SS`.
    pub time_of_day: String,
    /// Weather preset.
    pub weather: String,
}

impl ArtifactMetadata {
    /// The row fields the shared compiler reads, exactly as this artifact's compile read them —
    /// so a compile of the reviewed version in the browser runs over the same inputs.
    pub fn compiled_meta(&self) -> mission_compiler::MissionMeta {
        mission_compiler::MissionMeta {
            id: self.id.as_str().into(),
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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MissionArtifact {
    /// Artifact row id.
    pub id: MissionArtifactId,
    /// Mission the artifact belongs to.
    pub mission_id: MissionId,
    /// Mission version the artifact was compiled from.
    pub mission_version_id: MissionVersionId,
    /// SHA-256 of the authored version payload the artifact compiled from.
    pub version_payload_sha256: String,
    /// Compiled mission metadata recorded with the artifact.
    pub metadata: ArtifactMetadata,
    /// SHA-256 hex of the canonical JSON of [`MissionArtifact::metadata`].
    pub metadata_sha256: String,
    /// SHA-256 of the catalog snapshot the compile read.
    pub catalog_sha256: String,
    /// The modpack current when the artifact compiled; absent when there was none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modpack_id: Option<ModpackId>,
    /// Version of [`MissionArtifact::modpack_id`]; absent when no modpack is current.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modpack_version: Option<String>,
    /// Mission compiler package version that produced the document.
    pub compiler_version: String,
    /// Schema version of the compiled mission document.
    pub schema_version: String,
    /// The terrain key of the compiled document.
    pub terrain: String,
    /// SHA-256 of the compiled document's bytes.
    pub document_sha256: String,
    /// The compiled document's size in bytes.
    pub document_bytes: i64,
    /// Compile findings: a JSON array of `rule_id`/`severity`/`subject`/`subject_id`/`message`.
    pub diagnostics: Vec<ArtifactDiagnostic>,
    /// SHA-256 over every input and output digest above: the artifact's identity.
    pub artifact_digest: String,
    /// Discord user id of the member who compiled the artifact.
    pub created_by: String,
    /// When the artifact was stored, serialized as an RFC 3339 UTC timestamp.
    pub created_at: String,
}

/// `GET /missions/:id/artifacts/:artifactId/workspace`: the artifact and exactly the version it
/// compiled from, verified by the backend against the payload digest the artifact recorded.
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub struct ReviewWorkspace {
    /// The artifact under review.
    pub artifact: MissionArtifact,
    /// The version the artifact was compiled from.
    pub version: MissionVersion,
}
