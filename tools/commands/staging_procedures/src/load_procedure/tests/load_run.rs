//! The recorded load run against recorded host answers and a stub workstation, and the
//! `seed-load` and `clean-load` actions.

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::Arc;

use super::LoadProcedure;
use super::load_fixture_orchestration::{clean_load, seed_load};
use super::load_run::{LoadRunInputs, run_load};
use super::load_steps::{LOAD_CASES, load_plan};
use super::load_test_support::{
    START_UNIX_MS, StubWorkstation, account_file_text, census_rows, committed, exposition,
    load_settings, passing_report, repository_root, seeded_host,
};
use super::workstation_load::WorkstationLoad;
use crate::observation_journal::browser_inbox::BrowserInbox;
use crate::observation_journal::journal::ObservationJournal;
use crate::procedure_runner::fake_clock::FakeClock;
use crate::procedure_runner::procedure::{ProcedureRun, StagingProcedure};
use crate::procedure_runner::runner::RunContext;
use crate::procedure_runner::runner_support::{ScriptedHost, scratch_folder};
use crate::staging_command::PlanOnly;
use api_readiness_checks::operational_recording::{CaseStatus, FixtureManifest, Observations};

struct Recorded {
    run: ProcedureRun,
    output: String,
    stub: Arc<StubWorkstation>,
}

fn record(host: &mut ScriptedHost, clock: &FakeClock, stub: StubWorkstation) -> Recorded {
    let data = committed();
    let settings = load_settings();
    let plan = load_plan(&data, &settings).unwrap();
    plan.validate().unwrap();
    let folder = scratch_folder("load-run");
    let token_file = folder.join("load_tokens");
    std::fs::write(&token_file, account_file_text(1_100)).unwrap();
    let mut journal = ObservationJournal::create(&folder).unwrap();
    let inbox = BrowserInbox::new(&folder.join("browser_inbox"));
    let stub = Arc::new(stub);
    let workstation: Arc<dyn WorkstationLoad> = stub.clone();
    let mut output = Vec::new();
    let context = RunContext {
        host,
        clock,
        journal: &mut journal,
        inbox: &inbox,
        output: &mut output,
    };
    let inputs = LoadRunInputs {
        data: &data,
        settings: &settings,
        token_file: &token_file,
        workstation: &workstation,
    };
    let run = run_load(&plan, context, &inputs).unwrap();
    Recorded {
        run,
        output: String::from_utf8(output).unwrap(),
        stub,
    }
}

fn statuses(run: &ProcedureRun) -> BTreeMap<String, CaseStatus> {
    run.cases
        .iter()
        .map(|case| (case.name.as_str().to_string(), case.status.clone()))
        .collect()
}

fn failed(run: &ProcedureRun) -> Vec<String> {
    statuses(run)
        .into_iter()
        .filter(|(_, status)| matches!(status, CaseStatus::Failed(_)))
        .map(|(name, _)| name)
        .collect()
}

fn reason(run: &ProcedureRun, case: &str) -> String {
    match &statuses(run)[case] {
        CaseStatus::Failed(why) => why.clone(),
        other => panic!("{case} is {other:?}"),
    }
}

#[test]
fn staging_load_run_passes_every_case_against_a_seeded_host_and_a_passing_engine() {
    let clock = FakeClock::starting_at(START_UNIX_MS);
    let mut host = seeded_host(&clock);
    let recorded = record(
        &mut host,
        &clock,
        StubWorkstation::answering(Ok(passing_report(1_800.0))),
    );
    assert_eq!(
        failed(&recorded.run),
        Vec::<String>::new(),
        "{}",
        recorded.output
    );
    assert_eq!(recorded.run.cases.len(), LOAD_CASES.len());
    for step in [
        "population",
        "keying_probe",
        "game_operations_baseline",
        "member_load",
        "game_operations_delta",
    ] {
        assert!(
            recorded.output.contains(&format!("AWAIT {step}:")),
            "{}",
            recorded.output
        );
    }
    let plans = recorded.stub.plans.lock().unwrap();
    let plan = &plans[0];
    assert_eq!(plan.source_addresses.len(), 5);
    assert_eq!(plan.target_origin, "http://192.0.2.10:3080");
    assert_eq!(plan.fixture_events.len(), 10);
    assert_eq!(plan.fixture_events[3].slot_ids.len(), 128);
    assert_eq!(plan.workload.measured_seconds, 1_800.0);
    assert!(recorded.run.journal.len() >= LOAD_CASES.len());
    assert!(!recorded.output.contains("not-a-token"));
    let census = &recorded.run.measurements["heartbeat_census"]["samples"];
    assert!(census.as_array().unwrap().len() >= 2);
    let procedure = LoadProcedure {
        repository_root: Some(repository_root()),
        ..LoadProcedure::default()
    };
    let manifest = FixtureManifest::new(&serde_json::json!({ "check": "staging_load" })).unwrap();
    match procedure.observations(&recorded.run, &manifest) {
        Observations::Load {
            duration_seconds,
            completed_requests,
            minimum_concurrent_clients,
            workload_sha256,
            hardware,
            p95_json_read_ms,
            ..
        } => {
            assert_eq!(duration_seconds, 1_800);
            assert_eq!(completed_requests, 48_600);
            assert_eq!(minimum_concurrent_clients, 100);
            assert_eq!(workload_sha256, committed().workload_sha256);
            assert!(!hardware.is_empty());
            assert_eq!(p95_json_read_ms, 120.0);
        }
        other => panic!("not a load observation: {other:?}"),
    }
}

