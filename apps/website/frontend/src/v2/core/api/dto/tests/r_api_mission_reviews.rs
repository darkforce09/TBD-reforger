//! Captured-response round trips for review histories, artifacts and the review workspace, and the
//! shapes of the comment and decision bodies.

use super::*;

const HISTORY_DECIDED: &str =
    golden!("GET__missions__00000000-0000-4000-c000-000000000001__reviews.json");
const HISTORY_PENDING: &str =
    golden!("GET__missions__00000000-0000-4000-c000-000000000004__reviews.json");
const ARTIFACT_APPROVED: &str = golden!(
    "GET__missions__00000000-0000-4000-c000-000000000001__artifacts__00000000-0000-4000-f000-000000000001.json"
);
const ARTIFACT_PENDING: &str = golden!(
    "GET__missions__00000000-0000-4000-c000-000000000004__artifacts__00000000-0000-4000-f000-000000000004.json"
);
const WORKSPACE: &str = golden!(
    "GET__missions__00000000-0000-4000-c000-000000000004__artifacts__00000000-0000-4000-f000-000000000004__workspace.json"
);

/// A mission rejected once and then approved with conditions: two decided reviews, newest first,
/// and a thread holding the rejection, the author's reply and the conditions, in order.
#[test]
fn review_history_with_decisions_and_thread() {
    assert_golden::<MissionReviewHistory>(HISTORY_DECIDED, &[]);
    let history: MissionReviewHistory = serde_json::from_str(HISTORY_DECIDED).unwrap();
    let states: Vec<&str> = history.reviews.iter().map(|r| r.state.as_str()).collect();
    assert_eq!(states, ["approved_with_conditions", "rejected"]);
    assert!(history
        .reviews
        .iter()
        .all(|r| r.decided_by.is_some() && r.decided_at.is_some()));
    let kinds: Vec<&str> = history.comments.iter().map(|c| c.kind.as_str()).collect();
    assert_eq!(kinds, ["rejection", "comment", "approval_conditions"]);
    let reply = &history.comments[1];
    assert!(
        reply.review_id.is_none() && reply.artifact_id.is_some(),
        "a thread comment belongs to no review, but can concern an artifact"
    );
    assert_eq!(
        history.comments[2].review_id.as_deref(),
        Some(history.reviews[0].id.as_str()),
        "the conditions belong to the approval that carries them"
    );
}

/// A review still pending: nothing decided it, so neither decision field is on the wire, and the
/// queue lists the same artifact and version as the one under review.
#[test]
fn review_history_of_a_pending_review() {
    assert_golden::<MissionReviewHistory>(HISTORY_PENDING, &[]);
    let history: MissionReviewHistory = serde_json::from_str(HISTORY_PENDING).unwrap();
    let pending = &history.reviews[0];
    assert_eq!(pending.state, "pending");
    assert!(pending.decided_by.is_none() && pending.decided_at.is_none());
    assert!(history.comments.is_empty());
    let queue: Paginated<ApprovalRow> =
        serde_json::from_str(golden!("GET__approvals.json")).unwrap();
    let row = &queue.data[0];
    assert_eq!(row.review_id.as_deref(), Some(pending.id.as_str()));
    assert_eq!(
        row.artifact_id.as_deref(),
        Some(pending.artifact_id.as_str())
    );
    assert_eq!(
        row.artifact_digest.as_deref(),
        Some(pending.artifact_digest.as_str())
    );
    assert_eq!(row.version_semver.as_deref(), Some(pending.semver.as_str()));
    assert_eq!(row.submitted_at, pending.submitted_at);
}

/// Both captured artifacts are claimed whole, their modpack included, and each is the artifact its
/// review names.
#[test]
fn artifact_provenance() {
    for (golden, history) in [
        (ARTIFACT_APPROVED, HISTORY_DECIDED),
        (ARTIFACT_PENDING, HISTORY_PENDING),
    ] {
        assert_golden::<MissionArtifact>(golden, &[]);
        let artifact: MissionArtifact = serde_json::from_str(golden).unwrap();
        let history: MissionReviewHistory = serde_json::from_str(history).unwrap();
        let review = &history.reviews[0];
        assert_eq!(artifact.id, review.artifact_id);
        assert_eq!(artifact.artifact_digest, review.artifact_digest);
        assert_eq!(artifact.mission_version_id, review.mission_version_id);
        assert!(artifact.modpack_id.is_some() && artifact.modpack_version.is_some());
        assert!(artifact.document_bytes > 0 && artifact.diagnostics.is_empty());
    }
    let approved: MissionArtifact = serde_json::from_str(ARTIFACT_APPROVED).unwrap();
    assert_eq!(approved.compiler_version, "website-map-engine 0.1.0");
    assert_eq!(approved.schema_version, "1.1");
    assert_eq!(approved.document_bytes, 1271);
    assert_eq!(approved.metadata.time_of_day, "05:30:00");
}

