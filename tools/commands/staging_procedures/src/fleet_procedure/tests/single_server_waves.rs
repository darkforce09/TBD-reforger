//! The single-server waves W9–W14 on the fake clock against recorded observer answers: the
//! recorded run holds the identity link, both kicks, both rotations and both lost answers, and
//! each planted defect (a linked id the listing lacks, a kick not refused, an old credential
//! still accepted, a fencing token that is not 2, a second unit start, a result accepted twice)
//! fails only the cases it touches, naming what was observed.
use serde_json::json;

use super::fleet_cases::{
    IDENTITY_LINK, KICK, LOST_ACKNOWLEDGEMENT_CLAIM_RESPONSE, LOST_ACKNOWLEDGEMENT_RESULT_RESPONSE,
    MACHINE_CREDENTIAL_ROTATION_HOST_AGENT, REJECTION_ENDED_SESSION_KICK, SINGLE_SERVER_CASES,
};
use super::fleet_waves_tests::{failure, only_failed, run, status};
use super::judge_mapping::judge_scenarios;
use super::recorded_fleet::RecordedFleet;
use super::recorded_single_server::{OPERATOR_ARMA_ID, RecordedSingleServer};
use api_readiness_checks::operational_recording::CaseStatus;

/// A recorded fleet run with `single_server`'s W9–W14.
fn recorded(single_server: RecordedSingleServer) -> RecordedFleet {
    RecordedFleet {
        listed_arma_ids: [vec![OPERATOR_ARMA_ID], vec![], vec![], vec![], vec![]],
        single_server,
        ..RecordedFleet::default()
    }
}

#[test]
fn staging_fleet_single_server_waves_hold_every_case_of_a_recorded_run() {
    let (run, output) = run(&recorded(RecordedSingleServer::default()));
    for case in SINGLE_SERVER_CASES {
        assert_eq!(status(&run, case), CaseStatus::Ok, "{case}");
    }
    assert_eq!(
        run.measurements["w9_identity_link.linked_arma_id"],
        json!(OPERATOR_ARMA_ID)
    );
    assert_eq!(
        run.measurements["player_listing.w9_identity_link.server1"],
        json!(1)
    );
    assert_eq!(
        run.measurements["w10_kick.kicked_arma_id"],
        json!(OPERATOR_ARMA_ID)
    );
    assert_eq!(
        run.measurements["w11_stage_host_agent_credential.previous_credential"],
        json!("host-agent-old")
    );
    assert_eq!(
        run.measurements["w12_revoke_mod_runtime_credential.revoked_generation"],
        json!(8)
    );
    assert_eq!(
        run.measurements["w13_lost_claim_answer.command_id"],
        json!("w13-restart-5")
    );
    for line in [
        "AWAIT w9_identity_link: W9: the operator joins TBD Staging 1;",
        "AWAIT w11_stage_host_agent_credential: W11: the harness stages a new host_agent \
         credential of TBD Staging 1",
        "AWAIT w12_promote_mod_runtime_credential: W12: the harness promotes the staged \
         mod_runtime credential of TBD Staging 2",
        "AWAIT w13_lost_claim_answer: W13: the harness arms the relay of TBD Staging 5",
        "w10_ended_session_kick.refused ok",
        "w13_lost_claim_answer.ledger ok",
        "w14_lost_result_answer.single_start ok",
    ] {
        assert!(output.contains(line), "no {line:?} in\n{output}");
    }
    for label in [
        "w9_identity_link.listed_identity",
        "w10_kick.mod_log",
        "w12_revoke_mod_runtime_credential.session_revoked",
        "w13_lost_claim_answer.relay_drop",
        "w14_lost_result_answer.ledger",
    ] {
        assert!(
            run.journal
                .iter()
                .any(|record| format!("{record:?}").contains(label)),
            "no deciding observation for {label}"
        );
    }
}

