//! Mission reviews of immutable artifacts, and the per-mission review thread.

use api_identifiers::{
    DiscordUserId, MissionArtifactId, MissionId, MissionReviewCommentId, MissionReviewId,
    MissionVersionId,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use fleet_wire_contract::rfc3339_timestamps::{rfc3339_utc, rfc3339_utc_opt};

/// One review: the artifact it decides and its outcome.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct MissionReview {
    /// The review's id.
    pub id: MissionReviewId,
    /// The reviewed mission.
    pub mission_id: MissionId,
    /// The immutable artifact the review decides.
    pub artifact_id: MissionArtifactId,
    /// The artifact's identity digest (lowercase hex SHA-256 over its compile inputs).
    pub artifact_digest: String,
    /// The mission version the artifact was compiled from.
    pub mission_version_id: MissionVersionId,
    /// That version's semantic version string.
    pub semver: String,
    /// Discord id of the user who submitted the mission for review.
    pub submitted_by: String,
    /// When the review was opened (RFC 3339, UTC).
    #[serde(with = "rfc3339_utc")]
    pub submitted_at: DateTime<Utc>,
    /// Review state: `pending`, `approved`, `approved_with_conditions`, `rejected` or `superseded`.
    pub state: String,
    /// Discord id of the reviewer who decided it; `None` while pending or when superseded.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decided_by: Option<String>,
    /// When the review left `pending` (RFC 3339, UTC); `None` while pending.
    #[serde(with = "rfc3339_utc_opt", skip_serializing_if = "Option::is_none")]
    pub decided_at: Option<DateTime<Utc>>,
}

/// One comment of the review thread, with the version and artifact it concerns.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ReviewComment {
    /// The comment's id.
    pub id: MissionReviewCommentId,
    /// The mission whose thread the comment belongs to.
    pub mission_id: MissionId,
    /// The review the comment belongs to; always set for rejection and approval-condition comments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_id: Option<MissionReviewId>,
    /// The mission version the comment concerns, when it names one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mission_version_id: Option<MissionVersionId>,
    /// The artifact the comment concerns, when it names one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<MissionArtifactId>,
    /// Discord id of the comment's author.
    pub author_id: DiscordUserId,
    /// The author's display name.
    pub author_name: String,
    /// Comment kind: `comment`, `rejection` or `approval_conditions`.
    pub kind: String,
    /// The comment text (trimmed, at most 8000 bytes).
    pub body: String,
    /// When the comment was posted (RFC 3339, UTC).
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
}

/// `POST /approvals/{id}/approve` body: the artifact the reviewer decided and any conditions.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalDecision {
    /// The artifact the reviewer approves; must be the pending review's artifact.
    pub artifact_id: MissionArtifactId,
    /// Conditions attached to the approval; `Some` approves with conditions.
    #[serde(default)]
    pub conditions: Option<String>,
}

/// `POST /approvals/{id}/reject` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RejectionDecision {
    /// The artifact the reviewer rejects; must be the pending review's artifact.
    pub artifact_id: MissionArtifactId,
    /// Why the mission is rejected; stored as the review's rejection comment.
    pub reason: String,
}

/// `POST /missions/{id}/review-comments` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewCommentRequest {
    /// The comment text.
    pub body: String,
    /// The artifact the comment concerns; `None` for a comment on the mission as a whole.
    #[serde(default)]
    pub artifact_id: Option<MissionArtifactId>,
}

/// `GET /missions/{id}/reviews` response.
#[derive(Debug, Serialize)]
pub struct MissionReviewHistory {
    /// The mission's reviews.
    pub reviews: Vec<MissionReview>,
    /// The mission's review thread.
    pub comments: Vec<ReviewComment>,
}
