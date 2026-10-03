//! The line grammar of a staging recording's log.
//!
//! **Role:** Defines the values a staging recording writes as log lines (case names and
//! outcomes, environment identities, journaled observations), validates each where it is built,
//! and renders the complete log a staging receipt digests.
//!
//! **Position:** [`crate::operational_recording`] re-exports the value types to the
//! `cargo xtask staging` procedures and renders the log once for a candidate verdict, and again
//! when that verdict becomes a failure; `evidence.rs` later reads the written log through the
//! register's success marker and case pattern.
//!
//! **Signals & state:** none; value types and pure functions.
//!
//! **Invariants:** the log reads, one record per line:
//!
//! ```text
//! staging-run: <check> run=<id> started=<unix> command=<argv>
//! environment: <key>=<value>
//! fixture: sha256=<hex> manifest=<check>.fixture.json
//! observation: <step> <observer> <summary> sha256=<raw artifact digest>
//! case <check>_<name> ... ok | FAILED (<why>) | NOT RUN (missing: <dependency>)
//! missing: <dependency>
//! <check>: PASS <ok>/<declared>       or       <check>: FAIL <ok>/<declared> (<reasons>)
//! ```
//!
//! Case names and environment keys match `[a-z0-9_]+`, and no key names a secret. Every line but
//! a passing verdict is escaped as a whole: a backslash doubles, and a control character, a
//! Unicode line or paragraph separator, and the first character of each occurrence of the
//! success marker become `\u{hex}`. No value can therefore begin a line of its own, and only the
//! verdict line of a passing run carries the marker. The marker is `<check>: PASS` for a
//! `staging_` check, so it holds no backslash, never overlaps itself, and begins with a character
//! no escape sequence contains; escaping cannot reassemble it.

use crate::error::{Result, ensure};
use std::{collections::BTreeSet, fmt::Write as _};

/// Fragments that mark an environment key as secret-bearing; such a key is never recorded.
const SECRET_KEY_FRAGMENTS: [&str; 6] = [
    "token",
    "secret",
    "password",
    "credential",
    "authorization",
    "cookie",
];

/// The suffix of a `case <check>_<name>` line: one or more of `[a-z0-9_]`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CaseName(String);

impl CaseName {
    /// Accepts `name` only when it matches `[a-z0-9_]+`.
    pub fn new(name: &str) -> Result<Self> {
        ensure!(identifier(name), "case name {name:?} must match [a-z0-9_]+");
        Ok(Self(name.to_owned()))
    }

    /// The validated name.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// How one declared case ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaseStatus {
    /// Every effect the case awaits was observed within its deadline.
    Ok,
    /// The case ran and an effect was missing or wrong; the reason says what was observed.
    Failed(String),
    /// The case could not run; `missing` names the unavailable dependency.
    NotRun {
        /// The unavailable dependency, as the `missing:` line names it.
        missing: String,
    },
}

/// One declared case and how it ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedCase {
    /// The case name after the check prefix.
    pub name: CaseName,
    /// How the case ended.
    pub status: CaseStatus,
}

/// One `environment: <key>=<value>` identity of the staging environment; never a secret.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentEntry {
    key: String,
    value: String,
}

impl EnvironmentEntry {
    /// Accepts a `[a-z0-9_]+` key that names no secret, with a non-blank value.
    pub fn new(key: &str, value: &str) -> Result<Self> {
        ensure!(
            identifier(key),
            "environment key {key:?} must match [a-z0-9_]+"
        );
        ensure!(
            !SECRET_KEY_FRAGMENTS
                .iter()
                .any(|fragment| key.contains(fragment)),
            "environment key {key} names a secret, which a receipt never records"
        );
        ensure!(
            !value.trim().is_empty(),
            "environment key {key} has no value"
        );
        Ok(Self {
            key: key.to_owned(),
            value: value.to_owned(),
        })
    }

    /// The receipt form, `<key>=<value>`.
    pub(super) fn receipt_entry(&self) -> String {
        format!("{}={}", self.key, self.value)
    }
}

/// One journaled observation: its step, the observer that made it, a summary, and the SHA-256
/// of the raw artifact the journal archived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationRecord {
    step: String,
    observer: String,
    summary: String,
    artifact_sha256: String,
}

impl ObservationRecord {
    /// Accepts a `[A-Za-z0-9_.-]+` step, a non-blank observer and summary, and a lowercase
    /// hexadecimal SHA-256 digest.
    pub fn new(step: &str, observer: &str, summary: &str, artifact_sha256: &str) -> Result<Self> {
        ensure!(
            !step.is_empty()
                && step
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-')),
            "observation step {step:?} must match [A-Za-z0-9_.-]+"
        );
        ensure!(
            !observer.trim().is_empty() && !summary.trim().is_empty(),
            "observation of step {step} needs an observer and a summary"
        );
        ensure!(
            artifact_sha256.len() == 64
                && artifact_sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "observation of step {step} carries no lowercase SHA-256 artifact digest"
        );
        Ok(Self {
            step: step.to_owned(),
            observer: observer.to_owned(),
            summary: summary.to_owned(),
            artifact_sha256: artifact_sha256.to_owned(),
        })
    }
}

