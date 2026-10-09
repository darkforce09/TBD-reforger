//! Acceptance requires measured generated cases, independent of libtest function counts.
//!
//! **Role:** parses the `property-run:` records a property check prints and judges them against
//! the check's required properties, the receipt's environment and the run's seed.
//! **Position:** `readiness_verification.rs` stores the parsed records in each local receipt;
//! `evidence.rs` calls [`validate`] on every `property` receipt it judges.
//! **Signals & state:** none; reads the process environment through
//! [`crate::PropertyTestConfiguration::from_environment`].
//! **Invariants:** every required property executed at least its minimum generated cases under
//! the run's seed; a malformed or duplicated record fails the whole receipt.

use crate::PropertyTestConfiguration;
use crate::error::{Result, ensure};
use crate::{evidence::Receipt, register::Check};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct PropertyRun {
    pub(crate) version: u32,
    pub(crate) id: String,
    pub(crate) requested_cases: u32,
    pub(crate) executed_cases: u32,
    pub(crate) seed: u64,
    pub(crate) algorithm: String,
    pub(crate) input_sha256: String,
}

pub(super) fn parse(output: &str) -> Result<Vec<PropertyRun>> {
    let mut records = BTreeMap::new();
    for line in output.lines() {
        let Some(json) = line.strip_prefix("property-run: ") else {
            continue;
        };
        let record: PropertyRun = serde_json::from_str(json)?;
        ensure!(
            records.insert(record.id.clone(), record).is_none(),
            "duplicate property execution ID"
        );
    }
    Ok(records.into_values().collect())
}

pub(super) fn validate(check: &Check, receipt: &Receipt, output: &str) -> Result<()> {
    let configuration = PropertyTestConfiguration::from_environment()?;
    validate_with_configuration(check, receipt, output, configuration)
}

fn validate_with_configuration(
    check: &Check,
    receipt: &Receipt,
    output: &str,
    configuration: PropertyTestConfiguration,
) -> Result<()> {
    ensure!(!check.properties.is_empty(), "property acceptance is empty");
    let environment: Vec<_> = receipt
        .environment
        .iter()
        .filter(|entry| entry.starts_with("PROPTEST_"))
        .cloned()
        .collect();
    ensure!(
        environment == configuration.receipt_environment(),
        "missing, duplicate or conflicting property environment identity"
    );
    let markers: Vec<_> = output
        .lines()
        .filter(|line| line.starts_with("property-test-configuration:"))
        .collect();
    ensure!(
        markers == vec![configuration.marker()],
        "missing, duplicate or conflicting property configuration marker"
    );
    let records = parse(output)?;
    ensure!(
        records == receipt.property_runs,
        "property receipt differs from measured output"
    );
    for required in &check.properties {
        let record = records
            .iter()
            .find(|record| record.id == required.id)
            .ok_or_else(|| {
                crate::error::refusal!("required property did not execute: {}", required.id)
            })?;
        ensure!(
            record.version == 1
                && record.seed == configuration.rng_seed
                && record.algorithm == "ChaCha",
            "property runner identity mismatch: {}",
            required.id
        );
        ensure!(
            record.requested_cases >= required.minimum_cases
                && record.executed_cases == record.requested_cases
                && record.executed_cases > 0,
            "insufficient measured generated cases for {}: {}",
            required.id,
            record.executed_cases
        );
        ensure!(
            record.input_sha256.len() == 64
                && record
                    .input_sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "missing property input digest: {}",
            required.id
        );
    }
    Ok(())
}
