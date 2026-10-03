//! Evidence receipts describe observations; acceptance is recomputed from their output.
//!
//! **Role:** the receipt shape, and the judge that accepts a receipt and its log for one check.
//! **Position:** `readiness_verification.rs` reads and judges every receipt with [`read`] and
//! [`validate`]; `operational_recording.rs` has the judge confirm a staging PASS before writing it.
//! **Signals & state:** none; reads the receipt and its log from the evidence folder.
//! **Invariants:** a receipt holds only when its identity, both fingerprints, its age (at most
//! 24 hours), its duration, its command, its exit code and its output digest match, the log
//! carries the success marker, no test was ignored or skipped, and the log reports at least the
//! check's minimum distinct cases; its log never resolves outside the evidence folder.

use crate::error::{Result, ensure};
use crate::register::{Check, EvidenceClass, relative_path};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Receipt {
    pub(crate) version: u32,
    pub(crate) check_id: String,
    pub(crate) class: EvidenceClass,
    pub(crate) source_sha256: String,
    pub(crate) configuration_sha256: String,
    pub(crate) command: Vec<String>,
    pub(crate) tool_versions: Vec<String>,
    pub(crate) started_unix_seconds: u64,
    pub(crate) duration_milliseconds: u128,
    pub(crate) exit_code: i32,
    pub(crate) output_file: String,
    pub(crate) output_sha256: String,
    /// Required for external staging runs: environment, fixture and configuration identities.
    pub(crate) environment: Vec<String>,
    pub(crate) observations: Option<crate::operational::Observations>,
    #[serde(default)]
    pub(crate) property_runs: Vec<crate::property_evidence::PropertyRun>,
}

pub(super) fn validate(
    check: &Check,
    receipt: &Receipt,
    output: &str,
    source: &str,
    configuration: &str,
    now: u64,
) -> Result<u64> {
    ensure!(
        receipt.version == 1 && receipt.check_id == check.id && receipt.class == check.class,
        "receipt identity mismatch"
    );
    ensure!(receipt.source_sha256 == source, "stale source fingerprint");
    ensure!(
        receipt.configuration_sha256 == configuration,
        "stale configuration fingerprint"
    );
    ensure!(
        receipt.started_unix_seconds > 0 && receipt.started_unix_seconds <= now,
        "invalid evidence timestamp"
    );
    ensure!(
        now - receipt.started_unix_seconds <= 86400,
        "evidence older than 24 hours"
    );
    ensure!(
        receipt.duration_milliseconds <= u128::from(check.timeout_seconds) * 1000,
        "check exceeded its deadline"
    );
    ensure!(
        !receipt.tool_versions.is_empty()
            && receipt.tool_versions.iter().all(|s| !s.trim().is_empty()),
        "missing tool versions"
    );
    if let Some(command) = &check.command {
        ensure!(&receipt.command == command, "wrong command");
    }
    ensure!(!receipt.command.is_empty(), "missing executed command");
    if check.class == EvidenceClass::Operational {
        ensure!(
            !receipt.environment.is_empty(),
            "missing staging environment identity"
        );
        crate::operational::validate(
            &check.id,
            receipt
                .observations
                .as_ref()
                .ok_or_else(|| crate::error::refusal!("missing operational measurements"))?,
        )?;
    }
    ensure!(receipt.exit_code == 0, "check exited {}", receipt.exit_code);
    ensure!(
        receipt.output_sha256 == content_digest::sha256_hex(output.as_bytes()),
        "output digest mismatch"
    );
    ensure!(
        output.contains(&check.success_marker),
        "missing success marker"
    );
    ensure!(
        !output.contains("skip: TEST_DATABASE_URL"),
        "unexecuted database acceptance check"
    );
    ensure!(
        !output
            .lines()
            .any(|line| line.starts_with("test ") && line.contains("... ignored")),
        "unexecuted ignored test"
    );
    let ignored_total: u64 = Regex::new(r"(?m)^test result: .*?; (\d+) ignored;")?
        .captures_iter(output)
        .map(|capture| capture[1].parse::<u64>().unwrap_or(u64::MAX))
        .try_fold(0_u64, |sum, count| sum.checked_add(count))
        .ok_or_else(|| crate::error::refusal!("ignored count overflow"))?;
    ensure!(
        ignored_total == 0,
        "verification requires every test to execute"
    );
    let cases = crate::case_count::successful_cases(&Regex::new(&check.case_pattern)?, output);
    ensure!(
        cases >= check.minimum_cases,
        "only {cases} successful cases, expected at least {}",
        check.minimum_cases
    );
    if check.class == EvidenceClass::Property {
        crate::property_evidence::validate(check, receipt, output)?;
    }
    Ok(cases)
}

pub(super) fn read(directory: &Path, check: &Check) -> Result<(Receipt, String)> {
    let receipt: Receipt = serde_json::from_slice(&std::fs::read(
        directory.join(format!("{}.json", check.id)),
    )?)?;
    ensure!(
        relative_path(&receipt.output_file),
        "unsafe evidence output path"
    );
    let output_path = directory.join(&receipt.output_file);
    ensure!(
        output_path
            .canonicalize()?
            .starts_with(directory.canonicalize()?),
        "evidence output escapes directory"
    );
    Ok((receipt, std::fs::read_to_string(output_path)?))
}
