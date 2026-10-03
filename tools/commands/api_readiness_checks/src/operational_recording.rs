//! Staging recordings: one operational receipt per procedure run, judged before it is written.
//!
//! **Role:** Opens a recording of one staging check and, when its procedure ends, decides PASS or
//! FAIL, writes the log, the fixture manifest and the receipt, and returns the exit code.
//!
//! **Position:** The `cargo xtask staging` harness calls [`RecordingSession::begin`] with its
//! process environment before a procedure's first step and [`RecordingSession::finish`] with its
//! [`RecordedOutcome`], a partial run included. The receipts land in the evidence folder that
//! `cargo xtask verify api-readiness` judges through `evidence.rs`; the log grammar lives in
//! `operational_log.rs` and the measured thresholds in `operational.rs`.
//!
//! **Signals & state:** a session owns the start snapshot (the register's check definition, both
//! fingerprints, the start time, the tool versions and the run id) until `finish` consumes it.
//! `begin` removes the check's earlier receipt; `finish` writes `<id>.log`, `<id>.fixture.json`
//! and `<id>.json` through `evidence_storage.rs`, the receipt last.
//!
//! **Invariants:** a recording begins only for an `operational` check without a command whose
//! success marker is the passing verdict line's, and only when the environment `begin` is given
//! leaves `TEST_DATABASE_URL`, `DEPLOY_ENV` and every `PROPTEST_*` variable unset: the run
//! discipline reads the variables `begin` is given, never `std::env`, and refuses before the
//! register is read or the earlier receipt is removed. A run passes only when both fingerprints
//! still match the start snapshot, every declared case is ok and named once, the fleet and
//! Discord observations cite the manifest's digest, `operational.rs` accepts the measurements,
//! the duration is within the check's timeout, and `evidence::validate` accepts the exact receipt
//! and log. Anything else is written as a FAIL: exit code 1, the start digests, the real
//! observations, the missing dependencies named, and no success marker anywhere in the log.

use crate::error::{Result, ResultExt, ensure};
use crate::{
    evidence::{self, Receipt},
    evidence_storage, fingerprint, operational,
    operational_log::{self, LogVerdict, RunLog},
    register::{self, Check, EvidenceClass},
    tool_identity,
};
use deploy_settings::DEPLOY_ENV_OVERRIDE_VARIABLE;
use serde::Serialize;
use std::{
    collections::BTreeSet,
    ffi::OsString,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

pub use crate::operational::Observations;
pub use crate::operational_log::{
    CaseName, CaseStatus, EnvironmentEntry, ObservationRecord, RecordedCase,
};

/// Variables that stay unset while a staging check records, besides every `PROPTEST_*`:
/// the test database, and the override that would point the harness at a settings file the
/// configuration fingerprint does not cover.
const UNSET_VARIABLES: [&str; 2] = ["TEST_DATABASE_URL", DEPLOY_ENV_OVERRIDE_VARIABLE];

/// The three staging checks of the acceptance register, each recorded by one procedure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StagingCheck {
    /// `staging_fleet`: five game servers and two clients through every fleet scenario.
    Fleet,
    /// `staging_discord`: every Discord scenario.
    Discord,
    /// `staging_load`: the sustained load run.
    Load,
}

impl StagingCheck {
    /// The register id: `staging_fleet`, `staging_discord` or `staging_load`.
    pub fn id(self) -> &'static str {
        match self {
            Self::Fleet => "staging_fleet",
            Self::Discord => "staging_discord",
            Self::Load => "staging_load",
        }
    }

    /// The evidence file `<id>.<suffix>`.
    fn file(self, suffix: &str) -> String {
        format!("{}.{suffix}", self.id())
    }
}

/// The procedure's fixture manifest, serialized once: the digest the observations cite is the
/// digest of the bytes written to `<id>.fixture.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureManifest {
    bytes: Vec<u8>,
    sha256: String,
}

