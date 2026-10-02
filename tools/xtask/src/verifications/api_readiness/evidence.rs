//! Evidence receipts describe observations; acceptance is recomputed from their output.

use super::{
    fingerprint,
    register::{Check, EvidenceClass, relative_path},
};
use anyhow::{Result, ensure};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Receipt {
    pub version: u32,
    pub check_id: String,
    pub class: EvidenceClass,
    pub source_sha256: String,
    pub configuration_sha256: String,
    pub command: Vec<String>,
    pub tool_versions: Vec<String>,
    pub started_unix_seconds: u64,
    pub duration_milliseconds: u128,
    pub exit_code: i32,
    pub output_file: String,
    pub output_sha256: String,
    /// Required for external staging runs: environment, fixture and configuration identities.
    pub environment: Vec<String>,
    pub observations: Option<super::operational::Observations>,
    #[serde(default)]
    pub property_runs: Vec<super::property_evidence::PropertyRun>,
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
        super::operational::validate(
            &check.id,
            receipt
                .observations
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("missing operational measurements"))?,
        )?;
    }
    ensure!(receipt.exit_code == 0, "check exited {}", receipt.exit_code);
    ensure!(
        receipt.output_sha256 == fingerprint::digest(output.as_bytes()),
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
        .ok_or_else(|| anyhow::anyhow!("ignored count overflow"))?;
    ensure!(
        ignored_total == 0,
        "verification requires every test to execute"
    );
    let cases = super::case_count::successful_cases(&Regex::new(&check.case_pattern)?, output);
    ensure!(
        cases >= check.minimum_cases,
        "only {cases} successful cases, expected at least {}",
        check.minimum_cases
    );
    if check.class == EvidenceClass::Property {
        super::property_evidence::validate(check, receipt, output)?;
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
