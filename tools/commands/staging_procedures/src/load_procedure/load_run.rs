//! The load procedure's run: its workstation actions between the plan's observations.
//!
//! **Role:** [`run_load`] walks the plan's steps in order: before a step's effects are judged it
//! performs the step's workstation action (reading the token file's shape, sending the keying
//! refreshes, running the member load on its own thread while the heartbeat census is read every
//! minute), then polls each effect through the engine's `observe` until it holds, is
//! contradicted or passes its deadline, and ends with one status per declared case.
//!
//! **Position:** `LoadProcedure::run` replaces the generic runner with this, since the member load
//! is not an awaited effect; the probes, the journal and the log records are the engine's.
//!
//! **Signals & state:** the run's measurements, its deciding observations, the failures of each
//! case and the reasons a measurement could not be taken; the engine thread owns only its plan.
//!
//! **Invariants:** the run never reads stdin; a workstation action that fails fails the effects
//! that needed its measurement, naming why; every effect passes through `observe`, so every
//! observation is journaled; a census sample is taken at the load's start and end and once a
//! minute between them; no token enters a measurement, the journal or the output.

use std::collections::BTreeMap;
use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use crate::error::{Result, ResultExt, refusal};
use staging_load_plan::{FixtureEvent, LoadRunPlan};

use super::committed_load_data::CommittedLoadData;
use super::game_operations::{CENSUS_MEASUREMENT, CensusSample, HeartbeatCensus};
use super::load_preconditions::{
    ACCOUNT_FILE_MEASUREMENT, FIXTURE_EVENTS_MEASUREMENT, KEYING_MEASUREMENT, account_file_shape,
    send_keying_refreshes,
};
use super::load_queries::FLEET_HEARTBEAT_CENSUS;
use super::load_report_judges::REPORT_MEASUREMENT;
use super::load_steps::{KEYING_STEP, MEMBER_LOAD_STEP, POPULATION_STEP, source_addresses};
use super::workstation_load::WorkstationLoad;
use crate::environment_identity::load_generator_identity;
use crate::observation_journal::journal::JournalEntry;
use crate::operator_coordination::awaited_effect::{await_line, effect_line};
use crate::procedure_receipts::{CaseStatus, ObservationRecord, RecordedCase};
use crate::procedure_runner::probe_reading::{ProbeWindow, observe};
use crate::procedure_runner::procedure::{ProcedurePlan, ProcedureRun};
use crate::procedure_runner::runner::RunContext;
use crate::procedure_runner::step::{
    EffectPredicate, Measurements, ProbeSource, ProbeVerdict, Step,
};
use crate::remote_observers::database_reader::select;
use crate::staging_settings::StagingSettings;

/// Seconds between two heartbeat census reads while the member load runs.
pub(crate) const CENSUS_INTERVAL_SECONDS: u64 = 60;
/// The measurement naming the load generator's hardware.
pub(crate) const HARDWARE_MEASUREMENT: &str = "load_generator_hardware";
/// The measurement naming the load generator's network.
pub(crate) const NETWORK_MEASUREMENT: &str = "load_generator_network";

/// What a load run needs besides its plan and its context.
pub(crate) struct LoadRunInputs<'a> {
    pub data: &'a CommittedLoadData,
    pub settings: &'a StagingSettings,
    /// The token file the member load reads once.
    pub token_file: &'a Path,
    pub workstation: &'a Arc<dyn WorkstationLoad>,
}

/// Runs the load plan: see the module documentation.
pub(crate) fn run_load(
    plan: &ProcedurePlan,
    mut context: RunContext<'_>,
    inputs: &LoadRunInputs<'_>,
) -> Result<ProcedureRun> {
    let mut run = LoadExecution {
        plan,
        measurements: Measurements::new(),
        unavailable: BTreeMap::new(),
        records: Vec::new(),
        failures: BTreeMap::new(),
    };
    run.measurements.insert(
        HARDWARE_MEASUREMENT.into(),
        load_generator_identity::hardware().into(),
    );
    run.measurements.insert(
        NETWORK_MEASUREMENT.into(),
        load_generator_identity::network(inputs.settings).into(),
    );
    let hard_stop = context
        .clock
        .now_unix_ms()
        .saturating_add(plan.hard_stop_seconds.saturating_mul(1000));
    for step in &plan.steps {
        let started = context.clock.now_unix_ms();
        writeln!(
            context.output,
            "{}",
            await_line(step.id.as_str(), &step.instruction)
        )?;
        if started >= hard_stop {
            let why = format!(
                "not reached: the run stopped at its {} s hard stop",
                plan.hard_stop_seconds
            );
            for effect in &step.effects {
                run.fail(&mut context, step, effect, &why, None)?;
            }
            continue;
        }
        run.workstation_action(step.id.as_str(), &mut context, inputs);
        run.judge_effects(&mut context, step, started, hard_stop)?;
    }
    let cases = plan
        .declared_cases
        .iter()
        .map(|declared| RecordedCase {
            name: declared.name.clone(),
            status: match (
                &declared.unavailable_dependency,
                run.failures.get(declared.name.as_str()),
            ) {
                (Some(missing), _) => CaseStatus::NotRun {
                    missing: missing.clone(),
                },
                (None, Some(reasons)) => CaseStatus::Failed(reasons.join("; ")),
                (None, None) => CaseStatus::Ok,
            },
        })
        .collect();
    Ok(ProcedureRun {
        cases,
        journal: run.records,
        measurements: run.measurements,
    })
}

