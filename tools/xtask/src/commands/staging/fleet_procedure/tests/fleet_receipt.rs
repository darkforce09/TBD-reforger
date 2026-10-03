//! The fleet procedure's plan, lists, preflight, judge mapping, and one recorded run end to end:
//! a run whose every declared case holds still ends in a failing receipt that carries a client
//! count of 1, its real observations and the manifest it cites; a process environment that sets
//! `PROPTEST_RNG_SEED` is refused before any run folder, log or receipt is written.
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::MutexGuard;

use serde_json::{Value, json};

use super::fleet_reads::{
    FLEET_COMMANDS, FLEET_DEPLOYMENTS, FLEET_IDENTITIES, FLEET_RESTING_SESSIONS, FLEET_SESSIONS,
};
use super::judge_mapping::{client_count, judge_scenarios, server_ids};
use super::recorded_fleet::{RecordedFleet, T0, identities};
use super::recorded_single_server::InboxWritingHost;
use super::single_server_reads::{
    OPERATOR_IDENTITY_LINK, SERVER_COMMAND_LEASES, SERVER_MACHINE_CREDENTIALS,
};
use super::waves::deployment_waves::{ARLAND_MISSION, EVERON_MISSION};
use super::{FLEET_HARD_STOP_SECONDS, FleetProcedure};
use crate::commands::deploy::remote_rust_toolchain::PUT_RUST_TOOLCHAIN_ON_PATH;
use crate::commands::deploy::staging::payloads::instance_profile_commands;
use crate::commands::staging::operator_coordination::action_list::render;
use crate::commands::staging::procedure_runner::fake_clock::FakeClock;
use crate::commands::staging::procedure_runner::procedure::StagingProcedure;
use crate::commands::staging::procedure_runner::recording::{RecordingInputs, record};
use crate::commands::staging::procedure_runner::runner_support::{
    ScriptedHost, scratch_folder, test_settings,
};
use crate::commands::staging::procedure_runner::step::{
    Measurements, ProbeSource, StepContext, StepKind,
};
use crate::commands::staging::remote_observers::database_reader;
use crate::commands::staging::remote_observers::remote_command::{CommandOutput, CommandPurpose};
use crate::commands::staging::run_identity::EVIDENCE_DIRECTORY;
use crate::commands::staging::support_commands::preflight::PreflightProbe;
use crate::core::repository_layout::documentation::API_READINESS_REGISTER;
use crate::verifications::api_readiness::operational_recording::{
    CaseName, CaseStatus, RecordedCase,
};

/// An isolated Git tree whose register declares the real `staging_fleet` check; holds the
/// environment lock so no other test moves a fingerprinted variable during the recording.
struct IsolatedRepository {
    root: PathBuf,
    _environment: MutexGuard<'static, ()>,
}

