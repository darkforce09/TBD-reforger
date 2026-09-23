//! The review record's words, held against the captured review histories and artifacts.

use super::artifact_provenance_view::provenance_rows;
use super::review_record::approved_artifact_line;
use super::review_wording::*;
use crate::v2::core::api::dto::{ArtifactDiagnostic, MissionArtifact, MissionReviewHistory};
use crate::v2::core::test_support::fixtures::golden;

const DECIDED: &str = golden!("GET__missions__00000000-0000-4000-c000-000000000001__reviews.json");
const PENDING: &str = golden!("GET__missions__00000000-0000-4000-c000-000000000004__reviews.json");
const ARTIFACT: &str = golden!(
    "GET__missions__00000000-0000-4000-c000-000000000001__artifacts__00000000-0000-4000-f000-000000000001.json"
);

fn history(text: &str) -> MissionReviewHistory {
    serde_json::from_str(text).unwrap()
}

/// A digest is identified on screen by its first twelve hex digits.
#[test]
fn a_digest_is_shown_by_its_first_twelve_digits() {
    let history = history(PENDING);
    let review = &history.reviews[0];
    assert_eq!(short_digest(&review.artifact_digest), "bcfb3b1c4109");
    assert_eq!(
        reviewed_artifact_line(&review.semver, &review.artifact_digest),
        "v0.4.0 · artifact bcfb3b1c4109"
    );
    assert_eq!(short_digest("abc"), "abc", "a short value is shown whole");
}

/// Every state the backend sends has its own words and tone; one this build does not know is shown
/// as the backend spells it, in the neutral tone.
#[test]
fn every_review_state_is_named_and_toned() {
    for (state, label, tone) in [
        ("pending", "Pending review", "warning"),
        ("approved", "Approved", "success"),
        (
            "approved_with_conditions",
            "Approved with conditions",
            "tertiary",
        ),
        ("rejected", "Rejected", "error"),
        ("superseded", "Superseded", "neutral"),
        ("under_appeal", "under appeal", "neutral"),
    ] {
        assert_eq!(review_state_label(state), label);
        assert_eq!(review_state_tone(state), tone);
    }
    assert_eq!(comment_kind_label("rejection"), "Rejection reason");
    assert_eq!(
        comment_kind_label("approval_conditions"),
        "Approval conditions"
    );
    assert_eq!(comment_kind_label("comment"), "Comment");
    assert_eq!(comment_kind_label("escalation"), "escalation");
}

/// A decided review says who decided it and when; the viewer reads their own decisions as "you".
#[test]
fn decided_reviews_name_their_decision() {
    let history = history(DECIDED);
    let approved = &history.reviews[0];
    let rejected = &history.reviews[1];
    assert_eq!(
        decision_line(approved, Some("000000000000000001")),
        "Approved with conditions by you, 2026-07-24 15:00 UTC"
    );
    assert_eq!(
        decision_line(rejected, None),
        "Rejected by 000000000000000001, 2026-07-24 12:00 UTC"
    );
    assert_eq!(
        submission_line(approved, Some("000000000000000003")),
        "Submitted by you, 2026-07-24 13:00 UTC"
    );
}

/// A pending review awaits a decision, and a superseded one says a later submission replaced it.
#[test]
fn undecided_reviews_say_why_they_are_undecided() {
    let history = history(PENDING);
    let mut review = history.reviews[0].clone();
    assert_eq!(
        decision_line(&review, None),
        "Awaiting a reviewer's decision"
    );
    review.state = "superseded".into();
    review.decided_at = Some("2026-07-25T08:00:00Z".into());
    assert_eq!(
        decision_line(&review, None),
        "Replaced by a later submission, 2026-07-25 08:00 UTC"
    );
    review.decided_at = None;
    assert_eq!(
        decision_line(&review, None),
        "Replaced by a later submission"
    );
}

/// The conditions and the rejection reason belong to the review that carries them; the author's
/// reply belongs to none.
#[test]
fn a_decision_comment_belongs_to_its_review() {
    let history = history(DECIDED);
    assert_eq!(
        decision_comment(&history, &history.reviews[0].id),
        Some((
            "approval_conditions",
            "Night rotation only until the BTR patrol is retuned."
        ))
    );
    assert_eq!(
        decision_comment(&history, &history.reviews[1].id),
        Some((
            "rejection",
            "The extraction helicopter spawns inside the minefield."
        ))
    );
    assert_eq!(decision_comment(&history, "no-such-review"), None);
    assert!(pending_review(&history).is_none());
    assert!(pending_review(&self::history(PENDING)).is_some());
}

