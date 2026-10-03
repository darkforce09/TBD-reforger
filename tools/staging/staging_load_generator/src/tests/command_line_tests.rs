use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};

use clap::error::ErrorKind;
use serde_json::json;
use staging_load_plan::sample_plans::{fixture_events, read_template};
use staging_load_plan::workload_plan::{PerAddressCeilings, WindowCeiling, WorkloadPlan};
use staging_load_plan::{LoadRunPlan, decode_report, encode_plan};

use super::*;
use crate::stub_api_server::StubApi;

/// A scratch folder for one test, removed when dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "tbd-staging-load-command-line-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("scratch folder");
        Self(path)
    }

    fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Two clients on 127.0.0.2 for one second against `origin`, with their account file written.
fn short_plan(origin: &str, account_file: &Path) -> LoadRunPlan {
    let accounts: Vec<_> = (0..2u64)
        .map(|account| {
            json!({
                "discord_id": (9_100_000_000_000_000_000u64 + account).to_string(),
                "refresh_token": format!("refresh-{account}-0"),
            })
        })
        .collect();
    std::fs::write(account_file, json!({ "accounts": accounts }).to_string())
        .expect("account file");
    LoadRunPlan {
        workload: WorkloadPlan {
            seed: 5,
            ramp_seconds: 0.2,
            measured_seconds: 1.0,
            clients: 2,
            source_address_count: 1,
            requests_per_second: 10.0,
            accounts_per_client: 1,
            account_hold_seconds: 60.0,
            jitter_fraction: 0.05,
            request_timeout_seconds: 2.0,
            census_window_seconds: 0.5,
            per_address_ceilings: PerAddressCeilings {
                all_requests: WindowCeiling {
                    max_requests: 40,
                    window_seconds: 1.0,
                },
                auth_requests: WindowCeiling {
                    max_requests: 20,
                    window_seconds: 1.0,
                },
            },
            request_mix: vec![read_template("events", "/api/v1/events", &[200])],
        },
        target_origin: origin.to_owned(),
        source_addresses: vec![IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2))],
        account_file: account_file.to_path_buf(),
        fixture_events: fixture_events(1, 2),
    }
}

fn command_line(arguments: &[&str]) -> StagingLoadCommandLine {
    StagingLoadCommandLine::try_parse_from(
        std::iter::once("staging-load").chain(arguments.iter().copied()),
    )
    .expect("the arguments parse")
}

#[test]
fn the_plan_and_the_report_default_to_standard_input_and_output() {
    let parsed = command_line(&[]);
    assert_eq!((parsed.plan, parsed.report), (None, None));
    let parsed = command_line(&["--plan", "plan.json", "--report", "report.json"]);
    assert_eq!(parsed.plan, Some(PathBuf::from("plan.json")));
    assert_eq!(parsed.report, Some(PathBuf::from("report.json")));
}

#[test]
fn an_unknown_argument_is_a_usage_error_with_exit_code_2() {
    let error = StagingLoadCommandLine::try_parse_from(["staging-load", "--clients", "3"])
        .expect_err("refused");
    assert_eq!(error.kind(), ErrorKind::UnknownArgument);
    assert_eq!(error.exit_code(), 2);
}

#[test]
fn a_plan_file_runs_and_its_report_is_written_as_one_json_line() {
    let scratch = Scratch::new("run");
    let stub = StubApi::start(1, None);
    let plan = short_plan(&stub.origin, &scratch.file("accounts.json"));
    std::fs::write(
        scratch.file("plan.json"),
        encode_plan(&plan).expect("encodes"),
    )
    .expect("plan file");
    let plan_path = scratch.file("plan.json");
    let report_path = scratch.file("report.json");
    run_command_line(&command_line(&[
        "--plan",
        plan_path.to_str().expect("utf-8"),
        "--report",
        report_path.to_str().expect("utf-8"),
    ]))
    .expect("the run");
    let text = std::fs::read_to_string(&report_path).expect("report file");
    assert_eq!(text.matches('\n').count(), 1, "one line: {text}");
    let report = decode_report(&text).expect("a report");
    assert_eq!((report.seed, report.clients, report.accounts), (5, 2, 2));
    assert!(report.completed_requests > 0, "{report:?}");
}

#[test]
fn a_missing_plan_file_names_the_file() {
    let scratch = Scratch::new("missing");
    let plan_path = scratch.file("absent.json");
    let error = run_command_line(&command_line(&[
        "--plan",
        plan_path.to_str().expect("utf-8"),
    ]))
    .expect_err("refused");
    assert!(
        error
            .to_string()
            .starts_with(&format!("reading the load plan {}: ", plan_path.display())),
        "{error}"
    );
}

#[test]
fn a_refused_plan_writes_no_report() {
    let scratch = Scratch::new("refused");
    let mut plan = short_plan("http://127.0.0.1:9", &scratch.file("accounts.json"));
    plan.workload.clients = 0;
    std::fs::write(
        scratch.file("plan.json"),
        encode_plan(&plan).expect("encodes"),
    )
    .expect("plan file");
    let plan_path = scratch.file("plan.json");
    let report_path = scratch.file("report.json");
    let error = run_command_line(&command_line(&[
        "--plan",
        plan_path.to_str().expect("utf-8"),
        "--report",
        report_path.to_str().expect("utf-8"),
    ]))
    .expect_err("refused");
    assert_eq!(error.to_string(), "clients must be at least 1");
    assert!(!report_path.exists());
}

#[test]
fn a_plan_that_is_not_json_is_refused_as_undecodable() {
    let scratch = Scratch::new("undecodable");
    std::fs::write(scratch.file("plan.json"), "not json").expect("plan file");
    let plan_path = scratch.file("plan.json");
    let error = run_command_line(&command_line(&[
        "--plan",
        plan_path.to_str().expect("utf-8"),
    ]))
    .expect_err("refused");
    assert!(
        error.to_string().starts_with("decoding the load plan: "),
        "{error}"
    );
}
