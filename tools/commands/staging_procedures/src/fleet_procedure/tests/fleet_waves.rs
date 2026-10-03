//! The fleet waves W1–W8 on the fake clock against recorded observer answers: a recorded run
//! holds every case in time, and each planted defect fails only the cases it touches, naming
//! what was observed. W9–W14's defects are in `single_server_waves.rs`, which runs through
//! [`run`].
use super::judge_mapping::{client_count, judge_scenarios, server_ids};
use super::recorded_fleet::{INSTANCES, RecordedFleet, T0, server_id};
use super::{FleetProcedure, fleet_cases};
use crate::observation_journal::browser_inbox::BrowserInbox;
use crate::observation_journal::journal::ObservationJournal;
use crate::procedure_runner::fake_clock::FakeClock;
use crate::procedure_runner::procedure::{ProcedureRun, StagingProcedure};
use crate::procedure_runner::runner::RunContext;
use crate::procedure_runner::runner_support::{scratch_folder, test_settings};
use api_readiness_checks::operational_recording::CaseStatus;

/// Runs the fleet plan against `recorded`, with its browser inbox entries saved, and returns the
/// run and the printed lines.
pub(super) fn run(recorded: &RecordedFleet) -> (ProcedureRun, String) {
    let settings = test_settings();
    let plan = FleetProcedure.plan(&settings).unwrap();
    plan.validate().unwrap();
    let clock = FakeClock::starting_at(T0);
    let mut host = recorded.host(&clock);
    let folder = scratch_folder("fleet-waves");
    let mut journal = ObservationJournal::create(&folder).unwrap();
    recorded
        .single_server
        .write_inbox(&folder.join("browser_inbox"));
    let inbox = BrowserInbox::new(&folder.join("browser_inbox"));
    let mut output = Vec::new();
    let run = FleetProcedure
        .run(
            &plan,
            RunContext {
                host: &mut host,
                clock: &clock,
                journal: &mut journal,
                inbox: &inbox,
                output: &mut output,
            },
        )
        .unwrap();
    let _ = std::fs::remove_dir_all(&folder);
    (run, String::from_utf8(output).unwrap())
}

pub(super) fn status(run: &ProcedureRun, case: &str) -> CaseStatus {
    run.cases
        .iter()
        .find(|recorded| recorded.name.as_str() == case)
        .unwrap_or_else(|| panic!("case {case} is not declared"))
        .status
        .clone()
}

pub(super) fn failure(run: &ProcedureRun, case: &str) -> String {
    match status(run, case) {
        CaseStatus::Failed(why) => why,
        other => panic!("{case} is {other:?}, not failed"),
    }
}

/// Every declared case but `failed` is ok.
pub(super) fn only_failed(run: &ProcedureRun, failed: &[&str]) {
    for case in &run.cases {
        let expected_failure = failed.contains(&case.name.as_str());
        assert_eq!(
            matches!(case.status, CaseStatus::Failed(_)),
            expected_failure,
            "{} is {:?}",
            case.name.as_str(),
            case.status
        );
    }
}