#[test]
fn staging_load_run_fails_the_cases_a_short_report_misses() {
    let clock = FakeClock::starting_at(START_UNIX_MS);
    let mut host = seeded_host(&clock);
    let mut report = passing_report(1_800.0);
    report.member_accounts = 900;
    report.json_reads.p95_milliseconds = Some(500.1);
    report.minimum_concurrent_clients = 99;
    let recorded = record(&mut host, &clock, StubWorkstation::answering(Ok(report)));
    assert_eq!(
        failed(&recorded.run),
        vec!["concurrency", "member_accounts", "p95_json_reads"]
    );
    assert!(reason(&recorded.run, "member_accounts").contains("900 member accounts"));
}

#[test]
fn staging_load_run_names_the_engine_failure_in_every_case_the_report_decides() {
    let clock = FakeClock::starting_at(START_UNIX_MS);
    let mut host = seeded_host(&clock);
    let recorded = record(
        &mut host,
        &clock,
        StubWorkstation::answering(Err("the account file is short".into())),
    );
    let expected = [
        "concurrency",
        "member_accounts",
        "p95_json_reads",
        "p95_json_writes",
        "refresh_paced",
        "sustained_rate",
        "zero_unexpected_errors",
    ];
    assert_eq!(failed(&recorded.run), expected);
    assert!(reason(&recorded.run, "sustained_rate").contains("the account file is short"));
    let procedure = LoadProcedure {
        repository_root: Some(repository_root()),
        ..LoadProcedure::default()
    };
    let manifest = FixtureManifest::new(&serde_json::json!({ "check": "staging_load" })).unwrap();
    match procedure.observations(&recorded.run, &manifest) {
        Observations::Load {
            completed_requests,
            p95_json_read_ms,
            ..
        } => {
            assert_eq!(completed_requests, 0);
            assert_eq!(p95_json_read_ms, f64::MAX);
        }
        other => panic!("not a load observation: {other:?}"),
    }
}

#[test]
fn staging_load_run_fails_population_keying_heartbeats_and_game_operations_on_bad_answers() {
    let clock = FakeClock::starting_at(START_UNIX_MS);
    let broken = exposition(700, 670).replace(
        "tbd_http_requests_total{method=\"GET\"",
        "tbd_http_requests_total{method=\"POST\",route=\"/api/v1/ingest/matches\",status=\"503\"} 2\ntbd_http_requests_total{method=\"GET\"",
    );
    let mut host = seeded_host(&clock)
        .answer(
            "FROM users",
            0,
            0,
            "900|900|9100000000000000000|9100000000000000899\n",
        )
        .answer("server_runtime_sessions", 0, 0, &census_rows(120_000))
        .answer("/metrics", START_UNIX_MS + 1_000, 0, &broken);
    let mut stub = StubWorkstation::answering(Ok(passing_report(1_800.0)));
    stub.keying_status = 429;
    let recorded = record(&mut host, &clock, stub);
    assert_eq!(
        failed(&recorded.run),
        vec![
            "game_operations_measured",
            "game_servers_heartbeating",
            "population_seeded",
            "refresh_paced"
        ]
    );
    assert!(reason(&recorded.run, "population_seeded").contains("900 synthetic accounts"));
    assert!(reason(&recorded.run, "refresh_paced").contains("429"));
    assert!(reason(&recorded.run, "game_servers_heartbeating").contains("saw 0 of 5"));
    assert!(reason(&recorded.run, "game_operations_measured").contains("5xx"));
}

#[test]
fn staging_load_plan_declares_ten_cases_and_lists_its_actions_and_recovery() {
    let procedure = LoadProcedure {
        repository_root: Some(repository_root()),
        token_file: Some("/scratch/load_tokens".into()),
        ..LoadProcedure::default()
    };
    let settings = load_settings();
    let plan = procedure.plan(&settings).unwrap();
    plan.validate().unwrap();
    let names: Vec<&str> = plan
        .declared_cases
        .iter()
        .map(|case| case.name.as_str())
        .collect();
    assert_eq!(names, LOAD_CASES);
    assert_eq!(plan.hard_stop_seconds, 60 + 1_800 + 1_200);
    let definition = plan.definition();
    assert_eq!(
        definition["steps"][3]["effects"][0]["source"],
        "measurement:load_report"
    );
    assert_eq!(definition["steps"][0]["effects"][0]["source"], "host");
    let actions = procedure.action_list(&settings);
    let text: Vec<String> = actions
        .iter()
        .flat_map(|action| action.details.clone())
        .collect();
    assert!(
        text.iter()
            .any(|line| line == "cargo xtask staging seed-load --token-file /scratch/load_tokens")
    );
    assert!(
        text.iter()
            .any(|line| line
                == "cargo xtask staging load --record --token-file /scratch/load_tokens")
    );
    let recovery = procedure.recovery_action_list(&settings);
    assert_eq!(recovery.len(), 2);
    assert_eq!(recovery[0].details, vec!["cargo xtask staging clean-load"]);
    assert_eq!(recovery[1].details, vec!["rm -f /scratch/load_tokens"]);
}

