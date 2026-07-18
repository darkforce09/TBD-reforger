//! T-165.3 — contract codegen: JSON Schema → Rust serde types via `typify`, replacing the Node
//! `quicktype` pipeline (`packages/tbd-schema/scripts/codegen.mjs`). Four schemas are generated;
//! `loadout.rs` is HAND-MAINTAINED since T-165.3 (the quicktype output was provably lossy — it
//! merged the versioned `oneOf` and emitted empty `Wear{}`/`Equipment{}`) and is guarded by serde
//! round-trip tests against the committed sample fixtures inside that file.
use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

use crate::root::find_repo_root as repo_root;

/// (schema file, output module) — the generated four. `loadout-export` is deliberately absent.
const TARGETS: [(&str, &str); 4] = [
    ("registry-items.schema.json", "registry_items"),
    ("registry-compat.schema.json", "registry_compat"),
    ("mission-editor-payload.schema.json", "mission_editor"),
    ("faction-library.schema.json", "faction_library"),
];

pub fn codegen() -> Result<u8> {
    let root = repo_root()?;
    let schema_dir = root.join("packages/tbd-schema/schema");
    let out_dir = root.join("apps/website/api/src/contract/generated");
    fs::create_dir_all(&out_dir)?;

    for (schema_file, module) in TARGETS {
        let raw = fs::read_to_string(schema_dir.join(schema_file))
            .with_context(|| schema_file.to_string())?;
        let root_schema: schemars::schema::RootSchema =
            serde_json::from_str(&raw).with_context(|| format!("parse {schema_file}"))?;

        let mut settings = typify::TypeSpaceSettings::default();
        settings.with_derive("Debug".to_string());
        let mut space = typify::TypeSpace::new(&settings);
        space
            .add_root_schema(root_schema)
            .map_err(|e| anyhow::anyhow!("{schema_file}: typify: {e}"))?;

        let tokens = space.to_stream();
        let file: syn::File =
            syn::parse2(tokens).map_err(|e| anyhow::anyhow!("{schema_file}: syn parse: {e}"))?;
        let body = prettyplease::unparse(&file);
        let banner = format!(
            "// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.\n\
             // Source: packages/tbd-schema/schema/{schema_file} — regenerate with: make schema-codegen\n\n"
        );
        let out_path = out_dir.join(format!("{module}.rs"));
        fs::write(&out_path, banner + &body)?;
        rustfmt(&out_path)?;
        println!("  {schema_file} -> src/contract/generated/{module}.rs");
    }
    println!("schema-codegen complete (loadout.rs is hand-maintained — see its header)");
    Ok(0)
}

fn rustfmt(path: &Path) -> Result<()> {
    let status = Command::new("rustfmt")
        .args(["--edition", "2024"])
        .arg(path)
        .status()
        .context("rustfmt spawn")?;
    anyhow::ensure!(status.success(), "rustfmt failed on {}", path.display());
    Ok(())
}