struct LoadExecution<'p> {
    plan: &'p ProcedurePlan,
    measurements: Measurements,
    /// Measurement → why the workstation could not take it.
    unavailable: BTreeMap<String, String>,
    records: Vec<ObservationRecord>,
    failures: BTreeMap<String, Vec<String>>,
}

impl LoadExecution<'_> {
    fn workstation_action(
        &mut self,
        step: &str,
        context: &mut RunContext<'_>,
        inputs: &LoadRunInputs<'_>,
    ) {
        match step {
            POPULATION_STEP => match account_file_shape(inputs.token_file)
                .and_then(|shape| Ok(serde_json::to_value(shape)?))
            {
                Ok(shape) => self.measure(ACCOUNT_FILE_MEASUREMENT, shape),
                Err(error) => self.cannot_measure(ACCOUNT_FILE_MEASUREMENT, format!("{error:#}")),
            },
            KEYING_STEP => {
                let origin = inputs
                    .settings
                    .load_target_origin
                    .clone()
                    .unwrap_or_default();
                let answers = send_keying_refreshes(
                    inputs.workstation,
                    &origin,
                    &source_addresses(inputs.settings),
                );
                match serde_json::to_value(answers) {
                    Ok(value) => self.measure(KEYING_MEASUREMENT, value),
                    Err(error) => self.cannot_measure(KEYING_MEASUREMENT, error.to_string()),
                }
            }
            MEMBER_LOAD_STEP => {
                if let Err(error) = self.member_load(context, inputs) {
                    let why = format!("{error:#}");
                    self.cannot_measure(REPORT_MEASUREMENT, why.clone());
                    if !self.measurements.contains_key(CENSUS_MEASUREMENT) {
                        self.cannot_measure(CENSUS_MEASUREMENT, why);
                    }
                }
            }
            _ => {}
        }
    }

    /// Runs the member load on its own thread and the heartbeat census on this one.
    fn member_load(
        &mut self,
        context: &mut RunContext<'_>,
        inputs: &LoadRunInputs<'_>,
    ) -> Result<()> {
        let plan = self.member_load_plan(inputs)?;
        let workstation = Arc::clone(inputs.workstation);
        let engine = std::thread::Builder::new()
            .name("staging-load-engine".into())
            .spawn(move || workstation.member_load(&plan))
            .context("starting the load engine thread")?;
        let container = inputs.settings.database_container.clone();
        let mut census = HeartbeatCensus {
            expected_servers: inputs.settings.fleet.instance_count,
            samples: Vec::new(),
        };
        loop {
            census.samples.push(census_sample(context, &container));
            let mut waited = 0;
            while waited < CENSUS_INTERVAL_SECONDS && !engine.is_finished() {
                context.clock.sleep(Duration::from_secs(1));
                waited += 1;
            }
            if engine.is_finished() {
                break;
            }
        }
        census.samples.push(census_sample(context, &container));
        self.measure(CENSUS_MEASUREMENT, serde_json::to_value(&census)?);
        let report = engine
            .join()
            .map_err(|_| refusal!("the load engine thread stopped abnormally"))?
            .context("the load engine did not run")?;
        writeln!(
            context.output,
            "member load: {} requests sent, {} completed in the measured window, {} unexpected",
            report.requests_sent, report.completed_requests, report.unexpected_errors.total
        )?;
        self.measure(REPORT_MEASUREMENT, serde_json::to_value(&report)?);
        Ok(())
    }

    /// The engine's plan: the committed workload, the target, the addresses, the token file
    /// and the fixture events the `population` step measured.
    fn member_load_plan(&self, inputs: &LoadRunInputs<'_>) -> Result<LoadRunPlan> {
        let fixture_events: Vec<FixtureEvent> = serde_json::from_value(
            self.measurements
                .get(FIXTURE_EVENTS_MEASUREMENT)
                .cloned()
                .context("the population step measured no fixture events")?,
        )
        .context("the measured fixture events do not decode")?;
        let target_origin = inputs
            .settings
            .load_target_origin
            .clone()
            .context("TBD_LOAD_TARGET_ORIGIN is not set in deploy.env")?;
        let source_addresses: Vec<IpAddr> = source_addresses(inputs.settings);
        let plan = LoadRunPlan {
            workload: inputs.data.workload.clone(),
            target_origin,
            source_addresses,
            account_file: inputs.token_file.to_path_buf(),
            fixture_events,
        };
        plan.validate()
            .context("the load engine refuses the run's plan")?;
        Ok(plan)
    }

    fn measure(&mut self, key: &str, value: serde_json::Value) {
        self.measurements.insert(key.to_string(), value);
    }

    fn cannot_measure(&mut self, key: &str, why: String) {
        self.unavailable.insert(key.to_string(), why);
    }

    /// Polls each effect of `step` until it holds, is contradicted or passes its deadline.
    fn judge_effects(
        &mut self,
        context: &mut RunContext<'_>,
        step: &Step,
        started: u64,
        hard_stop: u64,
    ) -> Result<()> {
        for effect in &step.effects {
            if let ProbeSource::Measurement(key) = &effect.probe.source
                && let Some(why) = self.unavailable.get(key).cloned()
            {
                self.fail(
                    context,
                    step,
                    effect,
                    &format!("{} could not be measured: {why}", effect.description),
                    None,
                )?;
                continue;
            }
            let deadline = started.saturating_add(effect.deadline.seconds.saturating_mul(1000));
            let limit = deadline.min(hard_stop);
            let window = ProbeWindow {
                started,
                request_unix_ms: None,
                window_end: limit,
            };
            loop {
                let observed = observe(
                    context,
                    &self.measurements,
                    step,
                    &effect.id,
                    &effect.probe,
                    window,
                )?;
                let now = context.clock.now_unix_ms();
                let sha256 = observed.sha256.as_deref();
                match observed.verdict {
                    ProbeVerdict::Satisfied(satisfaction) => {
                        let label = format!("{}.{}", step.id.as_str(), effect.id);
                        if let Some(sha256) = sha256 {
                            self.records.push(ObservationRecord::new(
                                &label,
                                &observed.observer,
                                &satisfaction.summary,
                                sha256,
                            )?);
                        }
                        writeln!(
                            context.output,
                            "{}",
                            effect_line(step.id.as_str(), &effect.id, Ok(&satisfaction.summary))
                        )?;
                        self.measurements.extend(satisfaction.measurements);
                        break;
                    }
                    ProbeVerdict::Contradicted(why) => {
                        let why = format!("{}: {why}", effect.description);
                        self.fail(
                            context,
                            step,
                            effect,
                            &why,
                            sha256.map(|sha| (observed.observer.as_str(), sha)),
                        )?;
                        break;
                    }
                    ProbeVerdict::Pending(seen) if now >= limit => {
                        let cause = if limit < deadline {
                            "the run's hard stop came"
                        } else {
                            "its deadline passed"
                        };
                        let why = format!(
                            "{}: {cause} {} s after the step's start (last seen: {seen})",
                            effect.description,
                            limit.saturating_sub(started) / 1000
                        );
                        self.fail(
                            context,
                            step,
                            effect,
                            &why,
                            sha256.map(|sha| (observed.observer.as_str(), sha)),
                        )?;
                        break;
                    }
                    ProbeVerdict::Pending(_) => context
                        .clock
                        .sleep(Duration::from_secs(self.plan.poll_interval_seconds)),
                }
            }
        }
        Ok(())
    }

    /// Fails `effect`'s case with `why`, citing the observation that showed it when there is one.
    fn fail(
        &mut self,
        context: &mut RunContext<'_>,
        step: &Step,
        effect: &EffectPredicate,
        why: &str,
        evidence: Option<(&str, &str)>,
    ) -> Result<()> {
        let label = format!("{}.{}", step.id.as_str(), effect.id);
        if let Some((observer, sha256)) = evidence {
            self.records.push(ObservationRecord::new(
                &label,
                observer,
                &format!("FAILED {why}"),
                sha256,
            )?);
        }
        writeln!(
            context.output,
            "{}",
            effect_line(step.id.as_str(), &effect.id, Err(why))
        )?;
        self.failures
            .entry(effect.case.as_str().to_string())
            .or_default()
            .push(format!("{label}: {why}"));
        Ok(())
    }
}

/// One read of the heartbeat census, journaled; a failed read is a sample naming why.
fn census_sample(context: &mut RunContext<'_>, container: &str) -> CensusSample {
    let observed_unix_ms = context.clock.now_unix_ms();
    let failed = |error: String| CensusSample {
        observed_unix_ms,
        artifact_sha256: None,
        servers: Vec::new(),
        error: Some(error),
    };
    let command = match select(container, &FLEET_HEARTBEAT_CENSUS, &[]) {
        Ok(command) => command,
        Err(error) => return failed(format!("{error:#}")),
    };
    match context.host.run(&command) {
        Ok(output) if output.exit_code == 0 => {
            let entry = JournalEntry {
                step: "member_load.census",
                observer: command.observer,
                observed_unix_ms,
                summary: "fleet heartbeat census",
                verdict: "measured",
                artifact: output.stdout.as_bytes(),
            };
            match context.journal.archive(&entry) {
                Ok(sha256) => CensusSample::of_output(observed_unix_ms, sha256, &output.stdout),
                Err(error) => failed(format!("the census read could not be archived: {error:#}")),
            }
        }
        Ok(output) => failed(format!("{} exited {}", command.observer, output.exit_code)),
        Err(error) => failed(format!("{error:#}")),
    }
}