/// The workspace is the artifact and exactly the version it compiled from; the authored payload is
/// the editor superset, deliberately opaque.
#[test]
fn review_workspace() {
    assert_golden::<ReviewWorkspace>(WORKSPACE, &["version/json_payload"]);
    let workspace: ReviewWorkspace = serde_json::from_str(WORKSPACE).unwrap();
    assert_eq!(workspace.artifact.mission_version_id, workspace.version.id);
    assert_eq!(workspace.version.semver, "0.4.0");
    assert_eq!(
        workspace.version.editor_notes.as_deref(),
        Some("Resubmitted with placed seats")
    );
    let artifact: MissionArtifact = serde_json::from_str(ARTIFACT_PENDING).unwrap();
    assert_eq!(
        workspace.artifact, artifact,
        "the workspace's artifact is the artifact the provenance route serves"
    );
}

/// The compiler's row inputs, read off the artifact rather than off today's mission row: the
/// reviewed version compiles over what its artifact recorded.
#[test]
fn artifact_metadata_is_the_row_the_artifact_compiled_from() {
    let artifact: MissionArtifact = serde_json::from_str(ARTIFACT_PENDING).unwrap();
    let meta = artifact.metadata.compiled_meta();
    assert_eq!(meta.id, "00000000-0000-4000-c000-000000000004");
    assert_eq!(meta.title, "Operation Cold Anvil");
    assert_eq!(
        meta.author, "000000000000000003",
        "the account id, not a name"
    );
    assert_eq!(meta.terrain, "everon");
    assert_eq!(meta.custom_terrain_name, "");
    assert_eq!(meta.max_players, 32);
    assert_eq!(meta.time_of_day, "03:15:00");
    assert_eq!(meta.weather_preset, "heavy_rain");
}

/// A finding's subject id is always on the wire: an explicit null when it names no entity.
#[test]
fn a_diagnostic_carries_its_subject_id_even_when_null() {
    let with_subject = r#"{"message":"Slot SL (`s1`) authors tag \"alpha\" and the compile drops it","rule_id":"COMPILE-DROP-SLOT-TAG","severity":"info","subject":"/editor/slots/0/tag","subject_id":"s1"}"#;
    let without_subject = r#"{"message":"The mission authors no win condition","rule_id":"COMPILE-WIN-CONDITIONS","severity":"warning","subject":"/winConditions","subject_id":null}"#;
    assert_golden::<ArtifactDiagnostic>(with_subject, &[]);
    assert_golden::<ArtifactDiagnostic>(without_subject, &[]);
    let finding: ArtifactDiagnostic = serde_json::from_str(without_subject).unwrap();
    assert!(finding.subject_id.is_none());
    assert!(serde_json::to_string(&finding)
        .unwrap()
        .contains(r#""subject_id":null"#));
}

/// A comment names its artifact only when it is about one.
#[test]
fn review_comment_request_body() {
    assert_eq!(
        serde_json::to_value(ReviewCommentRequest {
            body: "Moved the pad".into(),
            artifact_id: Some("00000000-0000-4000-f000-000000000004".into()),
        })
        .unwrap(),
        serde_json::json!({
            "body": "Moved the pad",
            "artifact_id": "00000000-0000-4000-f000-000000000004"
        })
    );
    assert_eq!(
        serde_json::to_value(ReviewCommentRequest {
            body: "General note".into(),
            artifact_id: None,
        })
        .unwrap(),
        serde_json::json!({"body": "General note"})
    );
}

/// Both decisions name the artifact under review; conditions are sent only when there are any, and
/// a rejection always carries its reason.
#[test]
fn decision_bodies() {
    let artifact = "00000000-0000-4000-f000-000000000004";
    assert_eq!(
        serde_json::to_value(ApprovalDecision {
            artifact_id: artifact.into(),
            conditions: None,
        })
        .unwrap(),
        serde_json::json!({"artifact_id": artifact})
    );
    assert_eq!(
        serde_json::to_value(ApprovalDecision {
            artifact_id: artifact.into(),
            conditions: Some("Night rotation only".into()),
        })
        .unwrap(),
        serde_json::json!({"artifact_id": artifact, "conditions": "Night rotation only"})
    );
    assert_eq!(
        serde_json::to_value(RejectionDecision {
            artifact_id: artifact.into(),
            reason: "The pad sits in the minefield".into(),
        })
        .unwrap(),
        serde_json::json!({"artifact_id": artifact, "reason": "The pad sits in the minefield"})
    );
}
