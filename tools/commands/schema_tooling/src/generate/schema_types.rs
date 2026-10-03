//! Contract codegen: JSON Schema → Rust serde types via `typify`, with no Node in the pipeline.
//!
//! **Role:** writes, and checks the freshness of, the generated module tree of the
//! `contract_schema_types` crate: one module per API domain, one module folder per schema below
//! it, and the `mod.rs` files that declare them.
//! **Position:** `cargo xtask schema codegen` and `cargo xtask ci schema-codegen` call [`codegen`];
//! `cargo xtask ci verify-codegen-fresh` calls [`verify_fresh`]. Reads `contracts/definitions/`.
//! **Signals & state:** none; each run renders the whole tree in memory from the schemas.
//! **Invariants:** everything under [`OUTPUT_DIR`] is generator output: a Rust file the render
//! does not produce is removed by the codegen and refused by the freshness check. The
//! loadout-export model is not generated: its versioned root `oneOf` is provably lossy (the
//! branches merge and `Wear{}`/`Equipment{}` come out empty), so it is hand-maintained in
//! `apps/api/src/missions/contract/loadout_projection.rs` and guarded there by serde round-trip
//! tests against the committed sample fixtures.
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::error::{Result, ResultExt, refuse};
use process_runner::Run;

use repository_layout::contract_definitions_dir;

use repository_layout::find_repository_root as repo_root;

mod module_files;
mod module_plan;
mod module_tree;

/// The generated module tree, relative to the repository root: the `generated` folder of the
/// `contract_schema_types` crate.
const OUTPUT_DIR: &str = "crates/contracts/contract_schema_types/src/generated";

/// Each schema file maps to its module path inside [`OUTPUT_DIR`]: the API domain that serves or
/// reads it, then the schema's own module.
const TARGETS: [(&str, &str); 30] = [
    ("registry-items.schema.json", "missions/registry_items"),
    ("registry-compat.schema.json", "missions/registry_compat"),
    (
        "mission-editor-payload.schema.json",
        "missions/mission_editor",
    ),
    ("faction-library.schema.json", "missions/faction_library"),
    ("mission-review.schema.json", "missions/mission_review"),
    (
        "mission-deployment.schema.json",
        "missions/mission_deployment",
    ),
    (
        "reservation-response.schema.json",
        "operations/reservation_response",
    ),
    (
        "current-profile.schema.json",
        "identity_and_access/current_profile",
    ),
    (
        "event-access-administration.schema.json",
        "operations/event_access_administration",
    ),
    (
        "event-viewer-access.schema.json",
        "operations/event_viewer_access",
    ),
    ("event-hub.schema.json", "operations/event_hub"),
    ("event-orbat.schema.json", "operations/event_orbat"),
    (
        "waitlist-promotion-response.schema.json",
        "operations/waitlist_promotion_response",
    ),
    (
        "game-runtime-roster.schema.json",
        "operations/game_runtime_roster",
    ),
    (
        "game-runtime-deployment.schema.json",
        "operations/game_runtime_deployment",
    ),
    (
        "game-runtime-session.schema.json",
        "server_infrastructure/game_runtime_session",
    ),
    (
        "machine-credential.schema.json",
        "server_infrastructure/machine_credential",
    ),
    (
        "fleet-command.schema.json",
        "server_infrastructure/fleet_command",
    ),
    (
        "match-telemetry.schema.json",
        "match_telemetry/match_telemetry",
    ),
    (
        "personnel-roster.schema.json",
        "administration/personnel_roster",
    ),
    ("audit-log.schema.json", "administration/audit_log"),
    (
        "vehicle-database.schema.json",
        "community_content/vehicle_database",
    ),
    ("wiki-page.schema.json", "community_content/wiki_page"),
    (
        "content-upload.schema.json",
        "community_content/content_upload",
    ),
    (
        "equipment-data-viewer/resource-cards.schema.json",
        "community_content/equipment_data_viewer/resource_cards",
    ),
    (
        "equipment-data-viewer/dataset.schema.json",
        "community_content/equipment_data_viewer/dataset",
    ),
    (
        "equipment-data-viewer/resources.schema.json",
        "community_content/equipment_data_viewer/resources",
    ),
    (
        "equipment-data-viewer/source-inspection.schema.json",
        "community_content/equipment_data_viewer/source_inspection",
    ),
    (
        "equipment-data-viewer/relationships.schema.json",
        "community_content/equipment_data_viewer/relationships",
    ),
    (
        "equipment-data-viewer/field-inventory.schema.json",
        "community_content/equipment_data_viewer/field_inventory",
    ),
];

/// Regenerate the whole module tree from the schemas.
pub fn codegen() -> Result<u8> {
    let root = repo_root()?;
    write_generated_modules(&root)?;
    println!("schema-codegen complete (loadout_projection.rs is hand-maintained — see its header)");
    Ok(0)
}

/// Write every file of the module tree, then remove what the schemas no longer produce: Rust
/// files the render did not emit and the folders they leave empty.
fn write_generated_modules(root: &Path) -> Result<()> {
    let files = render_tree(root)?;
    let directory = root.join(OUTPUT_DIR);
    for (relative, source) in &files {
        let path = directory.join(relative);
        fs::create_dir_all(path.parent().context("generated file parent")?)?;
        fs::write(&path, source)?;
    }
    for relative in rust_files_under(&directory)? {
        if !files.contains_key(&relative) {
            fs::remove_file(directory.join(&relative))?;
        }
    }
    remove_empty_directories(&directory)?;
    for (schema_file, output) in TARGETS {
        let count = files
            .keys()
            .filter(|relative| relative.starts_with(&format!("{output}/")))
            .count();
        println!("  {schema_file} -> {OUTPUT_DIR}/{output}/ ({count} files)");
    }
    Ok(())
}