/// The approved artifact is named by the version and digest of the review that approved it, and by
/// its id alone when no review in the history decided it.
#[test]
fn the_approved_artifact_is_identified() {
    let history = history(DECIDED);
    assert_eq!(
        approved_artifact_line(&history, "00000000-0000-4000-f000-000000000001"),
        "Approved artifact: v1.3.0 · artifact 2534548d0f87 — deployments run exactly these bytes."
    );
    assert_eq!(
        approved_artifact_line(&history, "00000000-0000-4000-f000-000000000009"),
        "Approved artifact 00000000-0000-4000-f000-000000000009 — deployments run exactly these bytes."
    );
}

/// The history is read unasked only once the row shows a review has happened.
#[test]
fn a_review_trace_is_read_off_the_row() {
    assert!(!has_review_trace("draft", None, None));
    assert!(!has_review_trace("archived", None, None));
    for status in ["pending_approval", "live", "rejected"] {
        assert!(has_review_trace(status, None, None), "{status}");
    }
    assert!(has_review_trace(
        "draft",
        Some("2026-07-24T15:00:00Z"),
        None
    ));
    assert!(has_review_trace("archived", None, Some("artifact")));
}

/// Every provenance row a reviewer compares, read off the captured artifact.
#[test]
fn provenance_rows_read_the_captured_artifact() {
    let artifact: MissionArtifact = serde_json::from_str(ARTIFACT).unwrap();
    let rows = provenance_rows(&artifact, "1.3.0");
    let value = |label: &str| {
        rows.iter()
            .find(|(l, _)| *l == label)
            .map(|(_, v)| v.as_str())
            .unwrap_or_else(|| panic!("no {label} row"))
    };
    assert_eq!(value("Version"), "v1.3.0");
    assert_eq!(value("Compiled"), "2026-07-24 10:05 UTC");
    assert_eq!(value("Compiler"), "website-map-engine 0.1.0");
    assert_eq!(value("Schema"), "1.1");
    assert_eq!(
        value("Modpack"),
        "00000000-0000-4000-a000-000000000001 · version 2.1"
    );
    assert_eq!(value("Terrain"), "arland");
    assert_eq!(value("Document size"), "1.2 KiB (1271 bytes)");
    assert_eq!(value("Document SHA-256"), artifact.document_sha256);
    assert_eq!(value("Artifact digest"), artifact.artifact_digest);
    let mut bare = artifact.clone();
    bare.modpack_id = None;
    bare.modpack_version = None;
    let rows = provenance_rows(&bare, "1.3.0");
    assert!(rows.iter().any(|(label, value)| *label == "Modpack"
        && value == "None — no modpack was current at compile time"));
}

/// Sizes read in the unit a reader compares, with the exact count beside them.
#[test]
fn document_sizes_keep_their_exact_byte_count() {
    assert_eq!(document_size_label(800), "800 bytes");
    assert_eq!(document_size_label(1271), "1.2 KiB (1271 bytes)");
    assert_eq!(document_size_label(8_388_608), "8.0 MiB (8388608 bytes)");
}

/// A finding names its entity when it has one, and the tone follows its severity.
#[test]
fn a_finding_names_what_it_is_about() {
    let finding = ArtifactDiagnostic {
        rule_id: "COMPILE-DROP-SLOT-TAG".into(),
        severity: "info".into(),
        subject: "/editor/slots/0/tag".into(),
        subject_id: Some("s1".into()),
        message: "dropped".into(),
    };
    assert_eq!(diagnostic_subject(&finding), "/editor/slots/0/tag (s1)");
    let mut blank = finding.clone();
    blank.subject_id = None;
    assert_eq!(diagnostic_subject(&blank), "/editor/slots/0/tag");
    assert_eq!(severity_tone("error"), "error");
    assert_eq!(severity_tone("warning"), "warning");
    assert_eq!(severity_tone("info"), "neutral");
}

/// The review workspace is addressed by the frontend route, with its data encoded.
#[test]
fn the_review_workspace_link_is_the_frontend_route() {
    assert_eq!(
        review_workspace_href(
            "00000000-0000-4000-c000-000000000004",
            "00000000-0000-4000-f000-000000000004"
        ),
        "/missions/00000000-0000-4000-c000-000000000004/artifacts/00000000-0000-4000-f000-000000000004/workspace"
    );
    assert_eq!(
        review_workspace_href("m/1", "a b"),
        "/missions/m%2F1/artifacts/a%20b/workspace"
    );
}

/// Review text is trimmed and bounded as the backend bounds it.
#[test]
fn review_text_is_trimmed_and_bounded() {
    assert_eq!(
        validated_review_text("  Night only \n", "conditions"),
        Ok("Night only".to_string())
    );
    assert_eq!(
        validated_review_text(" \t\n", "reason"),
        Err("Write the reason first".to_string())
    );
    assert!(validated_review_text(&"x".repeat(8000), "comment").is_ok());
    assert_eq!(
        validated_review_text(&"é".repeat(4001), "comment"),
        Err("The comment is too long: 8002 bytes, at most 8000".to_string())
    );
}
