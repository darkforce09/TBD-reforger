//! The gameplay generation check the validate and publish commands call.
//!
//! **Role:** validates a gameplay generation against its committed schemas, the policy and the
//! publication receipt.
//! **Position:** under [`crate::equipment_gameplay`]; called by [`crate::equipment_vehicle_export`]
//! for a generation that carries gameplay data.
//! **Signals & state:** none; findings go into the caller's validation report.
//! **Invariants:** every file the check reads is hashed into the report; a schema that does not
//! compile is an error, not a pass.

use super::super::equipment_vehicle_export::{ValidationReport, files};
use super::{model::*, policy::Policy, validation_resource};
use crate::error::{Result, ResultExt, ensure};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

const SCHEMAS: [(&str, &str); 4] = [
    (
        "generation.json",
        include_str!(
            "../../../../../contracts/definitions/equipment-gameplay/generation.schema.json"
        ),
    ),
    (
        "resource_index.json",
        include_str!(
            "../../../../../contracts/definitions/equipment-gameplay/resource-index.schema.json"
        ),
    ),
    (
        "field_definitions.json",
        include_str!(
            "../../../../../contracts/definitions/equipment-gameplay/field-definitions.schema.json"
        ),
    ),
    (
        "resource",
        include_str!(
            "../../../../../contracts/definitions/equipment-gameplay/resource.schema.json"
        ),
    ),
];

pub(crate) fn validate(input: &Path) -> Result<ValidationReport> {
    let mut report = ValidationReport::default();
    if let Err(error) = inspect(input, &mut report) {
        report.errors.push(format!("{error:#}"));
    }
    report.valid = report.errors.is_empty();
    Ok(report)
}