/// Compare the module tree to a fresh render without relying on Git tracking or mutating files:
/// a missing, changed or unexpected Rust file fails.
pub fn verify_fresh(root: &Path) -> Result<()> {
    compare_tree(&render_tree(root)?, &root.join(OUTPUT_DIR))
}

/// Compare the generated folder with its expected files, keyed by their path inside it.
fn compare_tree(expected: &BTreeMap<String, String>, directory: &Path) -> Result<()> {
    for (relative, source) in expected {
        let path = directory.join(relative);
        let actual = fs::read_to_string(&path)
            .with_context(|| format!("missing generated output {}", path.display()))?;
        if &actual != source {
            refuse!("stale generated output: {}", path.display());
        }
    }
    for relative in rust_files_under(directory)? {
        if !expected.contains_key(&relative) {
            refuse!(
                "stray generated output: {}",
                directory.join(&relative).display()
            );
        }
    }
    Ok(())
}

/// Every file of the module tree, keyed by its path inside [`OUTPUT_DIR`]: each schema's module
/// folder and the `mod.rs` files above them.
fn render_tree(root: &Path) -> Result<BTreeMap<String, String>> {
    let format = |source: &str| rustfmt(root, source);
    let mut files = module_tree::render_tree_files(&TARGETS, &format)?;
    for (schema_file, output) in TARGETS {
        for (relative, source) in render_module(root, schema_file)? {
            let path = format!("{output}/{relative}");
            if files.insert(path.clone(), source).is_some() {
                refuse!("two generated files would share the path {path}");
            }
        }
    }
    Ok(files)
}

/// Typify's output for one schema, with the schema document it came from.
fn typify_output(root: &Path, schema_file: &str) -> Result<(syn::File, serde_json::Value)> {
    let schema_dir = contract_definitions_dir(root);
    let raw = fs::read_to_string(schema_dir.join(schema_file))
        .with_context(|| schema_file.to_string())?;
    let document: serde_json::Value =
        serde_json::from_str(&raw).with_context(|| format!("parse {schema_file}"))?;
    let root_schema: schemars::schema::RootSchema =
        serde_json::from_str(&raw).with_context(|| format!("parse {schema_file}"))?;

    let mut settings = typify::TypeSpaceSettings::default();
    settings.with_derive("Debug".to_string());
    if schema_file == "current-profile.schema.json" {
        // Web timestamps retain their exact fractional precision through a consumer round trip.
        settings.with_conversion(
            schemars::schema::SchemaObject {
                instance_type: Some(schemars::schema::InstanceType::String.into()),
                format: Some("date-time".into()),
                ..Default::default()
            },
            "::std::string::String",
            [typify::TypeSpaceImpl::Display].into_iter(),
        );
    }
    let mut space = typify::TypeSpace::new(&settings);
    space
        .add_root_schema(root_schema)
        .map_err(|e| crate::error::Error::msg(format!("{schema_file}: typify: {e}")))?;
    let file: syn::File = syn::parse2(space.to_stream())
        .map_err(|e| crate::error::Error::msg(format!("{schema_file}: syn parse: {e}")))?;
    Ok((file, document))
}

/// Every file of one schema's generated module directory, keyed by its path inside it.
fn render_module(root: &Path, schema_file: &str) -> Result<BTreeMap<String, String>> {
    let (file, document) = typify_output(root, schema_file)?;
    let output = module_plan::partition(file, &module_plan::definition_names(&document))?;
    module_files::render_files(output, schema_file, &|source| rustfmt(root, source))
}

/// `source` formatted by `rustfmt --edition 2024`, run in the checkout root so the repository's
/// `rustfmt.toml` applies; the source goes in on stdin and the formatted text comes back on stdout.
fn rustfmt(root: &Path, source: &str) -> Result<String> {
    let output = Run::new("rustfmt")
        .args(["--edition", "2024", "--emit", "stdout"])
        .cwd(root)
        .stdin(source)
        .output()
        .map_err(process_runner::Error::from)
        .context("rustfmt completion")?;
    if output.code != 0 {
        refuse!("rustfmt failed: {}", output.stderr);
    }
    Ok(output.stdout)
}

/// The Rust files under `directory`, as `/`-separated paths relative to it; none when the
/// directory does not exist.
fn rust_files_under(directory: &Path) -> Result<Vec<String>> {
    if !directory.is_dir() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(directory) {
        let entry = entry?;
        if entry.file_type().is_file()
            && entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "rs")
        {
            let relative = entry.path().strip_prefix(directory)?;
            files.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
    files.sort();
    Ok(files)
}

fn remove_empty_directories(directory: &Path) -> Result<()> {
    if !directory.is_dir() {
        return Ok(());
    }
    for entry in walkdir::WalkDir::new(directory).contents_first(true) {
        let entry = entry?;
        if entry.file_type().is_dir() && fs::read_dir(entry.path())?.next().is_none() {
            fs::remove_dir(entry.path())?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "tests/schema_types.rs"]
mod tests;
