//! What an author is told for every reason the backend refuses a submission.

use super::submission_refusal::{RefusalFindings, SubmissionRefusal};
use frontend_transport::Error;
use serde_json::json;

/// A refusal as the backend answers it: its status, its sentence and its details.
fn refused(status: u16, error: &str, details: serde_json::Value) -> Error {
    Error::from_error_body(status, Some(&json!({"error": error, "details": details})))
}

/// The compile refusal body the backend sends: the code, the schema, the count and the findings.
fn compile_refusal(code: &str, error: &str, findings: &[&str], count: usize) -> Error {
    refused(
        422,
        error,
        json!({
            "code": code,
            "schema": "mission.schema.json",
            "finding_count": count,
            "findings": findings,
        }),
    )
}

#[test]
fn no_placed_slots_says_what_to_place() {
    let refusal = SubmissionRefusal::from_refusal(
        &compile_refusal("NO_PLACED_SLOTS", "the version has no placed slots", &[], 0),
        "fallback",
    );
    assert_eq!(refusal, SubmissionRefusal::NoPlacedSlots);
    assert!(refusal.findings().is_none());
}

#[test]
fn an_uncompilable_version_lists_what_the_compiler_reported() {
    let refusal = SubmissionRefusal::from_refusal(
        &compile_refusal(
            "UNCOMPILABLE_VERSION",
            "the version does not compile",
            &["/editor/vehicles/0: cargo exceeds the vehicle's capacity"],
            1,
        ),
        "fallback",
    );
    assert!(matches!(refusal, SubmissionRefusal::UncompilableVersion(_)));
    assert_eq!(
        refusal.findings().map(|f| f.shown.clone()),
        Some(vec![
            "/editor/vehicles/0: cargo exceeds the vehicle's capacity".to_string()
        ])
    );
}

/// Unsupported authored data names every authored path, and a capped list says how many it leaves
/// out.
#[test]
fn unsupported_authored_data_lists_every_authored_path() {
    let shown: Vec<String> = (0..20)
        .map(|i| format!("/editor/slots/{i}/loadout/customAttachment"))
        .collect();
    let shown_refs: Vec<&str> = shown.iter().map(String::as_str).collect();
    let refusal = SubmissionRefusal::from_refusal(
        &compile_refusal(
            "UNSUPPORTED_AUTHORED_DATA",
            "the version authors gameplay data the mission document cannot carry",
            &shown_refs,
            25,
        ),
        "fallback",
    );
    let findings = refusal.findings().expect("the authored paths");
    assert_eq!(findings.shown, shown);
    assert_eq!(findings.total, 25);
    assert_eq!(findings.unlisted(), 5);
}

/// A count smaller than the list is not trusted over the list itself.
#[test]
fn a_count_never_undercuts_the_listed_findings() {
    let refusal = SubmissionRefusal::from_refusal(
        &compile_refusal("UNSUPPORTED_AUTHORED_DATA", "x", &["/a", "/b"], 1),
        "fallback",
    );
    assert_eq!(
        refusal.findings(),
        Some(&RefusalFindings {
            shown: vec!["/a".into(), "/b".into()],
            total: 2,
        })
    );
}