impl FixtureManifest {
    /// Serializes `manifest` as pretty JSON ending in a newline, and digests those bytes.
    pub fn new(manifest: &impl Serialize) -> Result<Self> {
        let mut bytes =
            serde_json::to_vec_pretty(manifest).context("serialize the fixture manifest")?;
        bytes.push(b'\n');
        Ok(Self {
            sha256: content_digest::sha256_hex(&bytes),
            bytes,
        })
    }

    /// The SHA-256 of the manifest bytes: the `fixture_sha256` fleet and Discord observations cite.
    pub fn sha256(&self) -> &str {
        &self.sha256
    }
}

/// Everything a procedure hands the recorder when it ends, a partial run included.
#[derive(Debug)]
pub struct RecordedOutcome {
    /// Every declared case in declaration order; one that could not run is `NotRun`.
    pub cases: Vec<RecordedCase>,
    /// Identities of the staging environment, never a secret.
    pub environment: Vec<EnvironmentEntry>,
    /// The measured values `operational.rs` judges, recorded whatever the verdict.
    pub observations: Observations,
    /// The procedure definition and fixture identities the run executed against.
    pub fixture_manifest: FixtureManifest,
    /// The journaled observations, one `observation:` line each.
    pub journal: Vec<ObservationRecord>,
}

/// What a finished recording wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedReceipt {
    /// 0 for PASS, 1 for FAIL.
    pub exit_code: u8,
    /// The log's verdict line.
    pub summary: String,
}

/// An open recording of one staging check: the snapshot its receipt is bound to.
#[derive(Debug)]
pub struct RecordingSession {
    root: PathBuf,
    evidence_directory: PathBuf,
    check: StagingCheck,
    definition: Check,
    command: Vec<String>,
    run_id: String,
    started_unix_seconds: u64,
    started_at: Instant,
    source_sha256: String,
    configuration_sha256: String,
    tool_versions: Vec<String>,
}

impl RecordingSession {
    /// Opens a recording of `check`, run by `argv` from `root`, whose evidence goes to
    /// `evidence_directory` (relative to `root` unless absolute). `environment` holds the
    /// variables of the process the recording runs in: the run discipline reads them, and only
    /// them, first, refusing one that sets `TEST_DATABASE_URL`, `DEPLOY_ENV` or any `PROPTEST_*`
    /// by its name, never its value. Then refuses a check the register does not declare as
    /// operational without a command, snapshots both fingerprints, the tool versions and the start
    /// time, and removes the check's earlier receipt.
    pub fn begin(
        root: &Path,
        evidence_directory: &Path,
        check: StagingCheck,
        argv: Vec<String>,
        environment: impl IntoIterator<Item = (OsString, OsString)>,
    ) -> Result<Self> {
        ensure_run_discipline(environment)?;
        ensure!(
            !argv.is_empty() && argv.iter().all(|argument| !argument.is_empty()),
            "a recording names the command that runs it"
        );
        let definition = register::read(root)?
            .checks
            .into_iter()
            .find(|candidate| candidate.id == check.id())
            .with_context(|| format!("the acceptance register declares no {}", check.id()))?;
        ensure!(
            definition.class == EvidenceClass::Operational && definition.command.is_none(),
            "{} is not an operational check without a local command",
            check.id()
        );
        let marker = operational_log::passing_marker(check.id());
        ensure!(
            definition.success_marker == marker,
            "{} must use `{marker}`, the success marker of a passing recording",
            check.id()
        );
        let evidence_directory = if evidence_directory.is_absolute() {
            evidence_directory.to_owned()
        } else {
            root.join(evidence_directory)
        };
        let source_sha256 = fingerprint::source(root)?;
        let configuration_sha256 = fingerprint::configuration(root)?;
        let tool_versions = tool_identity::versions(root)?;
        std::fs::create_dir_all(&evidence_directory)?;
        match std::fs::remove_file(evidence_directory.join(check.file("json"))) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let started_unix_seconds = crate::readiness_verification::now();
        Ok(Self {
            root: root.to_owned(),
            evidence_directory,
            check,
            definition,
            command: argv,
            run_id: run_id(started_unix_seconds),
            started_unix_seconds,
            started_at: Instant::now(),
            source_sha256,
            configuration_sha256,
            tool_versions,
        })
    }

