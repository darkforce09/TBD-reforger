use super::*;
use serde_json::json;

fn map(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap()
}

#[test]
fn console_outcome_accepts_a_response_of_4096_bytes() {
    let longest = "r".repeat(ConsoleCommandOutcome::RESPONSE_MAX_BYTES);
    let reported = map(json!({"response": longest, "response_truncated": true}));
    assert_eq!(
        validated_outcome(FleetAction::ConsoleCommand, true, Some(&reported)).unwrap(),
        Some(json!({"response": longest, "response_truncated": true}))
    );
    let multibyte = map(json!({"response": "é".repeat(2048), "response_truncated": false}));
    assert!(validated_outcome(FleetAction::ConsoleCommand, true, Some(&multibyte)).is_ok());
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

#[test]
fn console_success_requires_its_outcome_and_a_failure_may_omit_it() {
    assert!(validated_outcome(FleetAction::ConsoleCommand, true, None).is_err());
    assert_eq!(
        validated_outcome(FleetAction::ConsoleCommand, false, None).unwrap(),
        None
    );
}

#[test]
fn other_actions_keep_their_free_form_outcome() {
    let players = map(json!({"players": [{"arma_id": "arma-7", "name": "Seven"}]}));
    for action in FleetAction::ALL
        .into_iter()
        .filter(|action| *action != FleetAction::ConsoleCommand)
    {
        assert_eq!(
            validated_outcome(action, true, Some(&players)).unwrap(),
            Some(Value::Object(players.clone()))
        );
        assert_eq!(validated_outcome(action, true, None).unwrap(), None);
    }
}
