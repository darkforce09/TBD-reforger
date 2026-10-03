//! `mod project-equipment-gameplay`: a complete source generation into a gameplay one.
//!
//! **Role:** projects every resource of a diagnostic generation through the policy and writes the
//! gameplay generation, its index and its receipt.
//! **Position:** under [`crate::equipment_gameplay`]; uses `resource_projection` per resource and
//! the export's file helpers.
//! **Signals & state:** none beyond the output folder it writes.
//! **Invariants:** the output is written only from a generation whose source verification passed.

use super::{
    model::*,
    policy::Policy,
    resource_projection::{self, ProjectionState},
};
use crate::error::{Result, ResultExt, ensure};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    path::Path,
};

pub(super) fn project(root: &Path, input: &Path, output: &Path) -> Result<()> {
    ensure!(
        !output.exists(),
        "projection output already exists; use a new generation directory"
    );
    let policy = Policy::load(root)?;
    let generation: Value = serde_json::from_slice(&fs::read(input.join("generation.json"))?)?;
    ensure!(
        generation["schema_version"] == 2
            && generation["status"] == "completed"
            && generation["scope"] == "complete",
        "projection requires a complete source export"
    );
    let entries: BTreeMap<_, _> = generation["resources"]
        .as_array()
        .context("source resource index")?
        .iter()
        .map(|r| (text(&r["resource_name"]).to_owned(), r))
        .collect();
    let roots: BTreeSet<_> = entries
        .iter()
        .filter(|(_, e)| {
            e["domains"]
                .as_array()
                .is_some_and(|d| d.iter().any(|s| s == "equipment" || s == "vehicle"))
        })
        .map(|(n, _)| n.clone())
        .collect();
    ensure!(!roots.is_empty(), "no established catalog resources");
    let mut queue: VecDeque<_> = roots.iter().cloned().collect();
    let mut queued = roots.clone();
    let mut state = ProjectionState::default();
    let mut index = Vec::new();
    let mut bytes_by_section = BTreeMap::<String, u64>::new();
    fs::create_dir_all(output)?;
    while let Some(name) = queue.pop_front() {
        let entry = entries.get(&name).with_context(|| {
            format!("required gameplay dependency is absent from diagnostic baseline: {name}")
        })?;
        let source: SourceDocument =
            serde_json::from_slice(&fs::read(input.join(text(&entry["source_file"])))?)?;
        let record: Value =
            serde_json::from_slice(&fs::read(input.join(text(&entry["record_file"])))?)?;
        let compact = resource_projection::project(&policy, source, record, &mut state)
            .with_context(|| name.clone())?;
        for reference in &compact.references {
            if reference["kind"] == "gameplay" {
                let target = text(&reference["resource_name"]).to_owned();
                if queued.insert(target.clone()) {
                    queue.push_back(target);
                }
            }
        }
        let old_file = text(&entry["record_file"]);
        let group = if roots.contains(&name) {
            "resources"
        } else {
            "shared_configurations"
        };
        let relative = format!(
            "{group}/{}",
            old_file
                .strip_prefix("records/")
                .context("unexpected source record path")?
        );
        let bytes = serde_json::to_vec(&compact)?;
        for node in &compact.nodes {
            for fact in node.properties.values() {
                let section = text(&state.definitions[&fact.definition_id]["section"]);
                *bytes_by_section.entry(section.into()).or_default() +=
                    serde_json::to_vec(fact)?.len() as u64;
            }
        }
        write(output, &relative, &bytes)?;
        index.push(ResourceEntry {
            resource_id: compact.resource_id,
            resource_name: name,
            resource_file: relative,
            domains: serde_json::from_value(entry["domains"].clone())?,
        });
        if index.len() % 100 == 0 {
            eprintln!(
                "Gameplay projection: {} resources, {} pending",
                index.len(),
                queue.len()
            );
        }
    }
    let id = output
        .file_name()
        .and_then(|s| s.to_str())
        .context("generation directory name")?;
    let mut environment = generation["environment"].clone();
    environment["projection_policy_sha256"] = json!(policy.digest);
    let metadata = json!({"document_type":"gameplay_generation","schema_version":1,"dataset_kind":"gameplay","generation_id":id,"scope":"complete","status":"completed","started_at":generation["started_at"],"finished_at":generation["finished_at"],"environment":environment,"errors":[],"reader_verification":generation["reader_verification"],"equipment_ids":generation["equipment_ids"],"vehicle_ids":generation["vehicle_ids"],"policy_version":1,"policy_sha256":policy.digest,"extraction_method":"diagnostic_projection","source_generation_id":generation["generation_id"]});
    write_json(output, "generation.json", &metadata)?;
    write_json(
        output,
        "resource_index.json",
        &json!({"schema_version":1,"resources":index}),
    )?;
    write_json(
        output,
        "field_definitions.json",
        &json!({"schema_version":1,"fields":state.definitions}),
    )?;
    write_json(output, "native_types.json", &generation["type_hierarchy"])?;
    let omitted: Vec<_> = entries.iter().filter(|(name,_)|!queued.contains(*name)).map(|(_,e)|json!({"resource_id":e["resource_id"],"resource_name":e["resource_name"],"disposition":"excluded_dependency","reason":"Not reachable through a retained gameplay relationship"})).collect();
    let decisions: Vec<_> = policy.fields.iter().map(|((class,property,native_type),rule)|{
        let key=format!("{class}\t{property}\t{native_type}");
        json!({"class_name":class,"property":property,"native_type":native_type,"disposition":rule.disposition,"section":rule.section,"reason":rule.reason,"observed_count":state.decisions.get(&key).copied().unwrap_or(0)})
    }).collect();
    let report = json!({"schema_version":1,"policy_version":1,"policy_sha256":policy.digest,"baseline_field_combinations":policy.fields.len(),"unreviewed":[],"resource_count":index.len(),"node_count":state.nodes,"fact_count":state.facts,"section_fact_counts":state.sections,"section_fact_bytes":bytes_by_section,"decisions":decisions,"excluded_dependencies":omitted});
    write_json(output, "selection_report.json", &report)?;
    let bytes: u64 = walkdir::WalkDir::new(output)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
        .sum();
    eprintln!(
        "Gameplay projection: {} resources, {} containers, {} facts; {} bytes",
        index.len(),
        state.nodes,
        state.facts,
        bytes
    );
    Ok(())
}

fn write_json(root: &Path, relative: &str, value: &Value) -> Result<()> {
    write(root, relative, &serde_json::to_vec(value)?)
}
fn write(root: &Path, relative: &str, bytes: &[u8]) -> Result<()> {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().context("document directory")?)?;
    fs::write(path, bytes)?;
    Ok(())
}
