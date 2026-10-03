//! The generic procedure engine: runs each step, polls its probes, holds every effect to its
//! deadline, journals every observation and maps the effects to the declared cases.
//!
//! **Role:** [`ProcedureRunner::run`] walks a [`ProcedurePlan`]: it prints each step's `AWAIT`
//! line, runs a host action, waits for the request row, polls every effect until it holds, is
//! contradicted or misses its deadline, and ends with one status per declared case.
//!
//! **Position:** called through [`super::procedure::StagingProcedure::run`] by `recording.rs`;
//! takes each observation through `probe_reading.rs`, reads through a [`HostCommandRunner`] and
//! the browser inbox, keeps time with a [`Clock`], and writes to the [`ObservationJournal`].
//!
//! **Signals & state:** the run's measurements, its deciding observations and the failures of
//! each case, owned by the runner until `run` returns them.
//!
//! **Invariants:** the runner never reads stdin; a request row may take up to
//! [`REQUEST_APPEARANCE_SECONDS`] to appear, and deadlines count from its time; an effect not
//! satisfied by its deadline (or by the hard stop) makes its case `FAILED`, as does a
//! contradiction; a probe whose read would change the host is a contradiction; every
//! observation is journaled, and the observations that decide an effect become log records.

use std::collections::BTreeMap;
use std::io::Write;
use std::time::Duration;

use crate::error::Result;

use super::clock::Clock;
use super::probe_reading::{ProbeWindow, observe};
use super::procedure::{ProcedurePlan, ProcedureRun};
use super::step::{
    DeadlineAnchor, Measurements, Probe, ProbeVerdict, Satisfaction, Step, StepKind,
};
use crate::observation_journal::browser_inbox::BrowserInbox;
use crate::observation_journal::journal::{JournalEntry, ObservationJournal};
use crate::operator_coordination::awaited_effect::{await_line, effect_line, inbox_hint};
use crate::remote_observers::remote_command::HostCommandRunner;
use api_readiness_checks::operational_recording::{CaseStatus, ObservationRecord, RecordedCase};

/// How long the orchestrator's request may take to appear after a step's `AWAIT` line.
pub(crate) const REQUEST_APPEARANCE_SECONDS: u64 = 900;

/// What a run reads, waits with, and writes to.
pub(crate) struct RunContext<'a> {
    pub host: &'a mut dyn HostCommandRunner,
    pub clock: &'a dyn Clock,
    pub journal: &'a mut ObservationJournal,
    pub inbox: &'a BrowserInbox,
    /// Where the `AWAIT` and outcome lines go: stdout in a live run.
    pub output: &'a mut dyn Write,
}

/// Runs one plan.
pub(crate) struct ProcedureRunner<'a> {
    plan: &'a ProcedurePlan,
    context: RunContext<'a>,
    measurements: Measurements,
    records: Vec<ObservationRecord>,
    failures: BTreeMap<String, Vec<String>>,
}

impl<'a> ProcedureRunner<'a> {
    /// A runner of `plan` over `context`.
    pub(crate) fn new(plan: &'a ProcedurePlan, context: RunContext<'a>) -> Self {
        Self {
            plan,
            context,
            measurements: Measurements::new(),
            records: Vec::new(),
            failures: BTreeMap::new(),
        }
    }

