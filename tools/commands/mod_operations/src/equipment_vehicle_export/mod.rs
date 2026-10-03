//! Validation and publication of immutable, source-only Workbench generations.
//!
//! **Role:** `validate_command` checks an equipment and vehicle generation and prints its
//! [`ValidationReport`] as JSON; `publish_command` validates, seals and publishes one; `validate`
//! routes a gameplay generation to [`crate::equipment_gameplay`] and any other to the export
//! schema walk.
//! **Position:** called by [`crate::mod_dispatch`] for `mod validate-equipment-vehicle-export` and
//! `mod publish-equipment-vehicle-export`; reads the Workbench export generations the
//! EquipmentVehicleExport plugin writes.
//! **Signals & state:** none here; the publication lock belongs to `publication`.
//! **Invariants:** a generation is immutable once published; the validate command exits 1 when
//! the report is not valid and 0 when it is; every file the validation reads is digested into the
//! report.

pub(super) mod files;
pub(super) mod gameplay_receipt;
mod graph;
mod publication;
mod relationships;
mod unversioned_export_archive;
mod upload_bundle;
mod validation;

use std::collections::BTreeMap;
use std::path::Path;

use crate::Result;
use serde::Serialize;

#[derive(Debug, Default, Serialize)]
pub(crate) struct ValidationReport {
    pub valid: bool,
    pub(crate) generation_id: String,
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
