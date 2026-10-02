//! The approvals queue's words against the captured queue, the decision bodies and refusals, and the
//! drawer's wiring.

use super::decision_refusal::DecisionRefusal;
use super::review_briefing::{enum_label, game_mode_label};
use super::review_decision::{decision_body, DecisionBody, DecisionKind};
use super::submission_queue::{review_line, terrain_label, PREDATES_REVIEWS};
use crate::v2::core::api::client::ApiRefusal;
use crate::v2::core::api::dto::{ApprovalDecision, ApprovalRow, Paginated, RejectionDecision};
use crate::v2::core::test_support::class_r_scrub::{live_code, only_body};
use crate::v2::core::test_support::fixtures::golden;
use serde_json::json;

fn queue() -> Vec<ApprovalRow> {
    serde_json::from_str::<Paginated<ApprovalRow>>(golden!("GET__approvals.json"))
        .unwrap()
        .data
}

/// A row under review names its version, short artifact digest and review submission; a row that
/// predates reviews has no review line, and the queue says why.
#[test]
fn queue_rows_name_what_is_under_review() {
    let rows = queue();
    assert_eq!(
        review_line(&rows[0]).as_deref(),
        Some("v0.4.0 · artifact bcfb3b1c4109 · submitted 2026-07-24 11:05 UTC")
    );
    assert_eq!(review_line(&rows[1]), None);
    assert!(PREDATES_REVIEWS.contains("resubmits"));
    assert_eq!(terrain_label("everon"), "Everon");
    assert_eq!(terrain_label(""), "—");
}

/// Each decision sends the artifact of the pending review; a conditional approval and a rejection
/// need their text, trimmed.
#[test]
fn every_decision_names_the_artifact_under_review() {
    let artifact = queue()[0]
        .artifact_id
        .clone()
        .expect("the artifact under review");
    assert_eq!(
        decision_body(DecisionKind::Approve, &artifact, "ignored"),
        Ok(DecisionBody::Approve(ApprovalDecision {
            artifact_id: artifact.clone(),
            conditions: None,
        }))
    );
    assert_eq!(
        decision_body(
            DecisionKind::ApproveWithConditions,
            &artifact,
            " Night only \n"
        ),
        Ok(DecisionBody::Approve(ApprovalDecision {
            artifact_id: artifact.clone(),
            conditions: Some("Night only".into()),
        }))
    );
    assert_eq!(
        decision_body(DecisionKind::Reject, &artifact, "  Move the pad  "),
        Ok(DecisionBody::Reject(RejectionDecision {
            artifact_id: artifact.clone(),
            reason: "Move the pad".into(),
        }))
    );
    assert_eq!(
        decision_body(DecisionKind::ApproveWithConditions, &artifact, "  "),
        Err("Write the conditions first".to_string())
    );
    assert_eq!(
        decision_body(DecisionKind::Reject, &artifact, "\n"),
        Err("Write the reason first".to_string())
    );
    assert!(DecisionKind::Approve.text_label().is_none());
    assert!(DecisionKind::Reject.text_label().is_some());
    assert!(DecisionKind::ApproveWithConditions.text_label().is_some());
}

/// A stale decision reads the queue again and says why; the changed-artifact refusal names the
/// artifact under review now.
#[test]
fn stale_decisions_reload_the_queue_and_say_why() {
    let no_review = DecisionRefusal::from_refusal(
        &ApiRefusal::from_error_body(
            409,
            Some(
                &json!({"error": "the mission has no artifact under review; resubmit it to compile the current version", "details": {"code": "NO_PENDING_REVIEW"}}),
            ),
        ),
        "fallback",
    );
    assert_eq!(no_review, DecisionRefusal::NoPendingReview);
    assert!(no_review.reloads_queue());
    let changed = DecisionRefusal::from_refusal(
        &ApiRefusal::from_error_body(
            409,
            Some(
                &json!({"error": "the artifact under review is not the one decided", "details": {"code": "REVIEWED_ARTIFACT_CHANGED", "artifact_id": "00000000-0000-4000-f000-000000000009"}}),
            ),
        ),
        "fallback",
    );
    assert_eq!(
        changed,
        DecisionRefusal::ReviewedArtifactChanged {
            artifact_id: Some("00000000-0000-4000-f000-000000000009".into())
        }
    );
    assert!(changed.reloads_queue());
    assert!(changed
        .sentence()
        .contains("Artifact 00000000-0000-4000-f000-000000000009 is under review now"));
    let not_pending = DecisionRefusal::from_refusal(
        &ApiRefusal::from_error_body(
            409,
            Some(&json!({"error": "mission is not pending approval"})),
        ),
        "fallback",
    );
    assert!(not_pending.reloads_queue());
    assert_eq!(
        not_pending.sentence(),
        "Mission is not pending approval. The queue has been read again."
    );
    let invalid = DecisionRefusal::from_refusal(
        &ApiRefusal::from_error_body(
            400,
            Some(&json!({"error": "reason must contain 1 to 8000 bytes"})),
        ),
        "fallback",
    );
    assert!(!invalid.reloads_queue());
    assert_eq!(invalid.sentence(), "Reason must contain 1 to 8000 bytes");
}

/// The settings tiles read the wire enums as words.
#[test]
fn settings_tiles_read_the_wire_as_words() {
    assert_eq!(game_mode_label("pve_coop"), "COOP");
    assert_eq!(game_mode_label("zeus"), "Zeus");
    assert_eq!(enum_label("heavy_rain"), "Heavy rain");
    assert_eq!(enum_label(""), "—");
}

fn approvals_source() -> String {
    [
        include_str!("../page.rs"),
        include_str!("../review_drawer.rs"),
        include_str!("../review_decision.rs"),
    ]
    .map(live_code)
    .join("\n")
}

/// The decision goes through the typed endpoints with the artifact of the pending review, and a
/// stale refusal reads the queue again rather than leaving a decided or replaced review on screen.
#[test]
fn the_decision_is_sent_with_the_artifact_and_stale_refusals_reload() {
    let src = approvals_source();
    let form = only_body(&src, "pub(super) fn decision_form(");
    assert!(form.contains("approve_review(store, &mission, approval)"));
    assert!(form.contains("reject_review(store, &mission, rejection)"));
    assert!(form.contains("decision_body(decision, &artifact,"));
    let refusal = form
        .find("refusal.reloads_queue()")
        .expect("a refusal is classified");
    let reload = form[refusal..]
        .find("desk.refetch.run(())")
        .expect("a stale refusal reads the queue again");
    assert!(reload > 0);
    let drawer = only_body(&src, "pub(super) fn ReviewInspector(");
    assert!(drawer.contains("decision_form(row.mission_id.clone(), artifact_id, desk)"));
    assert!(drawer.contains("review_workspace_href(&row.mission_id, &artifact_id)"));
    assert!(drawer.contains("<ReviewCommentComposer"));
}