#[test]
fn staging_fleet_identity_link_refuses_a_linked_id_the_listing_does_not_show() {
    let (run, _) = run(&recorded(RecordedSingleServer {
        linked_id_mismatch: true,
        ..RecordedSingleServer::default()
    }));
    only_failed(&run, &[IDENTITY_LINK, KICK]);
    let link = failure(&run, IDENTITY_LINK);
    assert!(
        link.contains("the listing names [\"arma-operator\"], not the linked Arma id arma-other"),
        "{link}"
    );
    let kick = failure(&run, KICK);
    assert!(
        kick.contains("the kick removed Arma id arma-operator, not the linked Arma id arma-other"),
        "{kick}"
    );
    let scenarios = judge_scenarios(&run.cases);
    assert!(!scenarios.contains(&"identity_link".to_string()));
    assert!(!scenarios.contains(&"kick".to_string()));
}

#[test]
fn staging_fleet_ended_session_kick_must_be_refused() {
    let (run, _) = run(&recorded(RecordedSingleServer {
        ended_session_kick_accepted: true,
        ..RecordedSingleServer::default()
    }));
    only_failed(&run, &[REJECTION_ENDED_SESSION_KICK]);
    let why = failure(&run, REJECTION_ENDED_SESSION_KICK);
    assert!(
        why.contains("the saved answer is not the 409 RUNTIME_SESSION_ENDED refusal")
            && why.contains(
                "kick command w10-kick-2 against runtime session session-w7-1 was \
                             accepted (queued)"
            ),
        "{why}"
    );
}

#[test]
fn staging_fleet_game_server_host_agent_rotation_fails_while_the_old_credential_is_still_accepted()
{
    let (run, _) = run(&recorded(RecordedSingleServer {
        old_credential_still_accepted: true,
        ..RecordedSingleServer::default()
    }));
    only_failed(&run, &[MACHINE_CREDENTIAL_ROTATION_HOST_AGENT]);
    let why = failure(&run, MACHINE_CREDENTIAL_ROTATION_HOST_AGENT);
    assert!(
        why.contains("no claim answered 401 `machine credential revoked`")
            && why.contains("the revoked credential host-agent-old authenticated at"),
        "{why}"
    );
}

#[test]
fn staging_fleet_lost_claim_answer_needs_fencing_token_two() {
    let (run, _) = run(&recorded(RecordedSingleServer {
        requeue_keeps_first_token: true,
        ..RecordedSingleServer::default()
    }));
    only_failed(&run, &[LOST_ACKNOWLEDGEMENT_CLAIM_RESPONSE]);
    let why = failure(&run, LOST_ACKNOWLEDGEMENT_CLAIM_RESPONSE);
    assert!(
        why.contains("restart command w13-restart-5 succeeded under fencing token 1, not 2"),
        "{why}"
    );
    assert!(!judge_scenarios(&run.cases).contains(&"lost_acknowledgement".to_string()));
}

#[test]
fn staging_fleet_lost_answers_refuse_a_second_unit_start() {
    let (run, _) = run(&recorded(RecordedSingleServer {
        double_unit_start: true,
        ..RecordedSingleServer::default()
    }));
    only_failed(
        &run,
        &[
            LOST_ACKNOWLEDGEMENT_CLAIM_RESPONSE,
            LOST_ACKNOWLEDGEMENT_RESULT_RESPONSE,
        ],
    );
    for case in [
        LOST_ACKNOWLEDGEMENT_CLAIM_RESPONSE,
        LOST_ACKNOWLEDGEMENT_RESULT_RESPONSE,
    ] {
        let why = failure(&run, case);
        assert!(
            why.contains("single_start") && why.contains("tbd-reforger@5.service started 2 times"),
            "{why}"
        );
    }
}

#[test]
fn staging_fleet_lost_result_answer_refuses_a_result_accepted_twice() {
    let (run, _) = run(&recorded(RecordedSingleServer {
        result_accepted_twice: true,
        ..RecordedSingleServer::default()
    }));
    only_failed(&run, &[LOST_ACKNOWLEDGEMENT_RESULT_RESPONSE]);
    let why = failure(&run, LOST_ACKNOWLEDGEMENT_RESULT_RESPONSE);
    assert!(
        why.contains("retry_refused") && why.contains("the API accepted a result report at"),
        "{why}"
    );
    assert_eq!(
        status(&run, LOST_ACKNOWLEDGEMENT_CLAIM_RESPONSE),
        CaseStatus::Ok
    );
}
