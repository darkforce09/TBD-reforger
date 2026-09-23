//! Contract codegen: JSON Schema → Rust serde types via `typify`, with no Node in the pipeline.
//! Domain-specific schemas are generated. The loadout-export model is hand-maintained because
//! its versioned root `oneOf` is provably lossy (the branches merge and `Wear{}`/`Equipment{}` come
//! out empty), so it is hand-maintained in
//! `apps/website/api_v2/src/missions/contract/loadout_projection.rs` and guarded there by serde
//! round-trip tests against the committed sample fixtures.
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{Context, Result};

use developer_tools::repository_layout::contract_definitions_dir;

use crate::core::repository_root::find_repo_root as repo_root;

mod module_files;
mod module_plan;

/// Where the generated modules live, relative to the repository root.
const API_SOURCE_DIR: &str = "apps/website/api_v2/src";

/// Each schema file maps to the generated module directory of its owning domain.
const TARGETS: [(&str, &str); 18] = [
    (
        "registry-items.schema.json",
        "missions/contract/generated/registry_items",
    ),
    (
        "registry-compat.schema.json",
        "missions/contract/generated/registry_compat",
    ),
    (
        "mission-editor-payload.schema.json",
        "missions/contract/generated/mission_editor",
    ),
    (
        "faction-library.schema.json",
        "missions/contract/generated/faction_library",
    ),
    (
        "mission-review.schema.json",
        "missions/models/generated/mission_review",
    ),
    (
        "mission-deployment.schema.json",
        "missions/models/generated/mission_deployment",
    ),
    (
        "reservation-response.schema.json",
        "operations/models/generated/reservation_response",
    ),
    (
        "current-profile.schema.json",
        "identity_and_access/models/generated/current_profile",
    ),
    (
        "event-access-administration.schema.json",
        "operations/models/generated/event_access_administration",
    ),
    (
        "event-viewer-access.schema.json",
        "operations/models/generated/event_viewer_access",
    ),
    (
        "event-hub.schema.json",
        "operations/models/generated/event_hub",
    ),
    (
        "event-orbat.schema.json",
        "operations/models/generated/event_orbat",
    ),
    (
        "waitlist-promotion-response.schema.json",
        "operations/models/generated/waitlist_promotion_response",
    ),
    (
        "game-runtime-roster.schema.json",
        "operations/models/generated/game_runtime_roster",
    ),
    (
        "game-runtime-deployment.schema.json",
        "operations/models/generated/game_runtime_deployment",
    ),
    (
        "game-runtime-session.schema.json",
        "server_infrastructure/models/generated/game_runtime_session",
    ),
    (
        "machine-credential.schema.json",
        "server_infrastructure/models/generated/machine_credential",
    ),
    (
        "fleet-command.schema.json",
        "server_infrastructure/models/generated/fleet_command",
    ),
];

pub fn codegen() -> Result<u8> {
    let root = repo_root()?;
    write_generated_modules(&root)?;
    println!("schema-codegen complete (loadout_projection.rs is hand-maintained — see its header)");
    Ok(0)
}

/// Write every target's module directory, then remove what the schemas no longer produce:
/// Rust files the render did not emit, directories they leave empty, and a target's single-file
/// form, which would collide with its directory module.
fn write_generated_modules(root: &Path) -> Result<()> {
    let source_dir = root.join(API_SOURCE_DIR);
    for (schema_file, output) in TARGETS {
        let files = render_module(root, schema_file)?;
        let directory = source_dir.join(output);
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
        let single_file = directory.with_extension("rs");
        if single_file.exists() {
            fs::remove_file(&single_file)?;
        }
        println!("  {schema_file} -> src/{output}/ ({} files)", files.len());
    }
    Ok(())
}

/// Compare every target's module directory to a fresh render without relying on Git tracking
/// or mutating files: a missing, changed or unexpected file fails, and so does a leftover
/// single-file form of the target.
pub fn verify_fresh(root: &Path) -> Result<()> {
    let source_dir = root.join(API_SOURCE_DIR);
    for (schema_file, output) in TARGETS {
        compare_module(&render_module(root, schema_file)?, &source_dir.join(output))?;
    }
    Ok(())
}

/// Compare one target's module directory with its expected files.
fn compare_module(expected: &BTreeMap<String, String>, directory: &Path) -> Result<()> {
    let single_file = directory.with_extension("rs");
    anyhow::ensure!(
        !single_file.exists(),
        "stray generated output: {}",
        single_file.display()
    );
    for (relative, source) in expected {
        let path = directory.join(relative);
        let actual = fs::read_to_string(&path)
            .with_context(|| format!("missing generated output {}", path.display()))?;
        anyhow::ensure!(
            &actual == source,
            "stale generated output: {}",
            path.display()
        );
    }
    for relative in rust_files_under(directory)? {
        anyhow::ensure!(
            expected.contains_key(&relative),
            "stray generated output: {}",
            directory.join(&relative).display()
        );
    }
    Ok(())
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
        .map_err(|e| anyhow::anyhow!("{schema_file}: typify: {e}"))?;
    let file: syn::File = syn::parse2(space.to_stream())
        .map_err(|e| anyhow::anyhow!("{schema_file}: syn parse: {e}"))?;
    Ok((file, document))
}

/// Every file of one schema's generated module directory, keyed by its path inside it.
fn render_module(root: &Path, schema_file: &str) -> Result<BTreeMap<String, String>> {
    let (file, document) = typify_output(root, schema_file)?;
    let output = module_plan::partition(file, &module_plan::definition_names(&document))?;
    module_files::render_files(output, schema_file, &|source| rustfmt(root, source))
}

fn rustfmt(root: &Path, source: &str) -> Result<String> {
    let mut child = Command::new("rustfmt")
        .args(["--edition", "2024", "--emit", "stdout"])
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("rustfmt spawn")?;
    child
        .stdin
        .take()
        .context("rustfmt stdin")?
        .write_all(source.as_bytes())?;
    let output = child.wait_with_output().context("rustfmt completion")?;
    anyhow::ensure!(
        output.status.success(),
        "rustfmt failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).context("rustfmt output is UTF-8")
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
