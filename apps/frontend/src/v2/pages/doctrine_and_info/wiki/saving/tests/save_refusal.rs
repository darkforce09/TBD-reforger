//! Refusal mapping: the 409 conflict with its reload, the 422 findings with their lines, the 400
//! and 413 size refusals, and every other refusal's sentence.

use super::*;
use crate::v2::core::api::dto::wiki::WikiMarkupFindingCode;
use serde_json::json;

/// The refusal a request answered with `status` and `body`.
fn refusal(status: u16, body: serde_json::Value) -> ApiRefusal {
    ApiRefusal::from_error_body(status, Some(&body))
}

#[test]
fn wiki_refusal_conflict_names_both_revisions_and_offers_a_reload() {
    let answer = refusal(
        409,
        json!({
            "error": "the page changed since revision 4",
            "details": {"code": "wiki_revision_conflict", "current_revision": 6}
        }),
    );
    let problem = SaveProblem::from_refusal(&answer, Some(4));
    assert_eq!(
        problem,
        SaveProblem::RevisionConflict {
            base_revision: Some(4),
            current_revision: Some(6),
        }
    );
    assert_eq!(
        problem.headline(),
        "Not saved: this manual changed while you were working. You started from revision 4; \
         it is now at revision 6."
    );
    let draft = SaveFailure {
        origin: SaveOrigin::Draft,
        problem: problem.clone(),
    };
    assert_eq!(
        draft.reload_label().as_deref(),
        Some("Discard my draft and load revision 6")
    );
    let restore = SaveFailure {
        origin: SaveOrigin::Restore,
        problem,
    };
    assert_eq!(restore.reload_label().as_deref(), Some("Load revision 6"));
}

#[test]
fn wiki_refusal_a_bare_409_is_still_a_conflict() {
    let answer = refusal(409, json!({"error": "wiki page already exists"}));
    let problem = SaveProblem::from_refusal(&answer, None);
    assert_eq!(
        problem,
        SaveProblem::RevisionConflict {
            base_revision: None,
            current_revision: None,
        }
    );
    assert_eq!(
        problem.headline(),
        "Not saved: this manual changed while you were working."
    );
    let failure = SaveFailure {
        origin: SaveOrigin::Draft,
        problem,
    };
    assert_eq!(
        failure.reload_label().as_deref(),
        Some("Discard my draft and load the latest revision")
    );
}

#[test]
fn wiki_refusal_markup_lists_every_finding_with_its_line() {
    let answer = refusal(
        422,
        json!({
            "error": "the markup holds refused constructs",
            "details": {
                "code": "wiki_markup_refused",
                "findings": [
                    {"line": 3, "code": "raw_html", "detail": "raw HTML is not allowed"},
                    {"line": 9, "code": "unsafe_link_url", "detail": "link address javascript:alert(1) is not allowed"}
                ]
            }
        }),
    );
    let problem = SaveProblem::from_refusal(&answer, Some(2));
    let SaveProblem::MarkupRefused { findings } = &problem else {
        panic!("a 422 wiki_markup_refused is refused markup, got {problem:?}");
    };
    assert_eq!(findings.len(), 2);
    assert_eq!(findings[0].code, WikiMarkupFindingCode::RawHtml);
    assert_eq!(problem.headline(), "Not saved: the markup has 2 problems.");
    assert_eq!(
        problem.finding_lines(),
        vec![
            "Line 3: raw HTML is not allowed".to_string(),
            "Line 9: link address javascript:alert(1) is not allowed".to_string(),
        ]
    );
    let failure = SaveFailure {
        origin: SaveOrigin::Draft,
        problem,
    };
    assert_eq!(
        failure.reload_label(),
        None,
        "only a conflict offers a reload"
    );
}

#[test]
fn wiki_refusal_one_finding_reads_in_the_singular() {
    let problem = SaveProblem::MarkupRefused {
        findings: vec![WikiMarkupFinding {
            line: 1,
            code: WikiMarkupFindingCode::NestingTooDeep,
            detail: "nesting deeper than 16".into(),
        }],
    };
    assert_eq!(problem.headline(), "Not saved: the markup has 1 problem.");
}

#[test]
fn wiki_refusal_body_too_large_and_request_too_large_are_told_apart() {
    let body = refusal(
        400,
        json!({"error": "body_md is too large", "details": {"code": "wiki_body_too_large"}}),
    );
    let problem = SaveProblem::from_refusal(&body, Some(1));
    assert_eq!(problem, SaveProblem::BodyTooLarge);
    assert!(problem.headline().contains("262 144 bytes"));
    assert!(problem.finding_lines().is_empty());

    let request = refusal(
        413,
        json!({"error": "request body too large", "details": {"code": "request_too_large"}}),
    );
    let problem = SaveProblem::from_refusal(&request, Some(1));
    assert_eq!(problem, SaveProblem::RequestTooLarge);
    assert_eq!(
        problem.headline(),
        "Not saved: the request is larger than the server accepts."
    );
}

#[test]
fn wiki_refusal_any_other_400_shows_the_servers_sentence() {
    let answer = refusal(
        400,
        json!({"error": "category, title and body_md are required"}),
    );
    assert_eq!(
        SaveProblem::from_refusal(&answer, Some(1)),
        SaveProblem::Refused {
            message: "Category, title and body_md are required".into()
        }
    );
    let silent = ApiRefusal::from_error_body(400, None);
    assert_eq!(
        SaveProblem::from_refusal(&silent, Some(1)).headline(),
        "Failed to save wiki page"
    );
}

#[test]
fn wiki_refusal_unreached_expired_and_forbidden_saves_say_so() {
    assert_eq!(
        SaveProblem::from_refusal(&ApiRefusal::unreadable(), Some(1)).headline(),
        "Not saved: the request did not reach the server. Check the connection and try again."
    );
    assert_eq!(
        SaveProblem::from_refusal(&ApiRefusal::from_error_body(401, None), Some(1)).headline(),
        "Not saved: your session has ended. Sign in again, then save."
    );
    assert_eq!(
        SaveProblem::from_refusal(&refusal(403, json!({"error": "admin only"})), Some(1))
            .headline(),
        "Not saved: only an administrator can save a manual."
    );
}