    /// The run id the log header and the harness journal share.
    pub fn run_id(&self) -> &str {
        &self.run_id
    }

    /// Ends the recording: decides the verdict, has the judge confirm a PASS, and writes the log,
    /// the manifest and the receipt. Errors only when the evidence cannot be written.
    pub fn finish(self, outcome: RecordedOutcome) -> Result<RecordedReceipt> {
        let duration_milliseconds = self.started_at.elapsed().as_millis();
        let RecordedOutcome {
            cases,
            environment,
            observations,
            fixture_manifest,
            journal,
        } = outcome;
        let mut reasons = self.drift();
        reasons.extend(case_reasons(&cases));
        reasons.extend(acceptance_reasons(
            self.check,
            &observations,
            &fixture_manifest,
        ));
        let timeout_seconds = self.definition.timeout_seconds;
        if duration_milliseconds > u128::from(timeout_seconds) * 1000 {
            reasons.push(format!(
                "the run took {duration_milliseconds} ms, beyond the {timeout_seconds} s timeout"
            ));
        }
        let manifest_file = self.check.file("fixture.json");
        let log = RunLog {
            check: self.check.id(),
            run_id: &self.run_id,
            started_unix_seconds: self.started_unix_seconds,
            command: &self.command,
            environment: &environment,
            fixture_sha256: fixture_manifest.sha256(),
            manifest_file: &manifest_file,
            journal: &journal,
            cases: &cases,
        };
        let mut receipt = self.receipt(&environment, observations, duration_milliseconds);
        let passing = reasons.is_empty().then(|| log.render(&LogVerdict::Pass));
        if let Some(text) = &passing {
            receipt.output_sha256 = content_digest::sha256_hex(text.as_bytes());
            if let Err(error) = evidence::validate(
                &self.definition,
                &receipt,
                text,
                &self.source_sha256,
                &self.configuration_sha256,
                crate::readiness_verification::now(),
            ) {
                reasons.push(format!("judge: {error}"));
            }
        }
        let (text, verdict) = match passing {
            Some(text) if reasons.is_empty() => (text, LogVerdict::Pass),
            _ => {
                let text = log.render(&LogVerdict::Fail(&reasons));
                receipt.exit_code = 1;
                receipt.output_sha256 = content_digest::sha256_hex(text.as_bytes());
                (text, LogVerdict::Fail(&reasons))
            }
        };
        let directory = &self.evidence_directory;
        evidence_storage::write(directory, &receipt.output_file, text.as_bytes())?;
        evidence_storage::write(directory, &manifest_file, &fixture_manifest.bytes)?;
        evidence_storage::write(
            directory,
            &self.check.file("json"),
            &serde_json::to_vec_pretty(&receipt)?,
        )?;
        Ok(RecordedReceipt {
            exit_code: u8::from(receipt.exit_code != 0),
            summary: log.verdict_line(&verdict),
        })
    }

    /// The receipt of this run with a passing exit code and no output digest yet.
    fn receipt(
        &self,
        environment: &[EnvironmentEntry],
        observations: Observations,
        duration_milliseconds: u128,
    ) -> Receipt {
        Receipt {
            version: 1,
            check_id: self.check.id().to_owned(),
            class: EvidenceClass::Operational,
            source_sha256: self.source_sha256.clone(),
            configuration_sha256: self.configuration_sha256.clone(),
            command: self.command.clone(),
            tool_versions: self.tool_versions.clone(),
            started_unix_seconds: self.started_unix_seconds,
            duration_milliseconds,
            exit_code: 0,
            output_file: self.check.file("log"),
            output_sha256: String::new(),
            environment: environment
                .iter()
                .map(EnvironmentEntry::receipt_entry)
                .collect(),
            observations: Some(observations),
            property_runs: Vec::new(),
        }
    }

