use serde_json::json;

use super::*;

fn outcome(value: Value) -> Map<String, Value> {
    value.as_object().cloned().expect("an object")
}

#[test]
fn a_success_carries_its_outcome_and_no_reason() {
    let verdict = ActionVerdict::success(outcome(json!({"active_state": "active"})));
    assert!(verdict.succeeded());
    assert_eq!(verdict.failure_reason(), None);
    assert_eq!(verdict.outcome().unwrap()["active_state"], "active");
}

#[test]
fn a_failure_always_names_a_reason() {
    let verdict = ActionVerdict::failure("   ", None);
    assert!(!verdict.succeeded());
    assert_eq!(verdict.failure_reason(), Some(UNNAMED_FAILURE));
    let verdict = ActionVerdict::failure("  unit failed  ", None);
    assert_eq!(verdict.failure_reason(), Some("unit failed"));
}

#[test]
fn long_reasons_are_cut_to_the_ledger_limit_on_a_character_boundary() {
    // Two-byte characters make every odd byte offset a non-boundary.
    let reason = "é".repeat(400);
    let verdict = ActionVerdict::failure(&reason, None);
    let bounded = verdict.failure_reason().unwrap();
    assert!(
        bounded.len() <= FAILURE_REASON_MAX_BYTES,
        "{}",
        bounded.len()
    );
    assert!(bounded.ends_with(TRUNCATION_MARK));
    assert!(
        bounded
            .trim_end_matches(TRUNCATION_MARK)
            .chars()
            .all(|c| c == 'é')
    );

    let exact = "x".repeat(FAILURE_REASON_MAX_BYTES);
    assert_eq!(
        ActionVerdict::failure(&exact, None).failure_reason(),
        Some(exact.as_str())
    );
}

#[test]
fn the_parts_are_the_fields_of_the_result_report() {
    let verdict = ActionVerdict::failure("refused", Some(outcome(json!({"unit": "a"}))));
    let (succeeded, outcome, reason) = verdict.into_parts();
    assert!(!succeeded);
    assert_eq!(outcome.unwrap()["unit"], "a");
    assert_eq!(reason.as_deref(), Some("refused"));
}
