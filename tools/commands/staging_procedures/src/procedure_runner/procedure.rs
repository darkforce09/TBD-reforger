//! What a staging procedure supplies to the harness, and the checks its plan must pass before a
//! recording begins.
//!
//! **Role:** the [`StagingProcedure`] trait the fleet, Discord and load modules implement; the
//! [`ProcedurePlan`] (declared cases, steps, hard stop, poll interval) with its validation and
//! its manifest definition; and the [`ProcedureRun`] the runner hands back.
//!
//! **Position:** implemented in `fleet_procedure/`, `discord_procedure/` and `load_procedure/`;
//! read by `staging_dispatch.rs` (action lists, preflight) and `recording.rs` (the recorded run).
//!
//! **Signals & state:** none; the trait's implementors are stateless values carrying their
//! command-line options.
//!
//! **Invariants:** a plan that declares no case, names a case twice, maps an effect to an
//! undeclared case, leaves a runnable case without an effect, maps an effect to a not-run case,
//! or counts a deadline from a request row its step does not observe is refused before any
//! receipt is touched.

use std::collections::BTreeSet;

use crate::error::{Result, bail, ensure};
use serde_json::{Value, json};

use super::runner::{ProcedureRunner, RunContext};
use super::step::{DeadlineAnchor, DeclaredCase, Measurements, Step, StepKind, identifier};
use crate::operator_coordination::action_list::PlannedAction;
use crate::procedure_receipts::{
    EnvironmentEntry, FixtureManifest, ObservationRecord, Observations, RecordedCase, StagingCheck,
};
use crate::remote_observers::remote_command::HostCommandRunner;
use crate::staging_settings::StagingSettings;
use crate::support_commands::preflight::PreflightCheck;

/// A procedure's declared cases and steps, and the limits of its run.
pub(crate) struct ProcedurePlan {
    /// Every case the receipt declares, in declaration order.
    pub declared_cases: Vec<DeclaredCase>,
    /// The steps in the order they run.
    pub steps: Vec<Step>,
    /// Seconds from the run's start after which no step starts and no effect waits longer.
    pub hard_stop_seconds: u64,
    /// Seconds between two polls of a pending probe.
    pub poll_interval_seconds: u64,
}

