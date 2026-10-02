//! A recorded run: the procedure's plan executed between `RecordingSession::begin` and `finish`,
//! so the receipt, the log, the manifest and the journal all come from one run.
//!
//! **Role:** validates the plan, opens the recording, creates the run folder with its journal and
//! browser inbox, reads the environment identities and the fixture manifest, runs the procedure,
//! and hands the outcome to the recorder.
//!
//! **Position:** called by `dispatch.rs`, which hands it the process environment, for
//! `fleet --record`, `discord --record` and `load --record`; drives `runner.rs` through
//! [`StagingProcedure::run`] and `crate::verifications::api_readiness::operational_recording`
//! for the receipt.
//!
//! **Signals & state:** owns the recording session, the journal and the inbox for one run.
//!
//! **Invariants:** a plan the recorder could not judge honestly is refused before `begin`, and a
//! process environment the run discipline refuses is refused by `begin` before anything else, so
//! either refusal leaves the earlier receipt and writes no run folder, log or receipt; once
//! `begin` has run, every outcome ends in `finish`: a failure inside the run becomes a failing
//! receipt naming it, never a missing one; the exit code is the recorder's (0 only on PASS).

use std::ffi::OsString;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::{Value, json};

use super::clock::Clock;
use super::procedure::{ProcedurePlan, ProcedureRun, StagingProcedure};
use super::runner::RunContext;
use crate::commands::staging::environment_identity;
use crate::commands::staging::observation_journal::browser_inbox::BrowserInbox;
use crate::commands::staging::observation_journal::journal::ObservationJournal;
use crate::commands::staging::remote_observers::remote_command::HostCommandRunner;
use crate::commands::staging::run_identity::{EVIDENCE_DIRECTORY, RunIdentity};
use crate::commands::staging::staging_settings::StagingSettings;
use crate::verifications::api_readiness::operational_recording::{
    CaseStatus, EnvironmentEntry, FixtureManifest, RecordedCase, RecordedOutcome, RecordingSession,
};

/// What a recorded run reads and writes besides the procedure.
pub(crate) struct RecordingInputs<'a> {
    /// The repository root the run binds to.
    pub root: &'a Path,
    pub settings: &'a StagingSettings,
    pub host: &'a mut dyn HostCommandRunner,
    pub clock: &'a dyn Clock,
    /// The command line the receipt records.
    pub command: Vec<String>,
    /// The variables of the process the recording runs in, which the run discipline reads.
    pub process_environment: Vec<(OsString, OsString)>,
    /// Where the `AWAIT` and verdict lines go.
    pub output: &'a mut dyn Write,
}

/// Records one run of `procedure` and returns the recorder's exit code.
pub(crate) fn record(procedure: &dyn StagingProcedure, inputs: RecordingInputs<'_>) -> Result<u8> {
    let check = procedure.check();
    let plan = procedure.plan(inputs.settings)?;
    plan.validate()
        .with_context(|| format!("the {} procedure cannot be recorded", check.id()))?;
    let session = RecordingSession::begin(
        inputs.root,
        Path::new(EVIDENCE_DIRECTORY),
        check,
        inputs.command,
        inputs.process_environment,
    )?;
    let identity = RunIdentity::new(inputs.root, check, session.run_id());
    writeln!(
        inputs.output,
        "staging-run: {} run={} journal={}",
        check.id(),
        identity.run_id,
        identity.directory.display()
    )?;
    let outcome = match run_recorded(
        procedure,
        &plan,
        &identity,
        inputs.settings,
        inputs.host,
        inputs.clock,
        inputs.output,
    ) {
        Ok(outcome) => outcome,
        Err(error) => stopped_outcome(procedure, &plan, &error)?,
    };
    let receipt = session.finish(outcome)?;
    writeln!(inputs.output, "{}", receipt.summary)?;
    Ok(receipt.exit_code)
}

/// The run itself, between `begin` and `finish`.
fn run_recorded(
    procedure: &dyn StagingProcedure,
    plan: &ProcedurePlan,
    identity: &RunIdentity,
    settings: &StagingSettings,
    host: &mut dyn HostCommandRunner,
    clock: &dyn Clock,
    output: &mut dyn Write,
) -> Result<RecordedOutcome> {
    let mut journal = ObservationJournal::create(&identity.directory)?;
    std::fs::create_dir_all(identity.inbox_directory())?;
    let inbox = BrowserInbox::new(&identity.inbox_directory());
    let mut environment = environment_identity::collect(
        settings,
        host,
        &mut journal,
        clock.now_unix_ms(),
        procedure.check(),
    )?;
    environment.extend(procedure.staged_preconditions()?);
    let identities = procedure
        .fixture_identities(settings, host)
        .unwrap_or_else(|error| json!({ "unavailable": format!("{error:#}") }));
    let fixture_manifest = manifest(procedure, plan, identities)?;
    let context = RunContext {
        host,
        clock,
        journal: &mut journal,
        inbox: &inbox,
        output,
    };
    let run = procedure
        .run(plan, context)
        .unwrap_or_else(|error| stopped_run(plan, &error));
    Ok(RecordedOutcome {
        observations: procedure.observations(&run, &fixture_manifest),
        cases: run.cases,
        environment,
        fixture_manifest,
        journal: run.journal,
    })
}

/// The manifest: the check, the plan's definition and the procedure's fixture identities.
fn manifest(
    procedure: &dyn StagingProcedure,
    plan: &ProcedurePlan,
    identities: Value,
) -> Result<FixtureManifest> {
    FixtureManifest::new(&json!({
        "check": procedure.check().id(),
        "procedure": plan.definition(),
        "identities": identities,
    }))
}

/// A run that stopped with `error`: every runnable case failed naming it.
fn stopped_run(plan: &ProcedurePlan, error: &anyhow::Error) -> ProcedureRun {
    ProcedureRun {
        cases: plan
            .declared_cases
            .iter()
            .map(|declared| RecordedCase {
                name: declared.name.clone(),
                status: match &declared.unavailable_dependency {
                    Some(missing) => CaseStatus::NotRun {
                        missing: missing.clone(),
                    },
                    None => CaseStatus::Failed(format!("the run stopped: {error:#}")),
                },
            })
            .collect(),
        journal: Vec::new(),
        measurements: Default::default(),
    }
}

/// The outcome of a recording whose run folder or identities could not even be prepared.
fn stopped_outcome(
    procedure: &dyn StagingProcedure,
    plan: &ProcedurePlan,
    error: &anyhow::Error,
) -> Result<RecordedOutcome> {
    let fixture_manifest = manifest(
        procedure,
        plan,
        json!({ "unavailable": format!("{error:#}") }),
    )?;
    let run = stopped_run(plan, error);
    Ok(RecordedOutcome {
        observations: procedure.observations(&run, &fixture_manifest),
        cases: run.cases,
        environment: Vec::<EnvironmentEntry>::new(),
        fixture_manifest,
        journal: run.journal,
    })
}
