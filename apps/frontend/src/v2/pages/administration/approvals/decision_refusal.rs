//! What a reviewer is told when a decision is refused, and whether the queue must be read again.
//!
//! **Role:** reads a refused approval or rejection into one of the reasons the backend names and
//! words each one.
//! **Position:** read by the decision form after a decision fails.
//! **Signals & state:** none; pure over the refusal.
//! **Invariants:** `409 NO_PENDING_REVIEW` (nothing is under review any more),
//! `409 REVIEWED_ARTIFACT_CHANGED` (a resubmission replaced the artifact the reviewer was looking
//! at; `details.artifact_id` names the one under review now) and a 409 without a code (the mission
//! is no longer awaiting approval) all mean the queue on screen is stale, so each asks for it to be
//! read again. Any other refusal falls back to the backend's own sentence.

use crate::v2::core::api::client::ApiRefusal;

/// Why a decision was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum DecisionRefusal {
    /// Nothing of the mission is under review: it was decided, or a submission is replacing it.
    NoPendingReview,
    /// The artifact under review is no longer the one decided; this names the one that is.
    ReviewedArtifactChanged { artifact_id: Option<String> },
    /// The mission is no longer awaiting approval, carrying the backend's sentence.
    NotPendingApproval(String),
    /// Any other refusal, carrying the sentence to show.
    Other(String),
}

impl DecisionRefusal {
    /// Read a refused decision; `fallback` is shown when the backend sent no sentence.
    pub(super) fn from_refusal(refusal: &ApiRefusal, fallback: &str) -> Self {
        match refusal.code() {
            Some("NO_PENDING_REVIEW") => Self::NoPendingReview,
            Some("REVIEWED_ARTIFACT_CHANGED") => Self::ReviewedArtifactChanged {
                artifact_id: refusal.detail("artifact_id").map(str::to_string),
            },
            None if refusal.status == 409 => {
                Self::NotPendingApproval(refusal.message_or("The mission is not pending approval"))
            }
            _ => Self::Other(refusal.message_or(fallback)),
        }
    }

    /// Whether the queue on screen is stale and must be read again.
    pub(super) fn reloads_queue(&self) -> bool {
        !matches!(self, Self::Other(_))
    }

    /// The sentence the reviewer is shown.
    pub(super) fn sentence(&self) -> String {
        match self {
            Self::NoPendingReview => "Nothing of this mission is under review any more: it was \
                                      decided, or its author is resubmitting it. The queue has \
                                      been read again."
                .to_string(),
            Self::ReviewedArtifactChanged { artifact_id } => {
                let now = artifact_id
                    .as_deref()
                    .map(|id| format!(" Artifact {id} is under review now."))
                    .unwrap_or_default();
                format!(
                    "The author resubmitted while you were reviewing, so your decision named an \
                     artifact that is no longer under review. Nothing was decided.{now} The queue \
                     has been read again — review the new artifact before deciding."
                )
            }
            Self::NotPendingApproval(sentence) => {
                format!("{sentence}. The queue has been read again.")
            }
            Self::Other(sentence) => sentence.clone(),
        }
    }
}
