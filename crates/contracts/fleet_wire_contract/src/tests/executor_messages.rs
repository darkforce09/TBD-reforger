//! The executor shapes keep their exact JSON in both directions: the API's claim reads into the
//! agent's value, and the agent's reports read into the API's value, byte for byte.

use chrono::{DateTime, TimeZone, Utc};
use serde_json::json;

use super::*;

fn lease_expiry() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 23, 12, 0, 30)
        .single()
        .unwrap()
        + chrono::Duration::milliseconds(500)
}

fn claim() -> ClaimedFleetCommand {
    ClaimedFleetCommand {
        command_id: "6f1c2b9e-3a4d-4e5f-8a7b-9c0d1e2f3a4b".parse().unwrap(),
        server_id: "0e9d8c7b-6a5f-4e3d-2c1b-0a9f8e7d6c5b".parse().unwrap(),
        action: "console_command".to_owned(),
        arguments: json!({ "line": "#restart" }),
        fencing_token: 7,
        lease_expires_at: lease_expiry(),
    }
}

const CLAIM_GOLDEN: &str = concat!(
    r#"{"command_id":"6f1c2b9e-3a4d-4e5f-8a7b-9c0d1e2f3a4b","#,
    r#""server_id":"0e9d8c7b-6a5f-4e3d-2c1b-0a9f8e7d6c5b","#,
    r##""action":"console_command","arguments":{"line":"#restart"},"##,
    r#""fencing_token":7,"lease_expires_at":"2026-09-23T12:00:30.5Z"}"#,
);

#[test]
fn a_claim_round_trips_byte_for_byte() {
    assert_eq!(serde_json::to_string(&claim()).unwrap(), CLAIM_GOLDEN);
    let read: ClaimedFleetCommand = serde_json::from_str(CLAIM_GOLDEN).unwrap();
    assert_eq!(read, claim());
    assert_eq!(serde_json::to_string(&read).unwrap(), CLAIM_GOLDEN);
}

#[test]
fn a_claim_reads_any_offset_and_tolerates_keys_it_does_not_know() {
    let shifted = CLAIM_GOLDEN.replace("12:00:30.5Z", "14:00:30.500+02:00");
    assert_eq!(
        serde_json::from_str::<ClaimedFleetCommand>(&shifted).unwrap(),
        claim()
    );
    let extended = CLAIM_GOLDEN.replace(r#""fencing_token":7"#, r#""fencing_token":7,"note":1"#);
    assert_eq!(
        serde_json::from_str::<ClaimedFleetCommand>(&extended).unwrap(),
        claim()
    );
}

#[test]
fn a_start_report_carries_only_the_fencing_token() {
    let start = ExecutionStart { fencing_token: 7 };
    let golden = r#"{"fencing_token":7}"#;
    assert_eq!(serde_json::to_string(&start).unwrap(), golden);
    assert_eq!(
        serde_json::from_str::<ExecutionStart>(golden).unwrap(),
        start
    );
    for invalid in [
        r#"{"fencing_token":7,"at":1}"#,
        "{}",
        r#"{"fencing_token":"7"}"#,
    ] {
        assert!(
            serde_json::from_str::<ExecutionStart>(invalid).is_err(),
            "{invalid}"
        );
    }
}

#[test]
fn a_success_report_carries_its_outcome_and_omits_the_failure_reason() {
    let mut outcome = serde_json::Map::new();
    outcome.insert("response".to_owned(), json!("ok"));
    outcome.insert("response_truncated".to_owned(), json!(false));
    let result = ExecutionResult {
        fencing_token: 7,
        succeeded: true,
        outcome: Some(outcome),
        failure_reason: None,
    };
    let golden = concat!(
        r#"{"fencing_token":7,"succeeded":true,"#,
        r#""outcome":{"response":"ok","response_truncated":false}}"#,
    );
    assert_eq!(serde_json::to_string(&result).unwrap(), golden);
    let read: ExecutionResult = serde_json::from_str(golden).unwrap();
    assert_eq!(read, result);
    assert_eq!(serde_json::to_string(&read).unwrap(), golden);
}

#[test]
fn a_failure_report_omits_an_absent_outcome() {
    let result = ExecutionResult {
        fencing_token: 3,
        succeeded: false,
        outcome: None,
        failure_reason: Some("the unit did not start".to_owned()),
    };
    let golden =
        r#"{"fencing_token":3,"succeeded":false,"failure_reason":"the unit did not start"}"#;
    assert_eq!(serde_json::to_string(&result).unwrap(), golden);
    assert_eq!(
        serde_json::from_str::<ExecutionResult>(golden).unwrap(),
        result
    );
}

#[test]
fn a_report_reads_null_as_absent_and_refuses_unknown_keys() {
    let with_nulls = r#"{"fencing_token":3,"succeeded":true,"outcome":null,"failure_reason":null}"#;
    let read: ExecutionResult = serde_json::from_str(with_nulls).unwrap();
    assert_eq!(
        read,
        ExecutionResult {
            fencing_token: 3,
            succeeded: true,
            outcome: None,
            failure_reason: None,
        }
    );
    assert_eq!(
        serde_json::to_string(&read).unwrap(),
        r#"{"fencing_token":3,"succeeded":true}"#
    );
    for invalid in [
        r#"{"fencing_token":3,"succeeded":true,"players":[]}"#,
        r#"{"fencing_token":3}"#,
        r#"{"fencing_token":3,"succeeded":true,"outcome":[]}"#,
    ] {
        assert!(
            serde_json::from_str::<ExecutionResult>(invalid).is_err(),
            "{invalid}"
        );
    }
}
