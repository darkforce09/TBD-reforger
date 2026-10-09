use super::*;
use serde_json::json;

fn map(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap()
}

#[test]
fn console_outcome_refuses_4097_bytes_and_every_other_shape() {
    for outcome in [
        json!({"response": "r".repeat(4097), "response_truncated": true}),
        json!({"response": "é".repeat(2049), "response_truncated": true}),
        json!({"response": "Players on server:"}),
        json!({"response_truncated": false}),
        json!({"response": "ok", "response_truncated": false, "exit_code": 0}),
        json!({"response": 7, "response_truncated": false}),
        json!({"response": "ok", "response_truncated": "no"}),
    ] {
        for succeeded in [true, false] {
            let reported = map(outcome.clone());
            assert!(
                validated_outcome(FleetAction::ConsoleCommand, succeeded, Some(&reported)).is_err(),
                "{outcome} (succeeded: {succeeded})"
            );
        }
    }
}