impl ProcedurePlan {
    /// Refuses a plan the recorder could not judge honestly; see the module invariants.
    pub(crate) fn validate(&self) -> Result<()> {
        ensure!(
            !self.declared_cases.is_empty(),
            "the procedure declares no cases"
        );
        ensure!(
            self.hard_stop_seconds > 0 && self.poll_interval_seconds > 0,
            "the hard stop and the poll interval must be positive"
        );
        let mut declared = BTreeSet::new();
        for case in &self.declared_cases {
            ensure!(
                declared.insert(case.name.as_str()),
                "case {} is declared twice",
                case.name.as_str()
            );
        }
        let mut steps = BTreeSet::new();
        let mut decided = BTreeSet::new();
        for step in &self.steps {
            ensure!(
                steps.insert(step.id.as_str()),
                "step {} appears twice",
                step.id.as_str()
            );
            let mut effects = BTreeSet::new();
            for effect in &step.effects {
                let at = format!("{}.{}", step.id.as_str(), effect.id);
                ensure!(
                    identifier(&effect.id) && effects.insert(effect.id.as_str()),
                    "effect {at} needs a unique [a-z0-9_]+ id"
                );
                ensure!(
                    declared.contains(effect.case.as_str()),
                    "effect {at} decides undeclared case {}",
                    effect.case.as_str()
                );
                ensure!(
                    effect.deadline.anchor == DeadlineAnchor::StepStart || step.request.is_some(),
                    "effect {at} counts from a request row its step does not observe"
                );
                decided.insert(effect.case.as_str());
            }
        }
        for case in &self.declared_cases {
            let name = case.name.as_str();
            match (&case.unavailable_dependency, decided.contains(name)) {
                (None, false) => bail!("no effect decides case {name}"),
                (Some(_), true) => {
                    bail!("case {name} is declared not run but an effect decides it")
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// The plan as the fixture manifest records it: the steps with their kinds, instructions,
    /// requests, effects, deadlines and cases, and the declared cases.
    pub(crate) fn definition(&self) -> Value {
        let steps: Vec<Value> = self
            .steps
            .iter()
            .map(|step| {
                let effects: Vec<Value> = step
                    .effects
                    .iter()
                    .map(|effect| {
                        json!({
                            "id": effect.id,
                            "description": effect.description,
                            "case": effect.case.as_str(),
                            "deadline_seconds": effect.deadline.seconds,
                            "deadline_from": match effect.deadline.anchor {
                                DeadlineAnchor::RequestRow => "request_row",
                                DeadlineAnchor::StepStart => "step_start",
                            },
                            "reads_browser": effect.probe.reads_browser(),
                            "source": effect.probe.source_name(),
                        })
                    })
                    .collect();
                json!({
                    "id": step.id.as_str(),
                    "kind": step.kind.name(),
                    "instruction": step.instruction,
                    "host_action": match &step.kind {
                        StepKind::HostAction(command) => Value::String(command.command_line.clone()),
                        _ => Value::Null,
                    },
                    "request": step.request.as_ref().map(|request| request.description.clone()),
                    "effects": effects,
                })
            })
            .collect();
        let cases: Vec<Value> = self
            .declared_cases
            .iter()
            .map(|case| {
                json!({
                    "name": case.name.as_str(),
                    "unavailable_dependency": case.unavailable_dependency,
                })
            })
            .collect();
        json!({
            "hard_stop_seconds": self.hard_stop_seconds,
            "poll_interval_seconds": self.poll_interval_seconds,
            "declared_cases": cases,
            "steps": steps,
        })
    }
}

/// What a run of a plan produced.
#[derive(Debug)]
pub(crate) struct ProcedureRun {
    /// Every declared case in declaration order, with its status.
    pub cases: Vec<RecordedCase>,
    /// The deciding observations, one `observation:` log line each.
    pub journal: Vec<ObservationRecord>,
    /// Everything the probes measured.
    pub measurements: Measurements,
}

/// A staging procedure: the cases it declares, how it runs, and what its receipt observes.
pub(crate) trait StagingProcedure {
    /// The staging check this procedure records.
    fn check(&self) -> StagingCheck;

    /// The numbered real actions the operator approves before a run.
    fn action_list(&self, settings: &StagingSettings) -> Vec<PlannedAction>;

    /// The numbered actions that put the host back after a stopped run.
    fn recovery_action_list(&self, settings: &StagingSettings) -> Vec<PlannedAction>;

    /// The procedure's own read-only preconditions, which `staging preflight` checks.
    fn preflight_checks(&self, settings: &StagingSettings) -> Vec<PreflightCheck>;

    /// The declared cases and the steps of one run.
    fn plan(&self, settings: &StagingSettings) -> Result<ProcedurePlan>;

    /// The run's fixture identities (server ids, mission and artifact digests, fleet scenario
    /// rows, guild ids, the Workshop version, committed data digests), read before the first
    /// step; the manifest holds them beside the plan's definition.
    fn fixture_identities(
        &self,
        settings: &StagingSettings,
        host: &mut dyn HostCommandRunner,
    ) -> Result<Value>;

    /// Runs the plan. The generic runner awaits each step's effects; a procedure whose
    /// measurement is not a sequence of awaited effects replaces it.
    fn run<'a>(&self, plan: &'a ProcedurePlan, context: RunContext<'a>) -> Result<ProcedureRun> {
        ProcedureRunner::new(plan, context).run()
    }

    /// The measured values the receipt carries, whatever the verdict, citing `manifest`.
    fn observations(&self, run: &ProcedureRun, manifest: &FixtureManifest) -> Observations;

    /// The preconditions the procedure stages instead of waiting for them, as the receipt's
    /// `staged_precondition=<name>` environment entries; none by default.
    fn staged_preconditions(&self) -> Result<Vec<EnvironmentEntry>> {
        Ok(Vec::new())
    }
}
