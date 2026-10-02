//! The review of a mission's immutable artifacts: the submission that opens a review, the history
//! and thread, one artifact and its read-only workspace, and the two decisions.
//!
//! **Role:** the submission, review history, review comment, artifact, review workspace and
//! decision paths, and one call per route.
//! **Position:** called by the approvals drawer, the mission hub's review record and submit action,
//! and the editor's review workspace.
//! **Signals & state:** none.
//! **Invariants:** a decision names the artifact of the pending review, and the backend refuses one
//! whose artifact is no longer under review (`409 REVIEWED_ARTIFACT_CHANGED`, naming the artifact
//! that is) or when nothing is under review (`409 NO_PENDING_REVIEW`). A submission that does not
//! compile answers 422 with its reason in `details.code` and the authored paths it concerns in
//! `details.findings`. Every change answers an [`ApiRefusal`](crate::v2::core::api::client::ApiRefusal)
//! for that reason; the reads answer the plain failure pair.

use super::encode_path_segment as segment;

/// `POST /missions/:id/submit`: compile the current version into an artifact and open its review.
pub fn mission_submission_path(mission_id: &str) -> String {
    format!("/missions/{}/submit", segment(mission_id))
}

/// `GET /missions/:id/reviews`: every review, newest first, and the thread.
pub fn mission_reviews_path(mission_id: &str) -> String {
    format!("/missions/{}/reviews", segment(mission_id))
}

/// `POST /missions/:id/review-comments`.
pub fn review_comments_path(mission_id: &str) -> String {
    format!("/missions/{}/review-comments", segment(mission_id))
}

/// `GET /missions/:id/artifacts/:artifactId`: an artifact's provenance.
pub fn mission_artifact_path(mission_id: &str, artifact_id: &str) -> String {
    format!(
        "/missions/{}/artifacts/{}",
        segment(mission_id),
        segment(artifact_id)
    )
}

/// `GET /missions/:id/artifacts/:artifactId/workspace`: the artifact and the version it compiled
/// from.
pub fn review_workspace_path(mission_id: &str, artifact_id: &str) -> String {
    format!(
        "{}/workspace",
        mission_artifact_path(mission_id, artifact_id)
    )
}

/// `POST /approvals/:id/approve`.
pub fn approval_path(mission_id: &str) -> String {
    format!("/approvals/{}/approve", segment(mission_id))
}

/// `POST /approvals/:id/reject`.
pub fn rejection_path(mission_id: &str) -> String {
    format!("/approvals/{}/reject", segment(mission_id))
}

#[cfg(target_arch = "wasm32")]
pub use calls::*;

/// The browser-only calls, one per route.
#[cfg(target_arch = "wasm32")]
mod calls {
    use super::*;
    use crate::v2::core::api::client::{api_get, api_post_keeping_refusal, ApiErr, ApiRefusal};
    use crate::v2::core::api::dto::{
        ApprovalDecision, MissionArtifact, MissionReviewHistory, MissionRow, RejectionDecision,
        ReviewComment, ReviewCommentRequest, ReviewWorkspace,
    };
    use crate::v2::core::api::endpoints::json_body;
    use crate::v2::core::auth::AuthStore;

    /// Submit the mission's current version for review.
    pub async fn submit_mission_for_review(
        store: AuthStore,
        mission_id: &str,
    ) -> Result<MissionRow, ApiRefusal> {
        let path = mission_submission_path(mission_id);
        api_post_keeping_refusal(store, &path, serde_json::json!({})).await
    }

    /// Every review of the mission and its thread.
    pub async fn load_review_history(
        store: AuthStore,
        mission_id: &str,
    ) -> Result<MissionReviewHistory, ApiErr> {
        api_get(store, &mission_reviews_path(mission_id)).await
    }

    /// Add a comment to the mission's review thread.
    pub async fn post_review_comment(
        store: AuthStore,
        mission_id: &str,
        comment: &ReviewCommentRequest,
    ) -> Result<ReviewComment, ApiRefusal> {
        let path = review_comments_path(mission_id);
        api_post_keeping_refusal(store, &path, json_body(comment)?).await
    }

    /// One artifact's provenance.
    pub async fn load_mission_artifact(
        store: AuthStore,
        mission_id: &str,
        artifact_id: &str,
    ) -> Result<MissionArtifact, ApiErr> {
        api_get(store, &mission_artifact_path(mission_id, artifact_id)).await
    }

    /// The artifact and exactly the version it compiled from.
    pub async fn load_review_workspace(
        store: AuthStore,
        mission_id: &str,
        artifact_id: &str,
    ) -> Result<ReviewWorkspace, ApiErr> {
        api_get(store, &review_workspace_path(mission_id, artifact_id)).await
    }

    /// Approve the artifact under review, with any conditions the decision carries.
    pub async fn approve_review(
        store: AuthStore,
        mission_id: &str,
        decision: &ApprovalDecision,
    ) -> Result<MissionRow, ApiRefusal> {
        let path = approval_path(mission_id);
        api_post_keeping_refusal(store, &path, json_body(decision)?).await
    }

    /// Reject the artifact under review with the reason its author is given.
    pub async fn reject_review(
        store: AuthStore,
        mission_id: &str,
        decision: &RejectionDecision,
    ) -> Result<MissionRow, ApiRefusal> {
        let path = rejection_path(mission_id);
        api_post_keeping_refusal(store, &path, json_body(decision)?).await
    }
}