fn inspect(input: &Path, report: &mut ValidationReport) -> Result<()> {
    let root = repository_layout::find_repository_root()?;
    let policy = Policy::load(&root)?;
    let validators: BTreeMap<_, _> = SCHEMAS
        .iter()
        .map(|(name, schema)| {
            Ok((
                *name,
                jsonschema::validator_for(&serde_json::from_str::<Value>(schema)?)?,
            ))
        })
        .collect::<Result<_>>()?;
    let mut documents = BTreeMap::new();
    for file in [
        "generation.json",
        "resource_index.json",
        "field_definitions.json",
        "native_types.json",
        "selection_report.json",
    ] {
        let (value, digest) = files::read(input, file)?;
        if let Some(validator) = validators.get(file) {
            validator
                .validate(&value)
                .map_err(|error| crate::error::refusal!("{file}: {error}"))?;
        }
        report.files.insert(file.into(), digest);
        documents.insert(file, value);
    }
    let generation = &documents["generation.json"];
    report.generation_id = text(&generation["generation_id"]).into();
    ensure!(
        generation["status"] == "completed",
        "extraction did not complete"
    );
    ensure!(
        generation["errors"].as_array().is_some_and(Vec::is_empty),
        "extraction errors present"
    );
    ensure!(
        generation["reader_verification"]["status"] == "passed",
        "source reader verification has not passed"
    );
    ensure!(
        generation["policy_sha256"] == policy.digest,
        "generation uses a different selection policy"
    );
    let selection = &documents["selection_report.json"];
    ensure!(
        selection["policy_sha256"] == policy.digest,
        "selection report policy differs"
    );
    ensure!(
        selection["unreviewed"]
            .as_array()
            .is_some_and(Vec::is_empty),
        "unreviewed source fields"
    );
    let decisions = selection["decisions"]
        .as_array()
        .context("selection decisions missing")?;
    let mut covered = BTreeSet::new();
    for decision in decisions {
        let key = (
            text(&decision["class_name"]),
            text(&decision["property"]),
            text(&decision["native_type"]),
        );
        ensure!(covered.insert(key), "duplicate field selection decision");
        let rule = policy.rule(key.0, key.1, key.2)?;
        ensure!(
            decision["disposition"] == rule.disposition
                && decision["section"] == rule.section
                && decision["reason"] == rule.reason,
            "selection decision differs from policy: {key:?}"
        );
    }
    ensure!(
        covered.len() == policy.fields.len(),
        "incomplete field policy coverage"
    );
    let entries: Vec<ResourceEntry> =
        serde_json::from_value(documents["resource_index.json"]["resources"].clone())?;
    ensure!(!entries.is_empty(), "empty resource index");
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    let mut domains: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut references = Vec::new();
    let definitions = documents["field_definitions.json"]["fields"]
        .as_object()
        .context("field definitions")?;
    for entry in &entries {
        ensure!(
            ids.insert(entry.resource_id.clone()) && names.insert(entry.resource_name.clone()),
            "duplicate resource identity"
        );
        for domain in &entry.domains {
            domains
                .entry(domain.clone())
                .or_default()
                .insert(entry.resource_id.clone());
        }
        ensure!(
            entry.resource_file.starts_with("resources/")
                || entry.resource_file.starts_with("shared_configurations/"),
            "invalid gameplay document location"
        );
        let (value, digest) = files::read(input, &entry.resource_file)?;
        validators["resource"]
            .validate(&value)
            .map_err(|e| crate::error::refusal!("{}: {e}", entry.resource_file))?;
        let resource: CompactResource = serde_json::from_value(value)?;
        ensure!(
            resource.resource_id == entry.resource_id
                && resource.resource_name == entry.resource_name,
            "index identity mismatch"
        );
        validation_resource::inspect(
            &resource,
            definitions,
            &documents["native_types.json"],
            &policy,
            report,
        )
        .with_context(|| entry.resource_name.clone())?;
        references.extend(
            resource
                .references
                .iter()
                .filter(|r| r["kind"] == "gameplay")
                .map(|r| text(&r["resource_name"]).to_owned()),
        );
        ensure!(
            report
                .files
                .insert(entry.resource_file.clone(), digest)
                .is_none(),
            "shared resource output path"
        );
        report.resource_count += 1;
    }
    for reference in references {
        ensure!(
            names.contains(&reference),
            "unresolved gameplay reference: {reference}"
        );
    }
    for (domain, field) in [("equipment", "equipment_ids"), ("vehicle", "vehicle_ids")] {
        let expected: BTreeSet<String> = serde_json::from_value(generation[field].clone())?;
        ensure!(
            expected == domains.get(domain).cloned().unwrap_or_default(),
            "{field} differs from resource index"
        );
    }
    for (field, count) in [
        ("resource_count", report.resource_count),
        ("node_count", report.source_node_count),
        ("fact_count", report.fact_count),
    ] {
        ensure!(
            selection[field].as_u64() == Some(count as u64),
            "selection report {field} differs"
        );
    }
    if input.join("publication_receipt.json").exists() {
        let (receipt, digest) = files::read(input, "publication_receipt.json")?;
        super::super::equipment_vehicle_export::gameplay_receipt::validate(
            input, &receipt, report,
        )?;
        report
            .files
            .insert("publication_receipt.json".into(), digest);
    }
    if input.join("manifest.json").exists() {
        let (manifest, _) = files::read(input, "manifest.json")?;
        let declared: BTreeMap<String, super::super::equipment_vehicle_export::FileDigest> =
            serde_json::from_value(manifest["files"].clone())?;
        ensure!(
            declared == report.files && manifest["generation_id"] == report.generation_id,
            "published manifest differs from validated files"
        );
    }
    for entry in walkdir::WalkDir::new(input).follow_links(false) {
        let entry = entry?;
        ensure!(!entry.file_type().is_symlink(), "symlink in generation");
        if entry.file_type().is_file() {
            let relative = entry
                .path()
                .strip_prefix(input)?
                .to_string_lossy()
                .replace('\\', "/");
            ensure!(
                report.files.contains_key(&relative) || relative == "manifest.json",
                "unexpected generation file: {relative}"
            );
        }
    }
    Ok(())
}
