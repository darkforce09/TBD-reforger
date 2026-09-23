//! Mission reviews of immutable artifacts, and the per-mission review thread.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::core::wire_format::{rfc3339_utc, rfc3339_utc_opt};

/// One review: the artifact it decides and its outcome.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct MissionReview {
    pub id: Uuid,
    pub mission_id: Uuid,
    pub artifact_id: Uuid,
    pub artifact_digest: String,
    pub mission_version_id: Uuid,
    pub semver: String,
    pub submitted_by: String,
    #[serde(with = "rfc3339_utc")]
    pub submitted_at: DateTime<Utc>,
    pub state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decided_by: Option<String>,
    #[serde(with = "rfc3339_utc_opt", skip_serializing_if = "Option::is_none")]
    pub decided_at: Option<DateTime<Utc>>,
}

/// One comment of the review thread, with the version and artifact it concerns.
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct ReviewComment {
    pub id: Uuid,
    pub mission_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mission_version_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_id: Option<Uuid>,
    pub author_id: String,
    pub author_name: String,
    pub kind: String,
    pub body: String,
    #[serde(with = "rfc3339_utc")]
    pub created_at: DateTime<Utc>,
}

/// `POST /approvals/{id}/approve` body: the artifact the reviewer decided and any conditions.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalDecision {
    pub artifact_id: Uuid,
    #[serde(default)]
    pub conditions: Option<String>,
}

/// `POST /approvals/{id}/reject` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RejectionDecision {
    pub artifact_id: Uuid,
    pub reason: String,
}

/// `POST /missions/{id}/review-comments` body.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReviewCommentRequest {
    pub body: String,
    #[serde(default)]
    pub artifact_id: Option<Uuid>,
}

/// `GET /missions/{id}/reviews` response.
#[derive(Debug, Serialize)]
pub struct MissionReviewHistory {
    pub reviews: Vec<MissionReview>,
    pub comments: Vec<ReviewComment>,
}
