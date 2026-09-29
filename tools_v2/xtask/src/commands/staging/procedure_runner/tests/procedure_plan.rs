//! Plan validation refuses every plan the recorder could not judge honestly, the manifest
//! definition names each step's deadlines and cases, and a refused plan never touches a receipt.
use super::fake_clock::FakeClock;
use super::procedure::{ProcedurePlan, ProcedureRun, StagingProcedure};
use super::recording::{RecordingInputs, record};
use super::runner_support::{ScriptedHost, scratch_folder, test_settings};
use super::step::{
    Deadline, DeclaredCase, EffectPredicate, Probe, ProbeVerdict, RequestPredicate, Step, StepId,
    StepKind,
};
use crate::commands::staging::fleet_procedure::FleetProcedure;
use crate::commands::staging::operator_coordination::action_list::PlannedAction;
use crate::commands::staging::remote_observers::remote_command::{
    HostCommandRunner, RemoteCommand,
};
use crate::commands::staging::staging_settings::StagingSettings;
use crate::commands::staging::support_commands::preflight::PreflightCheck;
use crate::verifications::api_readiness::operational_recording::{
    CaseName, FixtureManifest, Observations, StagingCheck,
};

fn effect(id: &str, case: &str, deadline: Deadline) -> EffectPredicate {
    EffectPredicate {
        id: id.into(),
        description: format!("effect {id}"),
        probe: Probe::host(
            |_| Ok(RemoteCommand::read("database", "psql".into())),
            |_, _| ProbeVerdict::Pending("never".into()),
        ),
        deadline,
        case: CaseName::new(case).unwrap(),
    }
}

fn step(id: &str, request: bool, effects: Vec<EffectPredicate>) -> Step {
    Step {
        id: StepId::new(id).unwrap(),
        kind: StepKind::HostAction(RemoteCommand::change("relay control", "arm".into())),
        instruction: "arm the relay".into(),
        request: request.then(|| RequestPredicate {
            description: "the restart row".into(),
            probe: Probe::host(
                |_| Ok(RemoteCommand::read("database", "psql".into())),
                |_, _| ProbeVerdict::Pending("never".into()),
            ),
        }),
        effects,
    }
}

fn plan(cases: Vec<DeclaredCase>, steps: Vec<Step>) -> ProcedurePlan {
    ProcedurePlan {
        declared_cases: cases,
        steps,
        hard_stop_seconds: 6_900,
        poll_interval_seconds: 5,
    }
}

fn refusal(plan: &ProcedurePlan) -> String {
    format!("{:#}", plan.validate().unwrap_err())
}

#[test]
fn staging_plan_validation_refuses_what_the_recorder_cannot_judge() {
    let runs = |name: &str| DeclaredCase::runs(name).unwrap();
    assert!(refusal(&plan(Vec::new(), Vec::new())).contains("declares no cases"));
    assert!(refusal(&plan(vec![runs("a"), runs("a")], Vec::new())).contains("declared twice"));
    assert!(refusal(&plan(vec![runs("a")], Vec::new())).contains("no effect decides case a"));
    let undeclared = plan(
        vec![runs("a")],
        vec![step(
            "w1",
            false,
            vec![effect("x", "b", Deadline::from_step_start(1))],
        )],
    );
    assert!(refusal(&undeclared).contains("undeclared case b"));
    let request_anchor = plan(
        vec![runs("a")],
        vec![step(
            "w1",
            false,
            vec![effect("x", "a", Deadline::from_request_row(1))],
        )],
    );
    assert!(refusal(&request_anchor).contains("request row its step does not observe"));
    let duplicate_effect = plan(
        vec![runs("a")],
        vec![step(
            "w1",
            false,
            vec![
                effect("x", "a", Deadline::from_step_start(1)),
                effect("x", "a", Deadline::from_step_start(1)),
            ],
        )],
    );
    assert!(refusal(&duplicate_effect).contains("unique"));
    let duplicate_step = plan(
        vec![runs("a")],
        vec![
            step(
                "w1",
                false,
                vec![effect("x", "a", Deadline::from_step_start(1))],
            ),
            step("w1", false, Vec::new()),
        ],
    );
    assert!(refusal(&duplicate_step).contains("appears twice"));
    let decided_not_run = plan(
        vec![DeclaredCase::not_run("a", "second game client").unwrap()],
        vec![step(
            "w1",
            false,
            vec![effect("x", "a", Deadline::from_step_start(1))],
        )],
    );
    assert!(refusal(&decided_not_run).contains("declared not run"));
    assert!(StepId::new("W1").is_err() && DeclaredCase::runs("Bad-Name").is_err());
    let sound = plan(
        vec![
            runs("a"),
            DeclaredCase::not_run("b", "second game client").unwrap(),
        ],
        vec![step(
            "w1",
            true,
            vec![effect("x", "a", Deadline::from_request_row(150))],
        )],
    );
    sound.validate().unwrap();
}

