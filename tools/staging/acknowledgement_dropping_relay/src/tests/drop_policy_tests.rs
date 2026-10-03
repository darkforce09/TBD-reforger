//! The drop policy: which exchanges it recognises, what one arming withholds, and the status.

use std::time::Duration;

use axum::http::{Method, StatusCode};
use clap::ValueEnum;
use serde_json::{Value, json};

use super::{
    Arming, Decision, DropPolicy, DropTarget, ExecutorExchange, ExecutorResponse, FleetCommandId,
    RelayIdentity,
};

const COMMAND: &str = "7a1f0c9e-3b2d-4e5f-8a6b-1c2d3e4f5a6b";
const CLAIM: &str = "/api/v1/fleet-executor/commands/claim";

fn policy() -> DropPolicy {
    DropPolicy::new(RelayIdentity {
        listen: "127.0.0.1:18085".to_string(),
        upstream: "http://127.0.0.1:8080".to_string(),
        withhold: Duration::from_secs(30),
    })
}

fn claim_body(fencing_token: i64) -> Vec<u8> {
    json!({ "command_id": COMMAND, "server_id": "s", "action": "restart", "arguments": {},
            "fencing_token": fencing_token, "lease_expires_at": "2026-09-29T12:00:00Z" })
    .to_string()
    .into_bytes()
}

fn result_exchange() -> ExecutorExchange {
    ExecutorExchange::Result {
        command_id: COMMAND.to_string(),
    }
}

#[test]
fn only_posted_claims_and_result_reports_are_executor_exchanges() {
    let result = format!("/api/v1/fleet-executor/commands/{COMMAND}/result");
    assert_eq!(
        ExecutorExchange::classify(&Method::POST, CLAIM),
        Some(ExecutorExchange::Claim)
    );
    assert_eq!(
        ExecutorExchange::classify(&Method::POST, &result),
        Some(result_exchange())
    );
    for (method, path) in [
        (Method::GET, CLAIM.to_string()),
        (Method::PUT, result.clone()),
        (
            Method::POST,
            format!("/api/v1/fleet-executor/commands/{COMMAND}/executing"),
        ),
        (
            Method::POST,
            "/api/v1/fleet-executor/commands//result".to_string(),
        ),
        (
            Method::POST,
            "/api/v1/fleet-executor/commands/a/b/result".to_string(),
        ),
        (Method::POST, format!("{CLAIM}/extra")),
        (Method::POST, "/api/v1/servers/claim".to_string()),
    ] {
        assert_eq!(
            ExecutorExchange::classify(&method, &path),
            None,
            "{method} {path}"
        );
    }
}

#[test]
fn a_disarmed_policy_forwards_every_answer() {
    let policy = policy();
    for (exchange, status) in [
        (Some(ExecutorExchange::Claim), StatusCode::OK),
        (Some(result_exchange()), StatusCode::OK),
        (None, StatusCode::OK),
    ] {
        let decision = policy.decide(exchange.as_ref(), status, b"{}", &claim_body(1));
        assert_eq!(decision, Decision::Forward);
    }
    let status = policy.status();
    assert_eq!((status.forwarded_count, status.drop_count), (3, 0));
}

#[test]
fn an_armed_claim_drop_spends_itself_on_the_first_200_claim_answer_only() {
    let policy = policy();
    policy.arm(DropTarget::DropNextClaimResponse);
    let claim = Some(ExecutorExchange::Claim);
    let passing = [
        (claim.clone(), StatusCode::NO_CONTENT),
        (claim.clone(), StatusCode::SERVICE_UNAVAILABLE),
        (Some(result_exchange()), StatusCode::OK),
        (None, StatusCode::OK),
    ];
    for (exchange, status) in passing {
        let decision = policy.decide(exchange.as_ref(), status, b"{}", &claim_body(1));
        assert_eq!(decision, Decision::Forward, "{exchange:?} {status}");
    }
    assert_eq!(policy.status().arming, Arming::DropNextClaimResponse);

    let Decision::Withhold(record) =
        policy.decide(claim.as_ref(), StatusCode::OK, b"{}", &claim_body(3))
    else {
        panic!("the 200 claim answer is withheld");
    };
    assert_eq!(record.response, ExecutorResponse::Claim);
    assert_eq!(
        record.command_id.as_ref().map(FleetCommandId::as_str),
        Some(COMMAND)
    );
    assert_eq!(record.fencing_token, Some(3));
    assert_eq!(record.upstream_status, 200);
    assert!(record.withheld_at_unix_ms > 0);
    let again = policy.decide(claim.as_ref(), StatusCode::OK, b"{}", &claim_body(4));
    assert_eq!(again, Decision::Forward, "one arming withholds one answer");
    let status = policy.status();
    assert_eq!(status.arming, Arming::Disarmed);
    assert_eq!((status.forwarded_count, status.drop_count), (5, 1));
    assert_eq!(status.last_drop, Some(record));
}

