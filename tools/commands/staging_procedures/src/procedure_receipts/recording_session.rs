//! A recording: one receipt per procedure run, judged before it is written.
//!
//! **Role:** Opens a recording of one staging check and, when its procedure ends, decides PASS or
//! FAIL, writes the log, the fixture manifest and the receipt, and returns the exit code.
//!
//! **Position:** `procedure_runner/recording.rs` calls [`RecordingSession::begin`] before a
//! procedure's first step and [`RecordingSession::finish`] with its [`RecordedOutcome`], a
//! partial run included. The log grammar lives in `receipt_log.rs`, the measured thresholds in
//! `acceptance_thresholds.rs` and the per-check limits in `staging_check.rs`.
//!
//! **Signals & state:** a session owns the start snapshot (the check, the command, the start time
//! and the run id) until `finish` consumes it. `begin` removes the check's earlier receipt;
//! `finish` writes `<id>.log`, `<id>.fixture.json` and `<id>.json`, the receipt last, each through
//! a temporary file renamed over its destination.
//!
//! **Invariants:** a run passes only when every declared case is ok and named once, at least the
//! check's minimum of cases passed, the environment identities are recorded, the fleet and
//! Discord observations cite the manifest's digest, `acceptance_thresholds.rs` accepts the
//! measurements, and the run took no longer than the check's time limit. Anything else is written
//! as a FAIL: exit code 1, the real observations, the missing dependencies named, and no success
//! marker anywhere in the log.

use crate::error::{Result, ResultExt, ensure};
use serde::Serialize;
use std::{
    collections::BTreeSet,
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};

use super::acceptance_thresholds::{self, Observations};
use super::receipt_log::{
    CaseStatus, EnvironmentEntry, LogVerdict, ObservationRecord, RecordedCase, RunLog,
};
use super::staging_check::StagingCheck;

/// The procedure's fixture manifest, serialized once: the digest the observations cite is the
/// digest of the bytes written to `<id>.fixture.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FixtureManifest {
    bytes: Vec<u8>,
    sha256: String,
}

impl FixtureManifest {
    /// Serializes `manifest` as pretty JSON ending in a newline, and digests those bytes.
    pub(crate) fn new(manifest: &impl Serialize) -> Result<Self> {
        let mut bytes =
            serde_json::to_vec_pretty(manifest).context("serialize the fixture manifest")?;
        bytes.push(b'\n');
        Ok(Self {
            sha256: content_digest::sha256_hex(&bytes),
            bytes,
        })
    }

    /// The SHA-256 of the manifest bytes: the `fixture_sha256` fleet and Discord observations cite.
    pub(crate) fn sha256(&self) -> &str {
        &self.sha256
    }
}

/// Everything a procedure hands the recorder when it ends, a partial run included.
#[derive(Debug)]
pub(crate) struct RecordedOutcome {
    /// Every declared case in declaration order; one that could not run is `NotRun`.
    pub(crate) cases: Vec<RecordedCase>,
    /// Identities of the staging environment, never a secret.
    pub(crate) environment: Vec<EnvironmentEntry>,
    /// The measured values `acceptance_thresholds.rs` judges, recorded whatever the verdict.
    pub(crate) observations: Observations,
    /// The procedure definition and fixture identities the run executed against.
    pub(crate) fixture_manifest: FixtureManifest,
    /// The journaled observations, one `observation:` line each.
    pub(crate) journal: Vec<ObservationRecord>,
}

/// What a finished recording wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecordedReceipt {
    /// 0 for PASS, 1 for FAIL.
    pub(crate) exit_code: u8,
    /// The log's verdict line.
    pub(crate) summary: String,
}

/// The receipt `<id>.json`: what ran, when, how it ended, and what it measured.
#[derive(Debug, Serialize)]
struct Receipt {
    check_id: &'static str,
    command: Vec<String>,
    started_unix_seconds: u64,
    duration_milliseconds: u128,
    exit_code: u8,
    output_file: String,
    output_sha256: String,
    environment: Vec<String>,
    observations: Observations,
}

/// An open recording of one staging check.
#[derive(Debug)]
pub(crate) struct RecordingSession {
    receipts_directory: PathBuf,
    check: StagingCheck,
    command: Vec<String>,
    run_id: String,
    started_unix_seconds: u64,
    started_at: Instant,
}

