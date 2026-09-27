//! Validation and publication of immutable, source-only Workbench generations.
pub(super) mod files;
pub(super) mod gameplay_receipt;
mod graph;
mod legacy_archive;
mod publication;
mod relationships;
mod upload_bundle;
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
    let report = validate(input)?;
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

pub(super) fn validate(input: &Path) -> Result<ValidationReport> {
    let (generation, _) = files::read(input, "generation.json")?;
    if generation["document_type"] == "gameplay_generation" {
        super::equipment_gameplay::validate(input)
    } else {
        validation::validate(input)
    }
}

pub(crate) fn publish_command(input: &Path) -> Result<u8> {
    publication::publish(input)?;
    Ok(0)
}

#[cfg(test)]
#[path = "tests/validation.rs"]
mod tests;