/// The verdict a log ends with.
pub(super) enum LogVerdict<'a> {
    Pass,
    Fail(&'a [String]),
}

/// A recording's log records, before the verdict decides its last line.
pub(super) struct RunLog<'a> {
    pub(crate) check: &'a str,
    pub(crate) run_id: &'a str,
    pub(crate) started_unix_seconds: u64,
    pub(crate) command: &'a [String],
    pub(crate) environment: &'a [EnvironmentEntry],
    pub(crate) fixture_sha256: &'a str,
    pub(crate) manifest_file: &'a str,
    pub(crate) journal: &'a [ObservationRecord],
    pub(crate) cases: &'a [RecordedCase],
}

impl RunLog<'_> {
    /// Renders every record, the verdict line last, each line ending in a newline.
    pub(super) fn render(&self, verdict: &LogVerdict<'_>) -> String {
        let marker = passing_marker(self.check);
        let mut log = String::new();
        for line in self.records() {
            log.push_str(&escape_line(&line, &marker));
            log.push('\n');
        }
        log.push_str(&self.verdict_line(verdict));
        log.push('\n');
        log
    }

    /// `<check>: PASS <ok>/<declared>`, or the escaped `<check>: FAIL <ok>/<declared> (<reasons>)`.
    pub(super) fn verdict_line(&self, verdict: &LogVerdict<'_>) -> String {
        let declared = self.cases.len();
        let ok = self
            .cases
            .iter()
            .filter(|case| case.status == CaseStatus::Ok)
            .count();
        match verdict {
            LogVerdict::Pass => format!("{}: PASS {ok}/{declared}", self.check),
            LogVerdict::Fail(reasons) => escape_line(
                &format!(
                    "{}: FAIL {ok}/{declared} ({})",
                    self.check,
                    reasons.join("; ")
                ),
                &passing_marker(self.check),
            ),
        }
    }

    /// Every line before the verdict, unescaped.
    fn records(&self) -> Vec<String> {
        let mut lines = vec![format!(
            "staging-run: {} run={} started={} command={}",
            self.check,
            self.run_id,
            self.started_unix_seconds,
            self.command.join(" ")
        )];
        lines.extend(
            self.environment
                .iter()
                .map(|entry| format!("environment: {}={}", entry.key, entry.value)),
        );
        lines.push(format!(
            "fixture: sha256={} manifest={}",
            self.fixture_sha256, self.manifest_file
        ));
        lines.extend(self.journal.iter().map(|record| {
            format!(
                "observation: {} {} {} sha256={}",
                record.step, record.observer, record.summary, record.artifact_sha256
            )
        }));
        lines.extend(self.cases.iter().map(|case| {
            let outcome = match &case.status {
                CaseStatus::Ok => "ok".to_owned(),
                CaseStatus::Failed(why) => format!("FAILED ({why})"),
                CaseStatus::NotRun { missing } => format!("NOT RUN (missing: {missing})"),
            };
            format!("case {}_{} ... {outcome}", self.check, case.name.as_str())
        }));
        lines.extend(
            missing_dependencies(self.cases)
                .into_iter()
                .map(|dependency| format!("missing: {dependency}")),
        );
        lines
    }
}

/// The success marker a passing verdict line of `check` carries: `<check>: PASS`.
pub(super) fn passing_marker(check: &str) -> String {
    format!("{check}: PASS")
}

/// The distinct dependencies the not-run cases name, sorted.
fn missing_dependencies(cases: &[RecordedCase]) -> BTreeSet<&str> {
    cases
        .iter()
        .filter_map(|case| match &case.status {
            CaseStatus::NotRun { missing } => Some(missing.as_str()),
            _ => None,
        })
        .collect()
}

/// Escapes one line: `\` doubles; a control character, U+2028, U+2029 and the first character of
/// every occurrence of `marker` become `\u{hex}`.
fn escape_line(line: &str, marker: &str) -> String {
    let mut escaped = String::with_capacity(line.len());
    for (index, character) in line.char_indices() {
        if line[index..].starts_with(marker)
            || character.is_control()
            || matches!(character, '\u{2028}' | '\u{2029}')
        {
            let _ = write!(escaped, "\\u{{{:x}}}", u32::from(character));
        } else if character == '\\' {
            escaped.push_str("\\\\");
        } else {
            escaped.push(character);
        }
    }
    escaped
}

fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}