#[test]
fn staging_seed_load_dry_run_prints_the_plan_and_opens_no_connection() {
    let clock = FakeClock::starting_at(START_UNIX_MS);
    let mut host = ScriptedHost::new(&clock);
    let mut output = Vec::new();
    let token_file = scratch_folder("seed-dry").join("load_tokens");
    let code = seed_load(
        &load_settings(),
        &committed(),
        &token_file,
        &PlanOnly { dry_run: true },
        &mut host,
        &mut output,
    )
    .unwrap();
    let text = String::from_utf8(output).unwrap();
    assert_eq!(code, 0);
    assert!(host.calls.is_empty());
    assert!(
        text.contains("'seed-load-population' '--accounts' '1100' '--role' 'Player'"),
        "{text}"
    );
    assert!(
        text.contains("'--mission' '<id of that mission>'"),
        "{text}"
    );
    assert!(!token_file.exists());
}

#[test]
fn staging_seed_load_moves_the_account_file_into_a_new_mode_600_token_file() {
    let clock = FakeClock::starting_at(START_UNIX_MS);
    let accounts = account_file_text(1_100);
    let mut host = ScriptedHost::new(&clock)
        .answer("FROM missions WHERE title", 0, 0, "mission-1\n")
        .answer("seed-load-population", 0, 0, "seeded 1100 accounts\n")
        .answer("seed-load-fixture-events", 0, 0, "fixture=01 event=e\n")
        .answer("cat --", 0, 0, &accounts)
        .answer("rm -f", 0, 0, "");
    let mut output = Vec::new();
    let token_file = scratch_folder("seed-live").join("load_tokens");
    let code = seed_load(
        &load_settings(),
        &committed(),
        &token_file,
        &PlanOnly { dry_run: false },
        &mut host,
        &mut output,
    )
    .unwrap();
    let text = String::from_utf8(output).unwrap();
    assert_eq!(code, 0, "{text}");
    assert_eq!(std::fs::read_to_string(&token_file).unwrap(), accounts);
    assert_eq!(
        std::fs::metadata(&token_file).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert!(!text.contains("not-a-token"), "{text}");
    let order: Vec<&str> = host.calls.iter().map(|call| call.observer).collect();
    assert_eq!(
        order,
        [
            "database",
            "host tool",
            "host tool",
            "host account file",
            "host account file"
        ]
    );
    assert!(
        host.calls[2]
            .command_line
            .contains("'--mission' 'mission-1'")
    );
    let again = seed_load(
        &load_settings(),
        &committed(),
        &token_file,
        &PlanOnly { dry_run: false },
        &mut host,
        &mut Vec::new(),
    );
    assert!(again.unwrap_err().to_string().contains("already exists"));
}

#[test]
fn staging_seed_load_refuses_two_live_missions_before_changing_anything() {
    let clock = FakeClock::starting_at(START_UNIX_MS);
    let mut host = ScriptedHost::new(&clock).answer(
        "FROM missions WHERE title",
        0,
        0,
        "mission-1\nmission-2\n",
    );
    let token_file = scratch_folder("seed-two").join("load_tokens");
    let refused = seed_load(
        &load_settings(),
        &committed(),
        &token_file,
        &PlanOnly { dry_run: false },
        &mut host,
        &mut Vec::new(),
    );
    assert!(refused.unwrap_err().to_string().contains("2 live missions"));
    assert_eq!(host.calls.len(), 1);
    assert!(!Path::new(&token_file).exists());
}

#[test]
fn staging_clean_load_cleans_fixture_events_then_population_then_the_host_copy() {
    let clock = FakeClock::starting_at(START_UNIX_MS);
    let mut host = ScriptedHost::new(&clock)
        .answer("staging-fixtures", 0, 0, "")
        .answer("rm -f", 0, 0, "");
    let code = clean_load(
        &load_settings(),
        &PlanOnly { dry_run: false },
        &mut host,
        &mut Vec::new(),
    )
    .unwrap();
    assert_eq!(code, 0);
    let lines: Vec<&str> = host
        .calls
        .iter()
        .map(|call| call.command_line.as_str())
        .collect();
    assert!(
        lines[0].contains("'clean-load-fixture-events'")
            && lines[1].contains("'clean-load-population'")
    );
    assert!(
        lines[2].contains("rm -f -- '/home/deploy/tbd/load/load-accounts.json'"),
        "{}",
        lines[2]
    );
}