#[test]
fn staging_plan_definition_names_steps_deadlines_and_cases() {
    let definition = plan(
        vec![
            DeclaredCase::runs("a").unwrap(),
            DeclaredCase::not_run("b", "second game client").unwrap(),
        ],
        vec![step(
            "w1",
            true,
            vec![effect("x", "a", Deadline::from_request_row(150))],
        )],
    )
    .definition();
    assert_eq!(definition["hard_stop_seconds"], 6_900);
    assert_eq!(definition["steps"][0]["kind"], "host_action");
    assert_eq!(definition["steps"][0]["host_action"], "arm");
    assert_eq!(definition["steps"][0]["request"], "the restart row");
    assert_eq!(
        definition["steps"][0]["effects"][0]["deadline_seconds"],
        150
    );
    assert_eq!(
        definition["steps"][0]["effects"][0]["deadline_from"],
        "request_row"
    );
    assert_eq!(definition["steps"][0]["effects"][0]["case"], "a");
    assert_eq!(
        definition["declared_cases"][1]["unavailable_dependency"],
        "second game client"
    );
}

/// A procedure whose plan declares no case.
struct CaselessProcedure;

impl StagingProcedure for CaselessProcedure {
    fn check(&self) -> StagingCheck {
        StagingCheck::Fleet
    }

    fn action_list(&self, _: &StagingSettings) -> Vec<PlannedAction> {
        Vec::new()
    }

    fn recovery_action_list(&self, _: &StagingSettings) -> Vec<PlannedAction> {
        Vec::new()
    }

    fn preflight_checks(&self, _: &StagingSettings) -> Vec<PreflightCheck> {
        Vec::new()
    }

    fn plan(&self, _: &StagingSettings) -> anyhow::Result<ProcedurePlan> {
        Ok(plan(Vec::new(), Vec::new()))
    }

    fn fixture_identities(
        &self,
        _: &StagingSettings,
        _: &mut dyn HostCommandRunner,
    ) -> anyhow::Result<serde_json::Value> {
        Ok(serde_json::Value::Null)
    }

    fn observations(&self, _: &ProcedureRun, manifest: &FixtureManifest) -> Observations {
        Observations::Fleet {
            server_ids: Vec::new(),
            client_count: 0,
            scenarios: Vec::new(),
            fixture_sha256: manifest.sha256().to_string(),
        }
    }
}

#[test]
fn staging_record_refuses_a_plan_without_cases_before_any_receipt_is_touched() {
    let root = scratch_folder("record-refused");
    let settings = test_settings();
    let clock = FakeClock::starting_at(1_800_000_000_000);
    let mut host = ScriptedHost::new(&clock);
    let mut output = Vec::new();
    let error = record(
        &CaselessProcedure,
        RecordingInputs {
            root: &root,
            settings: &settings,
            host: &mut host,
            clock: &clock,
            command: vec![
                "cargo".into(),
                "xtask".into(),
                "staging".into(),
                "fleet".into(),
                "--record".into(),
            ],
            process_environment: Vec::new(),
            output: &mut output,
        },
    )
    .unwrap_err();
    assert!(
        format!("{error:#}").contains("declares no cases"),
        "{error:#}"
    );
    assert!(
        !root.join("target").exists(),
        "no evidence or journal folder was created"
    );
    assert!(host.calls.is_empty());
    assert_eq!(FleetProcedure.check().id(), "staging_fleet");
}
