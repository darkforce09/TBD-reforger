//! The operator shapes keep their exact JSON: what the API writes reads back into the same value
//! and writes the same bytes again.

use chrono::{DateTime, TimeZone, Utc};
use serde_json::json;

use super::*;

const COMMAND: &str = "6f1c2b9e-3a4d-4e5f-8a7b-9c0d1e2f3a4b";
const SERVER: &str = "0e9d8c7b-6a5f-4e3d-2c1b-0a9f8e7d6c5b";

fn at(second: u32, nanos: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 9, 23, 12, 0, second)
        .single()
        .unwrap()
        + chrono::Duration::nanoseconds(i64::from(nanos))
}

fn queued_receipt() -> FleetCommandReceipt {
    FleetCommandReceipt {
        id: COMMAND.parse().unwrap(),
        server_id: SERVER.parse().unwrap(),
        executor_kind: "host_agent".to_owned(),
        action: "restart".to_owned(),
        arguments: json!({}),
        requested_by: "123456789012345678".to_owned(),
        requested_at: at(0, 250_000_000),
        expires_at: at(0, 0) + chrono::Duration::seconds(300),
        state: "queued".to_owned(),
        attempts: 0,
        claimed_at: None,
        executing_at: None,
        finished_at: None,
        outcome: None,
        failure_reason: None,
    }
}

#[test]
fn a_finished_receipt_carries_every_field_in_order() {
    let receipt = FleetCommandReceipt {
        executor_kind: "mod_runtime".to_owned(),
        action: "kick".to_owned(),
        arguments: json!({ "arma_id": "a-1", "reason": "afk" }),
        state: "failed".to_owned(),
        attempts: 2,
        claimed_at: Some(at(1, 0)),
        executing_at: Some(at(2, 123_456_789)),
        finished_at: Some(at(3, 500_000_000)),
        outcome: Some(json!({ "players": [] })),
        failure_reason: Some("the player left".to_owned()),
        ..queued_receipt()
    };
    let golden = concat!(
        r#"{"id":"6f1c2b9e-3a4d-4e5f-8a7b-9c0d1e2f3a4b","#,
        r#""server_id":"0e9d8c7b-6a5f-4e3d-2c1b-0a9f8e7d6c5b","#,
        r#""executor_kind":"mod_runtime","action":"kick","#,
        r#""arguments":{"arma_id":"a-1","reason":"afk"},"#,
        r#""requested_by":"123456789012345678","requested_at":"2026-09-23T12:00:00.25Z","#,
        r#""expires_at":"2026-09-23T12:05:00Z","state":"failed","attempts":2,"#,
        r#""claimed_at":"2026-09-23T12:00:01Z","executing_at":"2026-09-23T12:00:02.123456789Z","#,
        r#""finished_at":"2026-09-23T12:00:03.5Z","outcome":{"players":[]},"#,
        r#""failure_reason":"the player left"}"#,
    );
    assert_eq!(serde_json::to_string(&receipt).unwrap(), golden);
    let read: FleetCommandReceipt = serde_json::from_str(golden).unwrap();
    assert_eq!(read, receipt);
    assert_eq!(serde_json::to_string(&read).unwrap(), golden);
}

#[test]
fn a_request_reads_its_action_and_arguments() {
    let golden = r#"{"action":"broadcast","arguments":{"message":"Restart in 5"}}"#;
    let request: FleetCommandRequest = serde_json::from_str(golden).unwrap();
    assert_eq!(request.action, FleetAction::Broadcast);
    assert_eq!(
        request.arguments.get("message"),
        Some(&json!("Restart in 5"))
    );
    assert_eq!(serde_json::to_string(&request).unwrap(), golden);
}

#[test]
fn a_request_refuses_unknown_keys_and_actions() {
    for invalid in [
        r#"{"action":"start","force":true}"#,
        r#"{"action":"shutdown"}"#,
        r#"{"arguments":{}}"#,
        r#"{"action":"start","arguments":[]}"#,
    ] {
        assert!(
            serde_json::from_str::<FleetCommandRequest>(invalid).is_err(),
            "{invalid}"
        );
    }
}
