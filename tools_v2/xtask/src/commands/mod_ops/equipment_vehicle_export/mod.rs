//! Validation and publication of immutable, source-only Workbench generations.
mod files;
mod graph;
mod legacy_archive;
mod publication;
mod relationships;
mod validation;

use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use serde::Serialize;

#[derive(Debug, Default, Serialize)]
pub(crate) struct ValidationReport {
    pub valid: bool,
    pub generation_id: String,
    pub resource_count: usize,
    pub source_node_count: usize,
    pub fact_count: usize,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub files: BTreeMap<String, FileDigest>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, serde::Deserialize)]
pub(crate) struct FileDigest {
    pub bytes: u64,
    pub sha256: String,
}

pub(crate) fn validate_command(input: &Path) -> Result<u8> {
    let report = validation::validate(input)?;
    eprintln!(
        "{}: {} resources, {} source nodes, {} facts, {} errors",
        if report.valid { "PASS" } else { "FAIL" },
        report.resource_count,
        report.source_node_count,
        report.fact_count,
        report.errors.len()
    );
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(u8::from(!report.valid))
}

pub(crate) fn publish_command(input: &Path) -> Result<u8> {
    publication::publish(input)?;
    Ok(0)
}

#[cfg(test)]
#[path = "tests/validation.rs"]
mod tests;
