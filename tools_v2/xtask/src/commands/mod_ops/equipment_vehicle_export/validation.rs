use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::Result;
use serde_json::Value;

use super::{ValidationReport, files, graph, relationships};

const SCHEMA: &str =
    include_str!("../../../../../../contracts_v2/definitions/equipment-vehicle-export.schema.json");

pub(super) fn validate(input: &Path) -> Result<ValidationReport> {
    let mut report = ValidationReport::default();
    let schema: Value = serde_json::from_str(SCHEMA)?;
    let generation_validator = schema_validator(&schema, "generation")?;
    let record_validator = schema_validator(&schema, "record")?;
    let snapshot_validator = schema_validator(&schema, "snapshot")?;
    let generation = match document(
        input,
        "generation.json",
        "export_generation",
        &generation_validator,
        &mut report,
    ) {
        Some(value) => value,
        None => return Ok(report),
    };
    report.generation_id = text(&generation, "generation_id").to_owned();
    for (key, expected) in [("status", "completed"), ("scope", "complete")] {
        if generation[key] != expected {
            report
                .errors
                .push(format!("generation {key} must be {expected}"));
        }
    }
    if generation["reader_verification"]["status"] != "passed" {
        report
            .errors
            .push("Workbench source reader verification has not passed".into());
    }
    for error in array(&generation["errors"]) {
        report.errors.push(format!("extraction: {error}"));
    }
    relationships::validate_hierarchy(&generation, &mut report);
    let entries = array(&generation["resources"]);
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    let mut paths = BTreeSet::from(["generation.json".to_owned()]);
    let mut references = Vec::new();
    let mut domains: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for entry in entries {
        let id = text(entry, "resource_id");
        let name = text(entry, "resource_name");
        if !ids.insert(id.to_owned()) {
            report
                .errors
                .push(format!("duplicate resource identity: {id}"));
        }
        if !names.insert(name.to_owned()) {
            report
                .errors
                .push(format!("duplicate resource name: {name}"));
        }
        for domain in array(&entry["domains"]).iter().filter_map(Value::as_str) {
            domains.entry(domain).or_default().insert(id.to_owned());
        }
        for field in ["record_file", "source_file"] {
            if !paths.insert(text(entry, field).to_owned()) {
                report
                    .errors
                    .push(format!("shared output path: {}", entry[field]));
            }
        }
        let record = document(
            input,
            text(entry, "record_file"),
            "resource_record",
            &record_validator,
            &mut report,
        );
        let snapshot = document(
            input,
            text(entry, "source_file"),
            "source_snapshot",
            &snapshot_validator,
            &mut report,
        );
        if let (Some(record), Some(snapshot)) = (record, snapshot) {
            for value in [&record, &snapshot] {
                if value["resource_id"] != id || value["resource_name"] != name {
                    report
                        .errors
                        .push(format!("index identity disagreement: {id}"));
                }
            }
            graph::validate(&record, &snapshot, &mut report);
            relationships::validate_types(&record, &snapshot, &generation, &mut report);
            for reference in array(&record["references"]) {
                if reference["kind"] == "gameplay" {
                    references.push((name.to_owned(), text(reference, "resource_name").to_owned()));
                }
            }
            report.resource_count += 1;
        }
    }
    for (owner, target) in references {
        if !names.contains(&target) {
            report.errors.push(format!(
                "unresolved gameplay reference: {owner} -> {target}"
            ));
        }
    }
    for (domain, field) in [("equipment", "equipment_ids"), ("vehicle", "vehicle_ids")] {
        let expected = domains.get(domain).cloned().unwrap_or_default();
        let actual: BTreeSet<_> = array(&generation[field])
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect();
        if actual != expected {
            report
                .errors
                .push(format!("{field} disagrees with resource domains"));
        }
    }
    for discovered in array(&generation["discovered_resources"])
        .iter()
        .filter_map(Value::as_str)
    {
        if !names.contains(discovered) {
            report
                .errors
                .push(format!("discovered resource omitted: {discovered}"));
        }
    }
    if generation["discovered_resources"]
        .as_array()
        .is_none_or(Vec::is_empty)
    {
        report.errors.push("discovery census is empty".into());
    }
    for entry in walkdir::WalkDir::new(input).follow_links(false) {
        let entry = entry?;
        if entry.file_type().is_symlink() {
            report
                .errors
                .push(format!("symlink in generation: {}", entry.path().display()));
        }
        if entry.file_type().is_file() {
            let relative = entry
                .path()
                .strip_prefix(input)?
                .to_string_lossy()
                .replace('\\', "/");
            if !paths.contains(&relative) && relative != "manifest.json" {
                report
                    .errors
                    .push(format!("unlisted generation file: {relative}"));
            }
        }
    }
    if input.join("manifest.json").exists() {
        match files::read(input, "manifest.json") {
            Ok((manifest, _)) => {
                let expected = serde_json::json!({"schema_version": 2, "generation_id": report.generation_id, "resource_count": report.resource_count, "files": report.files});
                if manifest != expected {
                    report.errors.push(
                        "finalized manifest counts or file hashes do not match the generation"
                            .into(),
                    );
                }
            }
            Err(error) => report
                .errors
                .push(format!("invalid finalized manifest: {error:#}")),
        }
    }
    report.valid = report.errors.is_empty();
    Ok(report)
}

fn schema_validator(schema: &Value, definition: &str) -> Result<jsonschema::Validator> {
    let scoped = serde_json::json!({"$schema":schema["$schema"],"$defs":schema["$defs"],"$ref":format!("#/$defs/{definition}")});
    Ok(jsonschema::validator_for(&scoped)?)
}

fn document(
    input: &Path,
    path: &str,
    kind: &str,
    validator: &jsonschema::Validator,
    report: &mut ValidationReport,
) -> Option<Value> {
    let (value, digest) = match files::read(input, path) {
        Ok(pair) => pair,
        Err(error) => {
            report.errors.push(format!("{path}: {error:#}"));
            return None;
        }
    };
    report.files.insert(path.to_owned(), digest);
    let mut valid = true;
    for error in validator.iter_errors(&value) {
        let detail: String = error.to_string().chars().take(1200).collect();
        report
            .errors
            .push(format!("{path} {}: {detail}", error.instance_path()));
        valid = false;
    }
    if value["document_type"] != kind {
        report.errors.push(format!("{path}: expected {kind}"));
        valid = false;
    }
    valid.then_some(value)
}

pub(super) fn array(value: &Value) -> &[Value] {
    value.as_array().map(Vec::as_slice).unwrap_or(&[])
}
pub(super) fn text<'a>(value: &'a Value, key: &str) -> &'a str {
    value[key].as_str().unwrap_or("")
}
