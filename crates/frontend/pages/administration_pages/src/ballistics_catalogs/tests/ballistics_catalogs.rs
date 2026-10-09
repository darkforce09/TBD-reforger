//! Ballistics catalog screen: upload readiness, the outcome of each answer the upload route can
//! give, and the order of the stored-version rows.

use super::*;
use frontend_api_dtos::ballistics_catalogs::{
    BallisticsCatalogList, BallisticsCatalogSummary, CalibrationFailure, CatalogUploadReport,
};
use frontend_transport::Error;
use serde_json::{Value, json};

fn picked(name: &str) -> PickedFile {
    PickedFile {
        name: name.into(),
        size_bytes: 2048.0,
    }
}

fn failure(case_id: &str, reason: &str) -> CalibrationFailure {
    CalibrationFailure {
        case_id: case_id.into(),
        reason: reason.into(),
    }
}

fn refusal(status: u16, body: Value) -> Error {
    Error::from_error_body(status, Some(&body))
}

fn summary(id: &str, version: u32, sha: &str) -> BallisticsCatalogSummary {
    BallisticsCatalogSummary {
        catalog_id: id.into(),
        catalog_version: version,
        title: format!("{id} v{version}"),
        game_build: "1.4.0.53".into(),
        export_generation_id: "0f3a".into(),
        catalog_sha256: sha.into(),
        uploaded_at: "2026-09-28T10:15:00Z".into(),
    }
}

#[test]
fn upload_is_blocked_until_both_parts_are_picked() {
    let catalog = picked("vanilla.json");
    let bundle = picked("vanilla.calibration.json");
    assert_eq!(
        upload_blocker(None, None).as_deref(),
        Some("Choose the catalog and its calibration bundle.")
    );
    assert_eq!(
        upload_blocker(None, Some(&bundle)).as_deref(),
        Some("Choose the catalog document.")
    );
    assert_eq!(
        upload_blocker(Some(&catalog), None).as_deref(),
        Some("Choose the calibration bundle.")
    );
    assert_eq!(upload_blocker(Some(&catalog), Some(&bundle)), None);
}

#[test]
fn upload_is_blocked_when_either_part_is_not_json() {
    let catalog = picked("vanilla.json");
    let bundle = picked("vanilla.calibration.json");
    let text = picked("notes.txt");
    let catalog_blocker = upload_blocker(Some(&text), Some(&bundle)).expect("blocked");
    assert!(
        catalog_blocker.contains("catalog \"notes.txt\""),
        "{catalog_blocker}"
    );
    let bundle_blocker = upload_blocker(Some(&catalog), Some(&text)).expect("blocked");
    assert!(
        bundle_blocker.contains("calibration bundle \"notes.txt\""),
        "{bundle_blocker}"
    );
}

#[test]
fn json_names_are_matched_in_any_letter_case() {
    assert!(is_json_name("CATALOG.JSON"));
    assert!(is_json_name("a.Json"));
    assert!(!is_json_name("catalog.json.gz"));
    assert!(!is_json_name("json"));
}

#[test]
fn a_created_report_without_failures_is_accepted() {
    let report = CatalogUploadReport {
        accepted: true,
        cases: 128,
        failures: vec![],
        forward_samples_not_judged: 0,
    };
    let outcome = upload_outcome(Ok(report.clone()));
    assert_eq!(outcome, UploadOutcome::Accepted(report));
    let (tone, headline) = outcome_headline(&outcome);
    assert_eq!(tone, OutcomeTone::Success);
    assert!(
        headline.contains("all 128 calibration cases passed"),
        "{headline}"
    );
    assert_eq!(outcome_key(&outcome), "accepted");
    assert!(outcome_failures(&outcome).is_empty());
}

#[test]
fn a_calibration_refusal_keeps_every_failed_case() {
    let body = json!({
        "error": "calibration failed",
        "details": {
            "accepted": false,
            "cases": 40,
            "failures": [
                {"case_id": "native/he/ring2/row7", "reason": "range 2410.3 m outside [2398.0, 2405.1]"},
                {"case_id": "provenance/game_build", "reason": "bundle 1.4.0.52, catalog 1.4.0.53"}
            ],
            "forward_samples_not_judged": 15427
        }
    });
    let outcome = upload_outcome(Err(refusal(422, body)));
    let expected = CatalogUploadReport {
        accepted: false,
        cases: 40,
        failures: vec![
            failure(
                "native/he/ring2/row7",
                "range 2410.3 m outside [2398.0, 2405.1]",
            ),
            failure("provenance/game_build", "bundle 1.4.0.52, catalog 1.4.0.53"),
        ],
        forward_samples_not_judged: 15427,
    };
    assert_eq!(outcome, UploadOutcome::CalibrationRefused(expected.clone()));
    let (tone, headline) = outcome_headline(&outcome);
    assert_eq!(tone, OutcomeTone::Failure);
    assert!(
        headline.contains("2 of 40 calibration cases failed"),
        "{headline}"
    );
    assert_eq!(outcome_key(&outcome), "calibration-refused");
    assert_eq!(outcome_failures(&outcome), expected.failures.as_slice());
}