    /// Runs every step and returns each declared case's status, the deciding observations and
    /// the measurements.
    pub(crate) fn run(mut self) -> Result<ProcedureRun> {
        let plan = self.plan;
        let hard_stop = self
            .context
            .clock
            .now_unix_ms()
            .saturating_add(plan.hard_stop_seconds.saturating_mul(1000));
        for step in &plan.steps {
            if self.context.clock.now_unix_ms() >= hard_stop {
                let why = format!(
                    "not reached: the run stopped at its {} s hard stop",
                    plan.hard_stop_seconds
                );
                self.fail_step(step, &why)?;
                continue;
            }
            self.run_step(step, hard_stop)?;
        }
        let cases = plan
            .declared_cases
            .iter()
            .map(|declared| RecordedCase {
                name: declared.name.clone(),
                status: match (
                    &declared.unavailable_dependency,
                    self.failures.get(declared.name.as_str()),
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
            journal: self.records,
            measurements: self.measurements,
        })
    }

    fn run_step(&mut self, step: &Step, hard_stop: u64) -> Result<()> {
        let started = self.context.clock.now_unix_ms();
        writeln!(
            self.context.output,
            "{}",
            await_line(step.id.as_str(), &step.instruction)
        )?;
        let reads_browser = step.effects.iter().any(|e| e.probe.reads_browser())
            || step
                .request
                .as_ref()
                .is_some_and(|r| r.probe.reads_browser());
        if reads_browser {
            let path = self.context.inbox.entry_path(step.id.as_str());
            writeln!(self.context.output, "{}", inbox_hint(&path))?;
        }
        if let StepKind::HostAction(command) = &step.kind {
            let label = format!("{}.action", step.id.as_str());
            let failure = match self.context.host.run(command) {
                Ok(output) => {
                    let summary = format!("exit {}", output.exit_code);
                    let sha256 = self.archive(
                        &label,
                        command.observer,
                        started,
                        &summary,
                        "host action",
                        output.stdout.as_bytes(),
                    )?;
                    self.record(&label, command.observer, &summary, &sha256)?;
                    (output.exit_code != 0)
                        .then(|| format!("the host action exited {}", output.exit_code))
                }
                Err(error) => Some(format!("the host action did not run: {error:#}")),
            };
            if let Some(why) = failure {
                return self.fail_step(step, &why);
            }
        }
        let request_unix_ms = match &step.request {
            None => None,
            Some(request) => {
                let window_end = hard_stop.min(started + REQUEST_APPEARANCE_SECONDS * 1000);
                match self.await_probe(
                    step,
                    "request",
                    &request.probe,
                    started,
                    None,
                    window_end,
                )? {
                    Ok(satisfaction) => Some(
                        satisfaction
                            .observed_unix_ms
                            .unwrap_or_else(|| self.context.clock.now_unix_ms()),
                    ),
                    Err(why) => {
                        let why = format!("{}: {why}", request.description);
                        return self.fail_step(step, &why);
                    }
                }
            }
        };
        let mut pending: Vec<usize> = (0..step.effects.len()).collect();
        loop {
            let mut still = Vec::new();
            for index in pending {
                let effect = &step.effects[index];
                let anchor = match effect.deadline.anchor {
                    DeadlineAnchor::RequestRow => request_unix_ms.unwrap_or(started),
                    DeadlineAnchor::StepStart => started,
                };
                let deadline = anchor.saturating_add(effect.deadline.seconds.saturating_mul(1000));
                let limit = deadline.min(hard_stop);
                let label = format!("{}.{}", step.id.as_str(), effect.id);
                let window = ProbeWindow {
                    started,
                    request_unix_ms,
                    window_end: limit,
                };
                let observed = observe(
                    &mut self.context,
                    &self.measurements,
                    step,
                    &effect.id,
                    &effect.probe,
                    window,
                )?;
                let now = self.context.clock.now_unix_ms();
                let outcome = match observed.verdict {
                    ProbeVerdict::Satisfied(satisfaction) => {
                        let at = satisfaction.observed_unix_ms.unwrap_or(now);
                        if at <= deadline {
                            self.decide(
                                &label,
                                &observed.observer,
                                &satisfaction.summary,
                                observed.sha256.as_deref(),
                            )?;
                            self.measurements.extend(satisfaction.measurements);
                            Some(Ok(satisfaction.summary))
                        } else {
                            Some(Err(format!(
                                "{} observed {} ms after its {} s deadline",
                                effect.description,
                                at - deadline,
                                effect.deadline.seconds
                            )))
                        }
                    }
                    ProbeVerdict::Contradicted(why) => {
                        Some(Err(format!("{}: {why}", effect.description)))
                    }
                    ProbeVerdict::Pending(seen) if now >= limit => {
                        let cause = if limit < deadline {
                            "the run's hard stop came"
                        } else {
                            "its deadline passed"
                        };
                        Some(Err(format!(
                            "{}: {cause} {} s after the {} (last seen: {seen})",
                            effect.description,
                            limit.saturating_sub(anchor) / 1000,
                            anchor_name(effect.deadline.anchor)
                        )))
                    }
                    ProbeVerdict::Pending(_) => {
                        still.push(index);
                        None
                    }
                };
                match outcome {
                    Some(Ok(summary)) => writeln!(
                        self.context.output,
                        "{}",
                        effect_line(step.id.as_str(), &effect.id, Ok(&summary))
                    )?,
                    Some(Err(why)) => {
                        if let Some(sha256) = observed.sha256.as_deref() {
                            self.record(
                                &label,
                                &observed.observer,
                                &format!("FAILED {why}"),
                                sha256,
                            )?;
                        }
                        writeln!(
                            self.context.output,
                            "{}",
                            effect_line(step.id.as_str(), &effect.id, Err(&why))
                        )?;
                        self.fail_case(effect.case.as_str(), format!("{label}: {why}"));
                    }
                    None => {}
                }
            }
            if still.is_empty() {
                return Ok(());
            }
            pending = still;
            self.context
                .clock
                .sleep(Duration::from_secs(self.plan.poll_interval_seconds));
        }
    }

    /// Polls `probe` until it is satisfied (`Ok`), contradicted or past `window_end` (`Err`).
    fn await_probe(
        &mut self,
        step: &Step,
        probe_id: &str,
        probe: &Probe,
        started: u64,
        request_unix_ms: Option<u64>,
        window_end: u64,
    ) -> Result<std::result::Result<Satisfaction, String>> {
        let label = format!("{}.{probe_id}", step.id.as_str());
        let window = ProbeWindow {
            started,
            request_unix_ms,
            window_end,
        };
        loop {
            let observed = observe(
                &mut self.context,
                &self.measurements,
                step,
                probe_id,
                probe,
                window,
            )?;
            let now = self.context.clock.now_unix_ms();
            let failure = match observed.verdict {
                ProbeVerdict::Satisfied(satisfaction) => {
                    self.decide(
                        &label,
                        &observed.observer,
                        &satisfaction.summary,
                        observed.sha256.as_deref(),
                    )?;
                    self.measurements.extend(satisfaction.measurements.clone());
                    return Ok(Ok(satisfaction));
                }
                ProbeVerdict::Contradicted(why) => format!("refused: {why}"),
                ProbeVerdict::Pending(seen) if now >= window_end => format!(
                    "not observed within {} s of the step's start (last seen: {seen})",
                    window_end.saturating_sub(started) / 1000
                ),
                ProbeVerdict::Pending(_) => {
                    self.context
                        .clock
                        .sleep(Duration::from_secs(self.plan.poll_interval_seconds));
                    continue;
                }
            };
            if let Some(sha256) = observed.sha256.as_deref() {
                self.record(
                    &label,
                    &observed.observer,
                    &format!("FAILED {failure}"),
                    sha256,
                )?;
            }
            return Ok(Err(failure));
        }
    }

    fn archive(
        &mut self,
        label: &str,
        observer: &str,
        at: u64,
        summary: &str,
        verdict: &str,
        artifact: &[u8],
    ) -> Result<String> {
        self.context.journal.archive(&JournalEntry {
            step: label,
            observer,
            observed_unix_ms: at,
            summary,
            verdict,
            artifact,
        })
    }

    /// Adds a deciding observation to the log records; one without an archived artifact has
    /// nothing to cite and stays in the journal only.
    fn decide(
        &mut self,
        label: &str,
        observer: &str,
        summary: &str,
        sha256: Option<&str>,
    ) -> Result<()> {
        match sha256 {
            Some(sha256) => self.record(label, observer, summary, sha256),
            None => Ok(()),
        }
    }

    fn record(&mut self, label: &str, observer: &str, summary: &str, sha256: &str) -> Result<()> {
        self.records
            .push(ObservationRecord::new(label, observer, summary, sha256)?);
        Ok(())
    }

    fn fail_case(&mut self, case: &str, why: String) {
        self.failures.entry(case.to_string()).or_default().push(why);
    }

    /// Fails every effect of `step` with `why`.
    fn fail_step(&mut self, step: &Step, why: &str) -> Result<()> {
        for effect in &step.effects {
            writeln!(
                self.context.output,
                "{}",
                effect_line(step.id.as_str(), &effect.id, Err(why))
            )?;
            let label = format!("{}.{}", step.id.as_str(), effect.id);
            self.fail_case(effect.case.as_str(), format!("{label}: {why}"));
        }
        Ok(())
    }
}

fn anchor_name(anchor: DeadlineAnchor) -> &'static str {
    match anchor {
        DeadlineAnchor::RequestRow => "request row",
        DeadlineAnchor::StepStart => "step's start",
    }
}