impl IsolatedRepository {
    fn new() -> Self {
        let environment = crate::core::test_environment::lock_env();
        let root = scratch_folder("fleet-receipt");
        let status = Command::new("git")
            .args(["init", "--quiet", "--initial-branch=main"])
            .current_dir(&root)
            .status()
            .expect("run git init");
        assert!(status.success(), "git init failed");
        let real_register: Value = serde_json::from_slice(
            &std::fs::read(
                repository_layout::find_repository_root_from(Path::new(env!("CARGO_MANIFEST_DIR")))
                    .expect("repository root")
                    .join(API_READINESS_REGISTER),
            )
            .unwrap(),
        )
        .unwrap();
        let definition = real_register["checks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|check| check["id"] == "staging_fleet")
            .cloned()
            .expect("the real register declares staging_fleet");
        let register = json!({
            "version": 1,
            "requirements": [{
                "id": "staging_fleet",
                "behavior": "fleet acceptance is recorded",
                "implementation": ["apps/module.rs"],
                "checks": ["staging_fleet"],
                "assumptions": [],
            }],
            "checks": [definition],
        });
        let repository = Self {
            root,
            _environment: environment,
        };
        repository.write("Cargo.toml", "[workspace]\n");
        repository.write("apps/module.rs", "pub fn recorded() {}\n");
        repository.write(
            API_READINESS_REGISTER,
            &serde_json::to_string_pretty(&register).unwrap(),
        );
        repository
    }

    fn write(&self, path: &str, contents: &str) {
        let path = self.root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn evidence(&self, file: &str) -> Vec<u8> {
        std::fs::read(self.root.join(EVIDENCE_DIRECTORY).join(file)).unwrap()
    }
}

impl Drop for IsolatedRepository {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn staging_fleet_record_refuses_a_proptest_environment_before_any_run_folder_log_or_receipt() {
    let repository = IsolatedRepository::new();
    let settings = test_settings();
    let clock = FakeClock::starting_at(T0);
    let mut host = ScriptedHost::new(&clock);
    let mut output = Vec::new();
    let error = record(
        &FleetProcedure,
        RecordingInputs {
            root: &repository.root,
            settings: &settings,
            host: &mut host,
            clock: &clock,
            command: ["cargo", "xtask", "staging", "fleet", "--record"]
                .map(String::from)
                .to_vec(),
            process_environment: vec![
                ("PATH".into(), "/usr/bin".into()),
                ("PROPTEST_RNG_SEED".into(), "2026092201".into()),
            ],
            output: &mut output,
        },
    )
    .unwrap_err();
    assert_eq!(
        format!("{error:#}"),
        "PROPTEST_RNG_SEED must be unset while a staging check records"
    );
    assert!(output.is_empty(), "{}", String::from_utf8_lossy(&output));
    assert!(host.calls.is_empty(), "no host command ran");
    assert!(
        !repository.root.join("target").exists(),
        "no run folder, log or receipt was written"
    );
}

#[test]
fn staging_fleet_record_writes_a_failing_receipt_with_client_count_one() {
    let repository = IsolatedRepository::new();
    let settings = test_settings();
    let clock = FakeClock::starting_at(T0);
    let recorded = RecordedFleet {
        listed_arma_ids: [vec!["arma-operator"], vec![], vec![], vec![], vec![]],
        ..RecordedFleet::default()
    };
    let mut host = InboxWritingHost {
        host: recorded.host(&clock),
        runs: repository.root.join("target/staging/staging_fleet"),
        recorded: recorded.single_server.clone(),
        written: false,
    };
    let mut output = Vec::new();
    let exit_code = record(
        &FleetProcedure,
        RecordingInputs {
            root: &repository.root,
            settings: &settings,
            host: &mut host,
            clock: &clock,
            command: ["cargo", "xtask", "staging", "fleet", "--record"]
                .map(String::from)
                .to_vec(),
            process_environment: Vec::new(),
            output: &mut output,
        },
    )
    .unwrap();
    let printed = String::from_utf8(output).unwrap();
    assert_eq!(exit_code, 1, "{printed}");
    let receipt: Value =
        serde_json::from_slice(&repository.evidence("staging_fleet.json")).unwrap();
    let log = String::from_utf8(repository.evidence("staging_fleet.log")).unwrap();
    let manifest_bytes = repository.evidence("staging_fleet.fixture.json");
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(receipt["exit_code"], 1);
    let observations = &receipt["observations"];
    assert_eq!(observations["kind"], "fleet");
    assert_eq!(observations["client_count"], 1);
    assert_eq!(observations["server_ids"].as_array().unwrap().len(), 5);
    assert_eq!(
        observations["scenarios"],
        json!([
            "stop",
            "start",
            "restart",
            "custom_console",
            "same_terrain",
            "cross_terrain",
            "kick",
            "identity_link",
            "lost_acknowledgement"
        ])
    );
    assert_eq!(
        observations["fixture_sha256"],
        crate::verifications::api_readiness::operational_recording::FixtureManifest::new(&manifest)
            .unwrap()
            .sha256()
    );
    assert!(!log.contains("staging_fleet: PASS"), "{log}");
    let verdict = log.lines().last().unwrap();
    assert!(
        verdict.starts_with("staging_fleet: FAIL 48/50")
            && verdict.contains("five distinct servers and real clients"),
        "{verdict}"
    );
    assert!(log.contains("case staging_fleet_server5_cross_terrain ... ok"));
    assert!(log.contains("observation: w5_list_players.server1_command database"));
    assert!(log.contains("case staging_fleet_lost_acknowledgement_result_response ... ok"));
    assert!(log.contains("observation: w9_identity_link.link_status chrome page read"));
    assert!(log.contains("observation: w13_lost_claim_answer.action relay control"));
    assert_eq!(manifest["check"], "staging_fleet");
    assert_eq!(manifest["identities"]["workshop_version"], "1.4.2");
    assert_eq!(
        manifest["identities"]["servers"][4]["name"],
        "TBD Staging 5"
    );
    assert_eq!(
        manifest["procedure"]["hard_stop_seconds"],
        FLEET_HARD_STOP_SECONDS
    );
    assert_eq!(
        manifest["procedure"]["steps"][6]["effects"][0]["deadline_seconds"],
        1_200
    );
    assert_eq!(
        crate::verifications::api_readiness::verify(
            &repository.root,
            Path::new(EVIDENCE_DIRECTORY),
            false
        )
        .unwrap(),
        1,
        "the judge accepted a failing fleet receipt"
    );
}

#[test]
fn staging_fleet_plan_declares_its_cases_and_reads_only() {
    let settings = test_settings();
    let plan = FleetProcedure.plan(&settings).unwrap();
    plan.validate().unwrap();
    assert_eq!(plan.declared_cases.len(), 50);
    assert_eq!(
        plan.declared_cases[49].name.as_str(),
        "lost_acknowledgement_result_response"
    );
    assert_eq!(plan.declared_cases[0].name.as_str(), "server1_stop");
    assert_eq!(
        plan.declared_cases[39].name.as_str(),
        "server5_runtime_session_succession"
    );
    let not_run: Vec<(&str, &str)> = plan
        .declared_cases
        .iter()
        .filter_map(|case| {
            let missing = case.unavailable_dependency.as_deref()?;
            Some((case.name.as_str(), missing))
        })
        .collect();
    assert_eq!(
        not_run,
        [
            ("kick_targets_one_of_two_clients", "second game client"),
            ("same_terrain_carries_two_clients", "second game client"),
        ]
    );
    for query in [
        FLEET_COMMANDS,
        FLEET_SESSIONS,
        FLEET_DEPLOYMENTS,
        FLEET_IDENTITIES,
        FLEET_RESTING_SESSIONS,
        OPERATOR_IDENTITY_LINK,
        SERVER_COMMAND_LEASES,
        SERVER_MACHINE_CREDENTIALS,
    ] {
        database_reader::validate(&query).unwrap();
    }
    assert!(
        FLEET_IDENTITIES
            .sql
            .contains(&format!("'{EVERON_MISSION}', '{ARLAND_MISSION}'"))
    );
    let measurements = Measurements::new();
    let context = StepContext {
        step_started_unix_ms: T0 + 60_000,
        request_unix_ms: Some(T0 + 61_000),
        observed_unix_ms: T0 + 62_000,
        measurements: &measurements,
    };
    let (mut reads, mut measured, mut browser, mut host_actions) = (0, 0, Vec::new(), Vec::new());
    for step in &plan.steps {
        if let StepKind::HostAction(command) = &step.kind {
            assert_eq!(
                command.purpose,
                CommandPurpose::Change,
                "{}",
                step.id.as_str()
            );
            let text = format!(
                "{} {}",
                command.command_line,
                command.stdin.clone().unwrap_or_default()
            );
            assert!(
                !text.contains("cat ") && !text.contains("password"),
                "{text}"
            );
            host_actions.push(step.id.as_str().to_string());
        }
        let request = step
            .request
            .as_ref()
            .map(|request| ("request", &request.probe));
        let effects = step.effects.iter().map(|e| (e.id.as_str(), &e.probe));
        for (id, probe) in request.into_iter().chain(effects) {
            let read = match &probe.source {
                ProbeSource::Host(read) => read,
                ProbeSource::BrowserInbox => {
                    browser.push(format!("{}.{id}", step.id.as_str()));
                    continue;
                }
                ProbeSource::Measurement(_) => {
                    measured += 1;
                    continue;
                }
            };
            let command = read(&context).unwrap();
            assert_eq!(
                command.purpose,
                CommandPurpose::Read,
                "{}",
                command.command_line
            );
            let text = format!(
                "{} {}",
                command.command_line,
                command.stdin.unwrap_or_default()
            );
            assert!(
                !text.contains("cat ") && !text.contains("password"),
                "{text}"
            );
            if command.observer == "database" {
                assert!(text.contains("default_transaction_read_only=on"));
                if !text.contains("executor_kind=") {
                    assert!(text.contains("since_ms=1800000055000"), "{text}");
                }
            }
            if command.observer == "unit journal" {
                assert!(text.contains("--since=@1800000056"), "{text}");
            }
            reads += 1;
        }
    }
    assert_eq!(reads, 8 + 5 * (2 + 4 + 4 + 1 + 1 + 4 + 5 + 5) + 26);
    assert_eq!(measured, 2);
    assert_eq!(
        browser,
        [
            "w9_identity_link.link_status",
            "w10_ended_session_kick.refused"
        ]
    );
    assert_eq!(
        host_actions,
        [
            "w11_stage_host_agent_credential",
            "w11_promote_host_agent_credential",
            "w12_stage_mod_runtime_credential",
            "w12_promote_mod_runtime_credential",
            "w13_lost_claim_answer",
            "w14_lost_result_answer"
        ]
    );
}

#[test]
fn staging_fleet_lists_number_every_wave_and_the_recovery() {
    let settings = test_settings();
    let actions = FleetProcedure.action_list(&settings);
    assert_eq!(actions.len(), 19);
    for action in &actions[..8] {
        assert_eq!(action.actor, "orchestrator (browser)");
        assert_eq!(action.details.len(), 6, "{action:?}");
        assert!(action.details[4].ends_with("on TBD Staging 5"));
        assert!(action.details[5].starts_with("awaited: "));
    }
    let text = render("staging_fleet actions", &actions);
    assert!(text.contains("  1. [orchestrator (browser)] W1: Stop on every fleet server"));
    assert!(
        text.contains(
            "  7. [orchestrator (browser)] W7: Deploy the mission \"TBD Staging Arland\""
        )
    );
    assert!(text.contains("(deadline 1200 s from the wave's first request)"));
    let recovery = render(
        "staging_fleet recovery",
        &FleetProcedure.recovery_action_list(&settings),
    );
    assert!(
        recovery.contains("1. [operator] Disarm the acknowledgement-dropping relay of instance 5")
    );
    assert!(recovery.contains("acknowledgement-dropping-relay-5/control.sock\" disarm"));
    assert!(recovery.contains("2. [orchestrator (browser)] Start every fleet server"));
    assert!(recovery.contains("tbd-reforger@3.service inactive: Start on TBD Staging 3"));
    assert!(recovery.contains("3. [orchestrator (browser)] Deploy TBD Staging Everon"));
    assert!(recovery.contains("4. [operator] Finish a credential rotation W11 or W12 left midway"));
    assert!(recovery.contains(
        "--instance 2 --executor mod_runtime --promote`, then `cargo xtask deploy staging`"
    ));
    assert!(text.contains(
        "  9. [operator (game) and orchestrator (browser)] W9: the operator joins TBD Staging 1;"
    ));
    assert!(text.contains(
        "12. [harness (host)] W11: the harness stages a new host_agent credential of TBD Staging 1"
    ));
    assert!(text.contains("'rotate-credential' '--instance' '1' '--executor' 'host_agent'"));
    assert!(text.contains("systemctl --user restart 'fleet_host_agent@1.service'"));
    assert!(text.contains("setup server-profile \"$INSTANCE/profile\""));
    assert!(text.contains(
        "18. [harness (host), then orchestrator (browser)] W13: the harness arms the relay of TBD Staging 5"
    ));
    assert!(
        text.contains(
            "acknowledgement-dropping-relay-5/control.sock\" arm drop-next-result-response"
        )
    );
    assert!(text.contains("(deadline 900 s from the step's start)"));
}

/// W12's promotion rewrites instance 2's profile with the staging deploy's own profile writer,
/// between the promotion and the game server's restart.
#[test]
fn staging_fleet_mod_runtime_promotion_writes_the_profile_as_the_deploy_does() {
    let settings = test_settings();
    let plan = FleetProcedure.plan(&settings).unwrap();
    let promotion = plan
        .steps
        .iter()
        .find_map(|step| match &step.kind {
            StepKind::HostAction(command)
                if step.id.as_str() == "w12_promote_mod_runtime_credential" =>
            {
                command.stdin.clone()
            }
            _ => None,
        })
        .expect("W12 promotes on the host");
    let profile = instance_profile_commands(&settings.checkout, &settings.api_origin);
    assert!(
        promotion.contains(&format!(
            "'--executor' 'mod_runtime' '--secrets-root' '/home/deploy/tbd/fleet' '--promote' \
             '--confirm-database' 'tbd_reforger' '--apply'\n\
             {PUT_RUST_TOOLCHAIN_ON_PATH}\n\
             INSTANCE='/home/deploy/tbd/fleet/instance-2'\n\
             SECRETS='/home/deploy/tbd/fleet/instance-2/secrets'\n\
             {profile}systemctl --user restart 'tbd-reforger@2.service'\n"
        )),
        "{promotion}"
    );
}

#[test]
fn staging_fleet_judge_mapping_names_a_scenario_only_when_every_server_passed() {
    let case = |name: &str, status: CaseStatus| RecordedCase {
        name: CaseName::new(name).unwrap(),
        status,
    };
    let mut cases: Vec<RecordedCase> = (1..=5)
        .flat_map(|n| {
            [
                case(&format!("server{n}_stop"), CaseStatus::Ok),
                case(&format!("server{n}_start"), CaseStatus::Ok),
            ]
        })
        .collect();
    cases[9] = case("server5_start", CaseStatus::Failed("late".into()));
    cases.push(case("kick", CaseStatus::Ok));
    cases.push(case("lost_acknowledgement_claim_response", CaseStatus::Ok));
    cases.push(case(
        "lost_acknowledgement_result_response",
        CaseStatus::NotRun {
            missing: "relay".into(),
        },
    ));
    assert_eq!(judge_scenarios(&cases), ["stop", "kick"]);
    let mut measurements = Measurements::new();
    measurements.insert("player_listing.w5_list_players.server1".into(), json!(1));
    measurements.insert("player_listing.w10_kick.server1".into(), json!(2));
    measurements.insert("w3_restart.server1.pid".into(), json!(9_999));
    measurements.insert("fleet.server2.id".into(), json!("id-2"));
    measurements.insert("fleet.server1.id".into(), json!("id-1"));
    assert_eq!(client_count(&measurements), 2);
    assert_eq!(server_ids(&measurements), ["id-1", "id-2"]);
    assert_eq!(client_count(&Measurements::new()), 0);
}

#[test]
fn staging_fleet_preflight_names_each_unmet_precondition() {
    let settings = test_settings();
    let checks = FleetProcedure.preflight_checks(&settings);
    let judge = |name: &str, stdout: &str| -> Result<String, String> {
        let check = checks.iter().find(|check| check.name == name).unwrap();
        let PreflightProbe::Host { judge, .. } = &check.probe else {
            panic!("{name} is not a host check");
        };
        judge(&CommandOutput {
            exit_code: 0,
            stdout: stdout.to_string(),
            stderr: String::new(),
        })
    };
    judge("fleet servers registered", &identities()).unwrap();
    judge("fleet missions and scenarios", &identities()).unwrap();
    let mut broken: Value = serde_json::from_str(&identities()).unwrap();
    broken["servers"].as_array_mut().unwrap().remove(2);
    broken["missions"][0]["status"] = json!("approved");
    broken["fleet_scenarios"].as_array_mut().unwrap().remove(1);
    let servers = judge("fleet servers registered", &broken.to_string()).unwrap_err();
    assert_eq!(servers, "TBD Staging 3 is not registered and active");
    let missions = judge("fleet missions and scenarios", &broken.to_string()).unwrap_err();
    assert!(
        missions.contains("TBD Staging Arland is \"approved\" on \"arland\"")
            && missions.contains("no fleet scenario row for everon"),
        "{missions}"
    );
    let session = |server: u16, generation: Value, age: u64, terrain: Value| {
        json!({"server": format!("TBD Staging {server}"), "generation": generation,
            "heartbeat_age_seconds": age, "terrain": terrain, "mission": "TBD Staging Everon"})
        .to_string()
    };
    let resting: Vec<String> = (1..=5)
        .map(|n| session(n, json!(n), 10, json!("everon")))
        .collect();
    judge("fleet runs the Everon deployment", &resting.join("\n")).unwrap();
    let unmet = [
        session(1, json!(1), 10, json!("everon")),
        session(2, json!(2), 90, json!("everon")),
        session(3, Value::Null, 10, Value::Null),
        session(4, json!(4), 10, json!("arland")),
    ]
    .join("\n");
    let why = judge("fleet runs the Everon deployment", &unmet).unwrap_err();
    assert!(
        why.contains("TBD Staging 2's session last heartbeated 90 s ago")
            && why.contains("TBD Staging 3 has no open runtime session")
            && why.contains("TBD Staging 4's generation 4 runs TBD Staging Everon on arland")
            && why.contains("TBD Staging 5 is not registered and active"),
        "{why}"
    );
}