#[test]
fn a_calibration_refusal_tolerates_a_code_beside_the_report() {
    let details = json!({"code": "calibration_failed", "failures": [], "cases": 3});
    let report = report_from_details(details.as_object().expect("object")).expect("report");
    assert_eq!(
        report,
        CatalogUploadReport {
            accepted: false,
            cases: 3,
            failures: vec![],
            forward_samples_not_judged: 0,
        }
    );
    let without_cases = json!({"failures": [{"case_id": "a", "reason": "b"}]});
    let report = report_from_details(without_cases.as_object().expect("object")).expect("report");
    assert_eq!(report.cases, 0);
    assert_eq!(report.failures, vec![failure("a", "b")]);
}

#[test]
fn a_422_without_failures_is_a_rejection_with_the_backend_sentence() {
    let body = json!({"error": "catalog document is not valid JSON"});
    let outcome = upload_outcome(Err(refusal(422, body)));
    assert_eq!(
        outcome,
        UploadOutcome::Rejected {
            status: 422,
            message: "Catalog document is not valid JSON".into()
        }
    );
    let malformed = json!({"error": "bad", "details": {"failures": "not a list"}});
    assert!(matches!(
        upload_outcome(Err(refusal(422, malformed))),
        UploadOutcome::Rejected { status: 422, .. }
    ));
}

#[test]
fn a_conflict_is_a_duplicate_version() {
    let body = json!({"error": "catalog version vanilla-mortars v2 already exists"});
    let outcome = upload_outcome(Err(refusal(409, body)));
    assert_eq!(
        outcome,
        UploadOutcome::Duplicate("Catalog version vanilla-mortars v2 already exists".into())
    );
    let (tone, headline) = outcome_headline(&outcome);
    assert_eq!(tone, OutcomeTone::Warning);
    assert!(headline.starts_with("Already stored — "), "{headline}");
    assert_eq!(outcome_key(&outcome), "duplicate");
}

#[test]
fn an_ended_session_and_an_unreached_server_are_warnings() {
    let expired = upload_outcome(Err(refusal(401, json!({"error": "expired"}))));
    assert_eq!(expired, UploadOutcome::SessionExpired);
    assert_eq!(outcome_key(&expired), "session-expired");
    let unreached = upload_outcome(Err(Error::Transport));
    assert_eq!(unreached, UploadOutcome::Unreachable);
    assert_eq!(outcome_key(&unreached), "unreachable");
    assert_eq!(outcome_headline(&expired).0, OutcomeTone::Warning);
    assert_eq!(outcome_headline(&unreached).0, OutcomeTone::Warning);
}

#[test]
fn a_created_report_that_is_not_accepted_is_never_shown_as_stored() {
    let report = CatalogUploadReport {
        accepted: false,
        cases: 5,
        failures: vec![failure("x", "y")],
        forward_samples_not_judged: 0,
    };
    assert_eq!(
        upload_outcome(Ok(report.clone())),
        UploadOutcome::CalibrationRefused(report)
    );
}

#[test]
fn version_rows_order_catalogs_by_id_and_versions_newest_first() {
    let sha = "a".repeat(64);
    let list = BallisticsCatalogList {
        data: vec![
            summary("vanilla-mortars", 1, &sha),
            summary("community-mortars", 1, &sha),
            summary("vanilla-mortars", 3, &sha),
            summary("vanilla-mortars", 2, &sha),
        ],
    };
    let rows = version_rows(&list);
    let order: Vec<(&str, u32, bool)> = rows
        .iter()
        .map(|row| (row.catalog_id.as_str(), row.catalog_version, row.latest))
        .collect();
    assert_eq!(
        order,
        vec![
            ("community-mortars", 1, true),
            ("vanilla-mortars", 3, true),
            ("vanilla-mortars", 2, false),
            ("vanilla-mortars", 1, false),
        ]
    );
}