impl RecordingSession {
    /// Opens a recording of `check`, run by `argv`, whose receipt goes to `receipts_directory`:
    /// refuses an empty command, snapshots the start time, and removes the check's earlier
    /// receipt.
    pub(crate) fn begin(
        receipts_directory: &Path,
        check: StagingCheck,
        argv: Vec<String>,
    ) -> Result<Self> {
        ensure!(
            !argv.is_empty() && argv.iter().all(|argument| !argument.is_empty()),
            "a recording names the command that runs it"
        );
        std::fs::create_dir_all(receipts_directory)?;
        match std::fs::remove_file(receipts_directory.join(check.file("json"))) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let started_unix_seconds =
            time_source::Clock::now_unix_ms(&time_source::SystemClock) / 1000;
        Ok(Self {
            receipts_directory: receipts_directory.to_owned(),
            check,
            command: argv,
            run_id: run_id(started_unix_seconds),
            started_unix_seconds,
            started_at: Instant::now(),
        })
    }

    /// The run id the log header and the harness journal share.
    pub(crate) fn run_id(&self) -> &str {
        &self.run_id
    }

    /// Ends the recording: decides the verdict and writes the log, the manifest and the receipt.
    /// Errors only when the receipt cannot be written.
    pub(crate) fn finish(self, outcome: RecordedOutcome) -> Result<RecordedReceipt> {
        let duration_milliseconds = self.started_at.elapsed().as_millis();
        let RecordedOutcome {
            cases,
            environment,
            observations,
            fixture_manifest,
            journal,
        } = outcome;
        let mut reasons = case_reasons(self.check, &cases);
        reasons.extend(acceptance_reasons(
            self.check,
            &observations,
            &fixture_manifest,
        ));
        if environment.is_empty() {
            reasons.push("no staging environment identity".to_owned());
        }
        let time_limit_seconds = self.check.time_limit_seconds();
        if duration_milliseconds > u128::from(time_limit_seconds) * 1000 {
            reasons.push(format!(
                "the run took {duration_milliseconds} ms, beyond the {time_limit_seconds} s limit"
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
        let verdict = if reasons.is_empty() {
            LogVerdict::Pass
        } else {
            LogVerdict::Fail(&reasons)
        };
        let text = log.render(&verdict);
        let exit_code = u8::from(!reasons.is_empty());
        let receipt = Receipt {
            check_id: self.check.id(),
            command: self.command.clone(),
            started_unix_seconds: self.started_unix_seconds,
            duration_milliseconds,
            exit_code,
            output_file: self.check.file("log"),
            output_sha256: content_digest::sha256_hex(text.as_bytes()),
            environment: environment
                .iter()
                .map(EnvironmentEntry::receipt_entry)
                .collect(),
            observations,
        };
        let directory = &self.receipts_directory;
        write_atomically(directory, &receipt.output_file, text.as_bytes())?;
        write_atomically(directory, &manifest_file, &fixture_manifest.bytes)?;
        write_atomically(
            directory,
            &self.check.file("json"),
            &serde_json::to_vec_pretty(&receipt)?,
        )?;
        Ok(RecordedReceipt {
            exit_code,
            summary: log.verdict_line(&verdict),
        })
    }
}

/// Reasons the declared cases refuse a PASS: none declared, a name declared twice, a failed case,
/// a case not run, fewer passing cases than the check's minimum.
fn case_reasons(check: StagingCheck, cases: &[RecordedCase]) -> Vec<String> {
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
    let passed = cases
        .iter()
        .filter(|case| case.status == CaseStatus::Ok)
        .count();
    let minimum = check.minimum_passing_cases();
    if passed < minimum {
        reasons.push(format!(
            "only {passed} passing cases, expected at least {minimum}"
        ));
    }
    reasons
}

/// Reasons the measurements refuse a PASS: `acceptance_thresholds.rs` rejects them, or the fleet
/// or Discord observations cite a fixture other than the manifest written beside the receipt.
fn acceptance_reasons(
    check: StagingCheck,
    observations: &Observations,
    manifest: &FixtureManifest,
) -> Vec<String> {
    let mut reasons = Vec::new();
    if let Err(error) = acceptance_thresholds::validate(check, observations) {
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

/// Writes `bytes` to `<directory>/<name>` through a fresh temporary file in the same folder,
/// synced and renamed over the destination, so a reader sees the old file or the whole new one.
fn write_atomically(directory: &Path, name: &str, bytes: &[u8]) -> Result<()> {
    static TEMPORARY_SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let temporary = directory.join(format!(
        ".{name}.{}-{}.tmp",
        std::process::id(),
        TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let written = (|| -> Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        std::fs::rename(&temporary, directory.join(name))?;
        Ok(())
    })();
    if written.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    written.with_context(|| format!("write the receipt file {name}"))
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