#[test]
fn staging_fleet_waves_hold_every_case_of_a_recorded_run_in_time() {
    let recorded = RecordedFleet {
        listed_arma_ids: [vec!["arma-operator"], vec![], vec![], vec![], vec![]],
        ..RecordedFleet::default()
    };
    let (run, output) = run(&recorded);
    assert_eq!(run.cases.len(), 50);
    for case in &run.cases {
        let expected = match case.name.as_str() {
            "kick_targets_one_of_two_clients" | "same_terrain_carries_two_clients" => {
                CaseStatus::NotRun {
                    missing: "second game client".into(),
                }
            }
            _ => CaseStatus::Ok,
        };
        assert_eq!(case.status, expected, "{}", case.name.as_str());
    }
    let awaits: Vec<&str> = output
        .lines()
        .filter_map(|line| line.strip_prefix("AWAIT "))
        .map(|line| line.split(':').next().unwrap())
        .collect();
    assert_eq!(
        awaits,
        [
            "w1_stop",
            "w2_start",
            "w3_restart",
            "w4_console_command",
            "w5_list_players",
            "w6_same_terrain",
            "w7_cross_terrain",
            "w8_return_to_origin",
            "w9_identity_link",
            "w10_kick",
            "w10_ended_session_kick",
            "w11_stage_host_agent_credential",
            "w11_revoke_host_agent_credential",
            "w11_promote_host_agent_credential",
            "w12_stage_mod_runtime_credential",
            "w12_revoke_mod_runtime_credential",
            "w12_promote_mod_runtime_credential",
            "w13_lost_claim_answer",
            "w14_lost_result_answer"
        ]
    );
    assert!(output.contains("in Server Control, Stop on TBD Staging 1, 2, 3, 4 and 5"));
    assert_eq!(
        server_ids(&run.measurements),
        INSTANCES.map(server_id).to_vec()
    );
    assert_eq!(client_count(&run.measurements), 1);
    assert_eq!(
        judge_scenarios(&run.cases),
        [
            "stop",
            "start",
            "restart",
            "custom_console",
            "same_terrain",
            "cross_terrain",
            "kick",
            "identity_link",
            "lost_acknowledgement"
        ]
    );
    for label in [
        "w1_stop.server1_unit",
        "w2_start.server5_succession",
        "w4_console_command.server3_command",
        "w5_list_players.server1_command",
        "w6_same_terrain.server2_process",
        "w7_cross_terrain.server4_config",
        "w8_return_to_origin.server5_confirmed",
        "w9_identity_link.link_status",
        "w10_ended_session_kick.refused",
        "w11_revoke_host_agent_credential.old_refused",
        "w12_promote_mod_runtime_credential.new_generation",
        "w13_lost_claim_answer.ledger",
        "w14_lost_result_answer.retry_refused",
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
fn staging_fleet_stop_late_or_before_the_step_fails_only_that_server() {
    let (run, _) = run(&RecordedFleet {
        late_stop: vec![3],
        stop_before_the_step: vec![4],
        ..RecordedFleet::default()
    });
    only_failed(&run, &["server3_stop", "server4_stop"]);
    let why = failure(&run, "server3_stop");
    assert!(
        why.contains("w1_stop.server3_command") && why.contains("after its 150 s deadline"),
        "{why}"
    );
    let earlier = failure(&run, "server4_stop");
    assert!(
        earlier.contains("its deadline passed")
            && earlier.contains("no stop command for TBD Staging 4 yet"),
        "{earlier}"
    );
    assert!(!judge_scenarios(&run.cases).contains(&"stop".to_string()));
}

#[test]
fn staging_fleet_succession_accepts_any_recorded_end_and_refuses_a_gap_or_two_open_sessions() {
    let (run, _) = run(&RecordedFleet {
        expired_before_start: vec![2],
        generation_gap_on_start: vec![3],
        two_open_sessions_on_restart: vec![4],
        ..RecordedFleet::default()
    });
    only_failed(
        &run,
        &[
            "server3_runtime_session_succession",
            "server4_runtime_session_succession",
        ],
    );
    let gap = failure(&run, "server3_runtime_session_succession");
    assert!(
        gap.contains("generation 3 follows generation 1, not generation 2"),
        "{gap}"
    );
    let open = failure(&run, "server4_runtime_session_succession");
    assert!(
        open.contains("TBD Staging 4 has 2 open runtime sessions"),
        "{open}"
    );
}

#[test]
fn staging_fleet_same_terrain_refuses_a_process_restart_and_a_changed_process() {
    let (run, _) = run(&RecordedFleet {
        host_restart_on_same_terrain: vec![2],
        process_changed_on_same_terrain: vec![4],
        ..RecordedFleet::default()
    });
    // A process that changed unexpectedly leaves the later waves nothing to compare with, and
    // they say so rather than guess.
    only_failed(
        &run,
        &[
            "server2_same_terrain",
            "server4_same_terrain",
            "server4_cross_terrain",
            fleet_cases::RETURN_TO_ORIGIN_TERRAIN,
        ],
    );
    let transition = failure(&run, "server2_same_terrain");
    assert!(
        transition.contains("is a host_restart, not a scenario_restart")
            && transition.contains("runs no everon artifact"),
        "{transition}"
    );
    let process = failure(&run, "server4_same_terrain");
    assert!(
        process.contains("runs PID 6004, not PID 3004 that w3_restart measured"),
        "{process}"
    );
    let cascade = failure(&run, "server4_cross_terrain");
    assert!(
        cascade.contains("w6_same_terrain measured no process of TBD Staging 4"),
        "{cascade}"
    );
    assert!(!judge_scenarios(&run.cases).contains(&"same_terrain".to_string()));
}

#[test]
fn staging_fleet_cross_terrain_refuses_a_confirmation_by_the_earlier_session() {
    let (run, _) = run(&RecordedFleet {
        stale_confirmation_across_terrain: vec![5],
        ..RecordedFleet::default()
    });
    only_failed(&run, &["server5_cross_terrain"]);
    let why = failure(&run, "server5_cross_terrain");
    assert!(
        why.contains("a session started before the deployment was requested"),
        "{why}"
    );
    assert_eq!(
        status(&run, fleet_cases::RETURN_TO_ORIGIN_TERRAIN),
        CaseStatus::Ok
    );
}
