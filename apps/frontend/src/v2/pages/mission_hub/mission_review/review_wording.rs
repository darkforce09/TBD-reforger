//! A mission's review record in words: states, decisions, comments, artifact provenance and the
//! review workspace link.
//!
//! **Role:** names a review state and the tone its badge takes, a comment kind, who submitted and
//! decided a review and when, an artifact's digest and document size, a compile finding, and the
//! address of the read-only review workspace; decides whether a mission's history is worth reading
//! before anyone asks for it.
//! **Position:** read by the review record, the approvals drawer and the approvals queue.
//! **Signals & state:** none; pure over its arguments.
//! **Invariants:** an instant is shown as its UTC line, so every sentence here is testable without a
//! browser clock. A state, kind or severity this build does not know is shown as the backend spells
//! it, never as a known one. An account is named as "you" for the viewer and by its id otherwise —
//! the id is all a review carries.

use crate::v2::core::api::dto::{ArtifactDiagnostic, MissionReview, MissionReviewHistory};
use crate::v2::core::api::endpoints::encode_path_segment;
use crate::v2::core::utils::utc_timestamp::utc_label;

/// How many hex digits of a digest identify it on screen.
pub(crate) const SHORT_DIGEST_LEN: usize = 12;

/// The first twelve hex digits of a digest — enough to tell artifacts apart at a glance.
pub(crate) fn short_digest(digest: &str) -> String {
    digest.chars().take(SHORT_DIGEST_LEN).collect()
}

/// A review state as a reviewer and an author read it.
pub(crate) fn review_state_label(state: &str) -> String {
    match state {
        "pending" => "Pending review".to_string(),
        "approved" => "Approved".to_string(),
        "approved_with_conditions" => "Approved with conditions".to_string(),
        "rejected" => "Rejected".to_string(),
        "superseded" => "Superseded".to_string(),
        other => other.replace('_', " "),
    }
}

/// The badge variant a review state is shown in.
pub(crate) fn review_state_tone(state: &str) -> &'static str {
    match state {
        "pending" => "warning",
        "approved" => "success",
        "approved_with_conditions" => "tertiary",
        "rejected" => "error",
        _ => "neutral",
    }
}

/// A comment kind as the thread labels it.
pub(crate) fn comment_kind_label(kind: &str) -> String {
    match kind {
        "comment" => "Comment".to_string(),
        "rejection" => "Rejection reason".to_string(),
        "approval_conditions" => "Approval conditions".to_string(),
        other => other.replace('_', " "),
    }
}

/// An account as shown: "you" for the viewer's own.
pub(crate) fn account_label(account: &str, me: Option<&str>) -> String {
    if me == Some(account) {
        "you".to_string()
    } else {
        account.to_string()
    }
}

/// Which version and artifact a review decides: `v0.4.0 · artifact bcfb3b1c4109`.
pub(crate) fn reviewed_artifact_line(semver: &str, artifact_digest: &str) -> String {
    format!("v{semver} · artifact {}", short_digest(artifact_digest))
}

/// Who submitted the review, and when.
pub(crate) fn submission_line(review: &MissionReview, me: Option<&str>) -> String {
    format!(
        "Submitted by {}, {}",
        account_label(&review.submitted_by, me),
        utc_label(&review.submitted_at)
    )
}

/// How the review ended, or that it has not.
pub(crate) fn decision_line(review: &MissionReview, me: Option<&str>) -> String {
    match (
        review.state.as_str(),
        &review.decided_by,
        &review.decided_at,
    ) {
        ("pending", _, _) => "Awaiting a reviewer's decision".to_string(),
        ("superseded", _, at) => match at {
            Some(at) => format!("Replaced by a later submission, {}", utc_label(at)),
            None => "Replaced by a later submission".to_string(),
        },
        (state, Some(by), Some(at)) => format!(
            "{} by {}, {}",
            review_state_label(state),
            account_label(by, me),
            utc_label(at)
        ),
        (state, _, _) => review_state_label(state),
    }
}

/// The decision comment — the conditions or the rejection reason — a review carries.
pub(crate) fn decision_comment<'a>(
    history: &'a MissionReviewHistory,
    review_id: &str,
) -> Option<(&'a str, &'a str)> {
    history
        .comments
        .iter()
        .find(|c| c.review_id.as_deref() == Some(review_id) && c.kind != "comment")
        .map(|c| (c.kind.as_str(), c.body.as_str()))
}

/// The review still waiting on a decision, when there is one.
pub(crate) fn pending_review(history: &MissionReviewHistory) -> Option<&MissionReview> {
    history.reviews.iter().find(|r| r.state == "pending")
}

/// Whether a mission's row shows any sign of a review — submitted, decided or approved — so its
/// history is read without being asked for. A draft that never left its author's hands has none.
pub(crate) fn has_review_trace(
    status: &str,
    reviewed_at: Option<&str>,
    approved_artifact_id: Option<&str>,
) -> bool {
    matches!(status, "pending_approval" | "live" | "rejected")
        || reviewed_at.is_some()
        || approved_artifact_id.is_some()
}

/// A document size in the unit a reader compares, with the exact byte count beside it.
pub(crate) fn document_size_label(bytes: i64) -> String {
    const KIB: f64 = 1024.0;
    let exact = format!("{bytes} bytes");
    #[allow(clippy::cast_precision_loss)]
    let size = bytes as f64;
    if size < KIB {
        exact
    } else if size < KIB * KIB {
        format!("{:.1} KiB ({exact})", size / KIB)
    } else {
        format!("{:.1} MiB ({exact})", size / (KIB * KIB))
    }
}

/// The badge variant a finding's severity is shown in.
pub(crate) fn severity_tone(severity: &str) -> &'static str {
    match severity {
        "error" => "error",
        "warning" => "warning",
        _ => "neutral",
    }
}

/// What a finding is about: its subject, and the entity it names when it names one.
pub(crate) fn diagnostic_subject(diagnostic: &ArtifactDiagnostic) -> String {
    match &diagnostic.subject_id {
        Some(id) if !id.is_empty() => format!("{} ({id})", diagnostic.subject),
        _ => diagnostic.subject.clone(),
    }
}

/// The read-only review workspace of one artifact, as the frontend routes it.
pub(crate) fn review_workspace_href(mission_id: &str, artifact_id: &str) -> String {
    format!(
        "/missions/{}/artifacts/{}/workspace",
        encode_path_segment(mission_id),
        encode_path_segment(artifact_id)
    )
}

/// A body of review text trimmed, or what is wrong with it: blank, or longer than the backend's
/// 8000 bytes. `what` names the text in the sentence.
pub(crate) fn validated_review_text(text: &str, what: &str) -> Result<String, String> {
    let trimmed = text.trim();
    match trimmed.len() {
        0 => Err(format!("Write the {what} first")),
        1..=8000 => Ok(trimmed.to_string()),
        n => Err(format!("The {what} is too long: {n} bytes, at most 8000")),
    }
}
