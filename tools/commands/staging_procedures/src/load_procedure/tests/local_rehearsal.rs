//! The local rehearsal, run dry against a stub local stack: the seeding, the keying, the
//! shortened member load, the judgement and the cleanup, with nothing recorded.

use std::path::PathBuf;
use std::sync::Arc;

use crate::error::{Result, bail};

use super::load_queries::{LOAD_FIXTURE_EVENTS, STRICT_RATE_LIMIT_BUCKETS};
use super::load_report_judges::{LoadThresholds, concurrency, member_accounts};
use super::load_test_support::committed;
use super::load_test_support::{StubWorkstation, fixture_event_rows, passing_report};
use super::local_rehearsal::{
    LOCAL_LIVE_MISSION, REHEARSAL_MEASURED_SECONDS, RehearsalEnvironment, rehearsal_addresses,
    rehearsal_thresholds, rehearsal_workload, rehearse,
};
use super::workstation_load::WorkstationLoad;
use crate::procedure_runner::runner_support::scratch_folder;
use crate::remote_observers::database_reader::CommittedQuery;
use crate::remote_observers::remote_command::CommandOutput;

struct StubStack {
    stub: Arc<StubWorkstation>,
    host_tool_calls: Vec<Vec<String>>,
    scratch: PathBuf,
}

impl RehearsalEnvironment for StubStack {
    fn host_tool(&mut self, arguments: &[String]) -> Result<CommandOutput> {
        self.host_tool_calls.push(arguments.to_vec());
        Ok(CommandOutput {
            exit_code: 0,
            stdout: format!("{} done\n", arguments[0]),
            stderr: String::new(),
        })
    }

    fn read(&mut self, query: &CommittedQuery) -> Result<String> {
        match query.name {
            name if name == LOCAL_LIVE_MISSION.name => Ok("mission-1\n".into()),
            name if name == LOAD_FIXTURE_EVENTS.name => {
                Ok(fixture_event_rows(10, 128, "Local mission"))
            }
            name if name == STRICT_RATE_LIMIT_BUCKETS.name => Ok(rehearsal_addresses()
                .iter()
                .map(|address| format!("strict|{address}|1\n"))
                .collect()),
            other => bail!("the stub stack holds no answer for {other}"),
        }
    }

    fn workstation(&self) -> Arc<dyn WorkstationLoad> {
        self.stub.clone()
    }

    fn scratch_folder(&mut self) -> Result<PathBuf> {
        Ok(self.scratch.clone())
    }
}

fn stub_stack(outcome: std::result::Result<staging_load_plan::LoadReport, String>) -> StubStack {
    StubStack {
        stub: Arc::new(StubWorkstation::answering(outcome)),
        host_tool_calls: Vec::new(),
        scratch: scratch_folder("rehearsal"),
    }
}

#[test]
fn staging_load_rehearsal_dry_run_seeds_loads_judges_and_cleans_against_the_stub_stack() {
    let mut stack = stub_stack(Ok(passing_report(REHEARSAL_MEASURED_SECONDS)));
    let mut output = Vec::new();
    let code = rehearse(&mut stack, &committed(), &mut output).unwrap();
    let text = String::from_utf8(output).unwrap();
    assert_eq!(code, 0, "{text}");
    assert!(
        text.contains("records nothing") && text.ends_with("rehearsal: PASS\n"),
        "{text}"
    );
    assert!(
        text.contains(
            "rehearsal: member load for a 50 s ramp (the shortest that keeps 100 clients signing \
             in from 5 addresses at 80 % of the auth ceiling) and 60 measured s"
        ),
        "{text}"
    );
    assert!(
        text.contains(
            "rehearsal: rehearsal thresholds for this window, not the acceptance: concurrency at \
             least 100 clients, member_accounts at least 100 (the accounts 100 clients reach in \
             110 s); the recorded run keeps the acceptance of 100 clients and 1000 member accounts"
        ),
        "{text}"
    );
    for case in [
        "keying refreshes",
        "strict buckets",
        "sustained_rate",
        "concurrency",
        "member_accounts",
        "p95_json_reads",
        "p95_json_writes",
        "refresh_paced",
    ] {
        assert!(
            text.contains(&format!("rehearsal: {case} ... ok")),
            "{case}: {text}"
        );
    }
    let subcommands: Vec<&str> = stack
        .host_tool_calls
        .iter()
        .map(|call| call[0].as_str())
        .collect();
    assert_eq!(
        subcommands,
        [
            "seed-load-population",
            "seed-load-fixture-events",
            "clean-load-fixture-events",
            "clean-load-population"
        ]
    );
    for call in &stack.host_tool_calls {
        assert!(call.ends_with(&[
            "--confirm-database".to_string(),
            "tbd_reforger".into(),
            "--apply".into()
        ]));
    }
    let account_file = stack.scratch.join("load-accounts.json");
    assert!(stack.host_tool_calls[0].contains(&account_file.display().to_string()));
    assert!(
        !stack.scratch.exists(),
        "the rehearsal leaves its account folder behind"
    );
    let plans = stack.stub.plans.lock().unwrap();
    assert_eq!(plans[0].target_origin, "http://127.0.0.1:8080");
    assert_eq!(plans[0].source_addresses, rehearsal_addresses());
    assert_eq!(
        plans[0].workload.measured_seconds,
        REHEARSAL_MEASURED_SECONDS
    );
    assert_eq!(
        plans[0].workload.ramp_seconds,
        committed().workload.minimum_ramp_seconds()
    );
    plans[0]
        .validate()
        .expect("the rehearsal plan passes the engine's checks");
    assert_eq!(plans[0].account_file, account_file);
}

