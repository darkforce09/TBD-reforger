use super::{FileDigest, ValidationReport, files};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use serde_json::{Value, json, value::RawValue};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

#[derive(Deserialize)]
struct Resource {
    nodes: Vec<Node>,
}
#[derive(Deserialize)]
struct Node {
    properties: BTreeMap<String, Box<RawValue>>,
}

pub(super) fn build(root: &Path, input: &Path, report: &ValidationReport) -> Result<Value> {
    let (definitions, _) = files::read(input, "field_definitions.json")?;
    let (index, _) = files::read(input, "resource_index.json")?;
    let entries = index["resources"].as_array().context("gameplay index")?;
    let selected: BTreeSet<_> = entries
        .iter()
        .map(|e| e["resource_id"].as_str().unwrap_or("").to_owned())
        .collect();
    let mut fact_bytes = BTreeMap::<String, u64>::new();
    for entry in entries {
        let relative = entry["resource_file"].as_str().context("resource file")?;
        let bytes = fs::read(files::child(input, relative)?)?;
        ensure!(
            files::digest(&bytes) == report.files[relative],
            "resource changed while measuring publication"
        );
        let resource: Resource = serde_json::from_slice(&bytes)?;
        for node in resource.nodes {
            for raw in node.properties.values() {
                let fact: Value = serde_json::from_str(raw.get())?;
                let id = fact["definition_id"].as_str().context("field definition")?;
                let section = definitions["fields"][id]["section"]
                    .as_str()
                    .context("field section")?;
                *fact_bytes.entry(section.into()).or_default() += raw.get().len() as u64;
            }
        }
    }
    let total_bytes: u64 = report.files.values().map(|f| f.bytes).sum();
    let fact_total: u64 = fact_bytes.values().sum();
    let (basis, excluded) = diagnostic_inventory(root, &selected)?;
    Ok(
        json!({"schema_version":1,"document_type":"gameplay_publication_receipt","generation_id":report.generation_id,
        "catalog_bytes":total_bytes,"retained_fact_json_bytes_by_section":fact_bytes,"structure_metadata_and_indexes_bytes":total_bytes-fact_total,
        "diagnostic_inventory_basis":basis,"excluded_diagnostic_dependencies":excluded}),
    )
}

pub(crate) fn validate(input: &Path, receipt: &Value, report: &ValidationReport) -> Result<()> {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../../../../contracts/definitions/equipment-gameplay/publication-receipt.schema.json"
    ))?;
    jsonschema::validator_for(&schema)?
        .validate(receipt)
        .map_err(|e| anyhow::anyhow!("publication receipt: {e}"))?;
    ensure!(
        receipt["generation_id"] == report.generation_id,
        "receipt generation differs"
    );
    let bytes: u64 = report
        .files
        .iter()
        .filter(|(name, _)| name.as_str() != "publication_receipt.json")
        .map(|(_, d)| d.bytes)
        .sum();
    ensure!(
        receipt["catalog_bytes"].as_u64() == Some(bytes),
        "receipt byte count differs"
    );
    let facts: u64 = receipt["retained_fact_json_bytes_by_section"]
        .as_object()
        .context("receipt sections")?
        .values()
        .map(|v| v.as_u64().unwrap_or(0))
        .sum();
    ensure!(
        facts.checked_add(
            receipt["structure_metadata_and_indexes_bytes"]
                .as_u64()
                .unwrap_or(0)
        ) == Some(bytes),
        "receipt section byte counts differ"
    );
    let (index, _) = files::read(input, "resource_index.json")?;
    let selected: BTreeSet<_> = index["resources"]
        .as_array()
        .context("resource index")?
        .iter()
        .map(|r| r["resource_id"].as_str().unwrap_or(""))
        .collect();
    let mut excluded = BTreeSet::new();
    for entry in receipt["excluded_diagnostic_dependencies"]
        .as_array()
        .context("excluded inventory")?
    {
        let id = entry["resource_id"].as_str().context("excluded identity")?;
        ensure!(
            !selected.contains(id) && excluded.insert(id),
            "excluded inventory overlaps or repeats an identity"
        );
        ensure!(
            !entry["domains"]
                .as_array()
                .context("excluded domains")?
                .iter()
                .any(|v| v == "equipment" || v == "vehicle"),
            "excluded equipment or vehicle"
        );
    }
    Ok(())
}

fn diagnostic_inventory(root: &Path, selected: &BTreeSet<String>) -> Result<(Value, Vec<Value>)> {
    let parent = root.parent().context("gameplay publication root")?;
    if !parent.join("current.json").exists() {
        return Ok((
            json!({"status":"unavailable","reason":"No published full diagnostic census is available"}),
            Vec::new(),
        ));
    }
    let (pointer, _) = files::read(parent, "current.json")?;
    let directory = files::child(
        parent,
        pointer["directory"]
            .as_str()
            .context("diagnostic publication location")?,
    )?;
    let (manifest, manifest_digest) = files::read(&directory, "manifest.json")?;
    ensure!(
        pointer["manifest_sha256"] == manifest_digest.sha256,
        "diagnostic manifest hash differs"
    );
    let (generation, digest) = files::read(&directory, "generation.json")?;
    let expected: FileDigest =
        serde_json::from_value(manifest["files"]["generation.json"].clone())?;
    ensure!(digest == expected, "diagnostic census hash differs");
    let mut excluded = Vec::new();
    for resource in generation["resources"]
        .as_array()
        .context("diagnostic census")?
    {
        if selected.contains(resource["resource_id"].as_str().unwrap_or("")) {
            continue;
        }
        ensure!(
            !resource["domains"]
                .as_array()
                .is_some_and(|d| d.iter().any(|v| v == "equipment" || v == "vehicle")),
            "an established equipment or vehicle identity is absent"
        );
        excluded.push(json!({"resource_id":resource["resource_id"],"resource_name":resource["resource_name"],"domains":resource["domains"],"disposition":"excluded_dependency","reason":"Not reachable through an approved gameplay relationship"}));
    }
    Ok((
        json!({"status":"present","generation_id":generation["generation_id"],"manifest_sha256":manifest_digest.sha256}),
        excluded,
    ))
}