    /// One reason per fingerprint that no longer matches the start snapshot or cannot be
    /// recomputed.
    fn drift(&self) -> Vec<String> {
        [
            (
                "source",
                fingerprint::source(&self.root),
                &self.source_sha256,
            ),
            (
                "configuration",
                fingerprint::configuration(&self.root),
                &self.configuration_sha256,
            ),
        ]
        .into_iter()
        .filter_map(|(name, current, recorded)| match current {
            Ok(current) if &current == recorded => None,
            Ok(_) => Some(format!(
                "the {name} fingerprint changed during the recording"
            )),
            Err(error) => Some(format!(
                "the {name} fingerprint cannot be recomputed: {error}"
            )),
        })
        .collect()
    }
}

/// The source and configuration fingerprints of the checkout at `root` now: the digests a
/// recording begun here would snapshot, printed by `cargo xtask staging fingerprints`.
pub fn current_fingerprints(root: &Path) -> Result<(String, String)> {
    Ok((
        fingerprint::source(root)?,
        fingerprint::configuration(root)?,
    ))
}

/// Reasons the declared cases refuse a PASS: none declared, a name declared twice, a failed case,
/// a case not run.
fn case_reasons(cases: &[RecordedCase]) -> Vec<String> {
    let mut reasons = Vec::new();
    if cases.is_empty() {
        reasons.push("no declared cases".to_owned());
    }
    let mut names = BTreeSet::new();
    for case in cases {
        if !names.insert(&case.name) {
            reasons.push(format!("case {} is declared twice", case.name.as_str()));
        }
    }
    let failed = cases
        .iter()
        .filter(|case| matches!(case.status, CaseStatus::Failed(_)))
        .count();
    let not_run = cases
        .iter()
        .filter(|case| matches!(case.status, CaseStatus::NotRun { .. }))
        .count();
    if failed > 0 {
        reasons.push(format!("{failed} failed"));
    }
    if not_run > 0 {
        reasons.push(format!("{not_run} not run"));
    }
    reasons
}

/// Reasons the measurements refuse a PASS: `operational.rs` rejects them, or the fleet or Discord
/// observations cite a fixture other than the manifest written beside the receipt.
fn acceptance_reasons(
    check: StagingCheck,
    observations: &Observations,
    manifest: &FixtureManifest,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if let Err(error) = operational::validate(check.id(), observations) {
        reasons.push(format!("acceptance: {error}"));
    }
    if let Observations::Fleet { fixture_sha256, .. } | Observations::Discord { fixture_sha256, .. } =
        observations
        && fixture_sha256 != manifest.sha256()
    {
        reasons.push(format!(
            "the observations cite fixture {fixture_sha256}, not the manifest's {}",
            manifest.sha256()
        ));
    }
    reasons
}

/// Refuses a recording while `variables` set `TEST_DATABASE_URL`, `DEPLOY_ENV` or any
/// `PROPTEST_*`; the refusal names the variable, never its value.
fn ensure_run_discipline(variables: impl IntoIterator<Item = (OsString, OsString)>) -> Result<()> {
    for (name, _) in variables {
        let name = name.as_encoded_bytes();
        ensure!(
            !name.starts_with(b"PROPTEST_")
                && !UNSET_VARIABLES.iter().any(|unset| unset.as_bytes() == name),
            "{} must be unset while a staging check records",
            String::from_utf8_lossy(name)
        );
    }
    Ok(())
}

/// `<start unix seconds>-<process id>-<sequence>`: unique across the recordings of one host.
fn run_id(started_unix_seconds: u64) -> String {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    format!(
        "{started_unix_seconds}-{}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    )
}

#[cfg(test)]
#[path = "tests/operational_recording.rs"]
mod tests;