#[test]
fn staging_load_rehearsal_cleans_up_and_fails_when_the_engine_stops() {
    let mut stack = stub_stack(Err("the local API refused the connection".into()));
    let mut output = Vec::new();
    let code = rehearse(&mut stack, &committed(), &mut output).unwrap();
    let text = String::from_utf8(output).unwrap();
    assert_eq!(code, 1);
    assert!(
        text.contains("stopped: the local API refused the connection")
            && text.ends_with("rehearsal: FAIL\n"),
        "{text}"
    );
    let subcommands: Vec<&str> = stack
        .host_tool_calls
        .iter()
        .map(|call| call[0].as_str())
        .collect();
    assert_eq!(
        &subcommands[2..],
        ["clean-load-fixture-events", "clean-load-population"]
    );
    assert!(!stack.scratch.exists());
}

#[test]
fn staging_load_rehearsal_thresholds_scale_to_the_short_window_and_leave_the_acceptance_alone() {
    let committed = committed();
    let workload = rehearsal_workload(&committed.workload);
    assert_eq!(
        (workload.ramp_seconds, workload.measured_seconds),
        (50.0, REHEARSAL_MEASURED_SECONDS)
    );
    let limits = rehearsal_thresholds(&workload).unwrap();
    assert_eq!(
        limits,
        LoadThresholds {
            measured_seconds: REHEARSAL_MEASURED_SECONDS,
            concurrent_clients: 100,
            member_accounts: 100,
            ..LoadThresholds::ACCEPTANCE
        }
    );
    assert_eq!(
        (
            LoadThresholds::ACCEPTANCE.concurrent_clients,
            LoadThresholds::ACCEPTANCE.member_accounts
        ),
        (100, 1_000)
    );
    // A rehearsal's 100 member accounts pass its own thresholds and never the acceptance.
    let mut report = passing_report(REHEARSAL_MEASURED_SECONDS);
    report.member_accounts = 100;
    assert!(member_accounts(&report, &limits).is_ok());
    assert!(concurrency(&report, &limits).is_ok());
    assert!(member_accounts(&report, &LoadThresholds::ACCEPTANCE).is_err());
    report.member_accounts = 99;
    report.minimum_concurrent_clients = 99;
    assert!(member_accounts(&report, &limits).is_err());
    assert!(concurrency(&report, &limits).is_err());
    // Fewer clients scale both; a window long enough for a switch doubles the accounts.
    let mut fewer = committed.workload.clone();
    fewer.clients = 40;
    let fewer = rehearsal_workload(&fewer);
    assert_eq!(fewer.ramp_seconds, 20.0);
    let limits = rehearsal_thresholds(&fewer).unwrap();
    assert_eq!(
        (limits.concurrent_clients, limits.member_accounts),
        (40, 40)
    );
    let mut longer = rehearsal_workload(&committed.workload);
    longer.measured_seconds = 200.0;
    assert_eq!(rehearsal_thresholds(&longer).unwrap().member_accounts, 200);
}