#[test]
fn an_armed_result_drop_takes_the_command_from_the_path_and_the_token_from_the_report() {
    let policy = policy();
    policy.arm(DropTarget::DropNextResultResponse);
    let report = br#"{"fencing_token":2,"succeeded":true}"#;
    let exchange = Some(result_exchange());
    let conflict = policy.decide(exchange.as_ref(), StatusCode::CONFLICT, report, b"{}");
    assert_eq!(conflict, Decision::Forward);
    let claim = policy.decide(
        Some(&ExecutorExchange::Claim),
        StatusCode::OK,
        b"{}",
        &claim_body(1),
    );
    assert_eq!(claim, Decision::Forward);
    let Decision::Withhold(record) =
        policy.decide(exchange.as_ref(), StatusCode::OK, report, b"{}")
    else {
        panic!("the 200 result answer is withheld");
    };
    assert_eq!(record.response, ExecutorResponse::Result);
    assert_eq!(
        record.command_id.as_ref().map(FleetCommandId::as_str),
        Some(COMMAND)
    );
    assert_eq!(record.fencing_token, Some(2));
}

#[test]
fn an_unreadable_claim_answer_is_still_withheld_and_names_no_command() {
    let policy = policy();
    policy.arm(DropTarget::DropNextClaimResponse);
    let Decision::Withhold(record) = policy.decide(
        Some(&ExecutorExchange::Claim),
        StatusCode::OK,
        b"{}",
        b"not json",
    ) else {
        panic!("a 200 claim answer is withheld whatever its body");
    };
    assert_eq!((record.command_id, record.fencing_token), (None, None));
}

#[test]
fn arming_replaces_and_disarming_clears() {
    let policy = policy();
    assert_eq!(
        policy.arm(DropTarget::DropNextClaimResponse).arming,
        Arming::DropNextClaimResponse
    );
    assert_eq!(
        policy.arm(DropTarget::DropNextResultResponse).arming,
        Arming::DropNextResultResponse
    );
    assert_eq!(policy.disarm().arming, Arming::Disarmed);
}

#[test]
fn the_target_words_agree_across_the_socket_the_command_line_and_the_status() {
    for target in DropTarget::value_variants() {
        let clap_name = target
            .to_possible_value()
            .expect("named")
            .get_name()
            .to_string();
        assert_eq!(clap_name, target.word());
        assert_eq!(DropTarget::from_word(target.word()), Some(*target));
        assert_eq!(serde_json::to_value(target).unwrap(), json!(target.word()));
        let arming = serde_json::to_value(Arming::from(Some(*target))).unwrap();
        assert_eq!(arming, json!(target.word()));
    }
    assert_eq!(DropTarget::from_word("drop-everything"), None);
}

#[test]
fn the_status_document_names_its_fields_for_the_harness() {
    let policy = policy();
    policy.arm(DropTarget::DropNextClaimResponse);
    policy.decide(
        Some(&ExecutorExchange::Claim),
        StatusCode::OK,
        b"{}",
        &claim_body(1),
    );
    let document: Value = serde_json::to_value(policy.status()).unwrap();
    assert_eq!(document["arming"], "disarmed");
    assert_eq!(document["listen"], "127.0.0.1:18085");
    assert_eq!(document["upstream"], "http://127.0.0.1:8080");
    assert_eq!(document["withhold_milliseconds"], 30_000);
    assert_eq!(document["forwarded_count"], 0);
    assert_eq!(document["drop_count"], 1);
    assert_eq!(document["last_drop"]["response"], "claim");
    assert_eq!(document["last_drop"]["command_id"], COMMAND);
    assert_eq!(document["last_drop"]["fencing_token"], 1);
    assert_eq!(document["last_drop"]["upstream_status"], 200);
    assert!(
        document["last_drop"]["withheld_at_unix_ms"]
            .as_u64()
            .unwrap()
            > 0
    );
}
