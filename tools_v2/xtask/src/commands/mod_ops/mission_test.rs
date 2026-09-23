//! `cargo xtask mod test-mission`: choose what the Workbench client's mod loads.
//!
//! The mod runs the deployment its server credential reads, or — with no answer from the
//! platform — the last verified artifact cached in its profile
//! (`$profile:TBD_MissionArtifactCache/`). In the Workbench profile
//! (`$HOME/.../ArmaReforgerWorkbench/profile`):
//!
//! * no target — show the configured credential and the cached artifact;
//! * `<golden>` — stage that mission document from `contracts_v2` as the cached artifact, which
//!   Workbench then boots offline (a configured credential would make the platform's deployment
//!   win);
//! * `backend` — clear the cache, so the next boot reads the platform deployment.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::Value;
use verification_core::scan;

use super::website_api_client::{
    ARTIFACT_CACHE_DIRECTORY, StagedArtifact, clear_artifact_cache, sha256_hex,
    stage_artifact_cache,
};
use crate::core::repository_root::find_repo_root;

const CFG_REL: &str = ".local/share/Steam/steamapps/compatdata/1874910/pfx/drive_c/users/steamuser/Documents/My Games/ArmaReforgerWorkbench/profile";

/// Entry for `xtask mod test-mission [TARGET]`.
pub fn run(target: Option<&str>) -> Result<u8> {
    let root = find_repo_root()?;
    run_with_root(&root, target)
}

/// Testable entry that does not walk for the repo root. Honours `$HOME` for the profile tree.
pub fn run_with_root(root: &Path, target: Option<&str>) -> Result<u8> {
    let home = match std::env::var_os("HOME") {
        Some(h) => PathBuf::from(h),
        None => {
            eprintln!("HOME is unset");
            return Ok(1);
        }
    };
    let prof = home.join(CFG_REL);
    let cfg = prof.join("TBD_BackendConfig.json");

    if !cfg.is_file() {
        eprintln!("no config at {} — has the mod ever run?", cfg.display());
        return Ok(1);
    }

    match target {
        None | Some("") => {
            println!("current:");
            show(&cfg, &prof)?;
            Ok(0)
        }
        Some("backend") => {
            let cleared = clear_artifact_cache(&prof)?;
            println!(
                "{}; the next boot reads the platform deployment:",
                if cleared {
                    "cleared the cached artifact"
                } else {
                    "no cached artifact"
                }
            );
            show(&cfg, &prof)?;
            Ok(0)
        }
        Some(name) => stage_golden(root, &prof, &cfg, name),
    }
}

fn stage_golden(root: &Path, prof: &Path, cfg: &Path, name: &str) -> Result<u8> {
    let schema = developer_tools::repository_layout::contracts_dir(root);
    let want = format!("{name}.json");
    let Ok(Some(golden)) = find_golden(&schema, &want) else {
        eprintln!("no golden named '{name}' under contracts_v2");
        return Ok(1);
    };
    let document = fs::read(&golden).with_context(|| format!("read {}", golden.display()))?;
    let parsed: Value =
        serde_json::from_slice(&document).with_context(|| format!("parse {}", golden.display()))?;
    let mission_id = parsed
        .pointer("/meta/id")
        .and_then(Value::as_str)
        .with_context(|| format!("meta.id missing in {}", golden.display()))?;
    let terrain = parsed
        .pointer("/meta/terrain")
        .and_then(Value::as_str)
        .unwrap_or_default();
    stage_artifact_cache(
        prof,
        &document,
        &StagedArtifact {
            artifact_id: &format!("workbench-golden-{name}"),
            mission_id,
            terrain_key: terrain,
        },
    )?;
    // The registry override is optional.
    let _ = fs::copy(
        root.join("apps/mod/tbd-framework/Data/registry.json"),
        prof.join("TBD_Registry.json"),
    );
    println!("staged {name} as the cached artifact:");
    show(cfg, prof)?;
    Ok(0)
}

/// The first `<name>.json` under `contracts_v2`, in sorted walk order.
fn find_golden(
    schema: &Path,
    want_name: &str,
) -> Result<Option<PathBuf>, verification_core::NotRun> {
    let files = scan::walk_files(&[schema], |p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n == want_name)
    })?;
    Ok(files.into_iter().next())
}

fn show(cfg: &Path, prof: &Path) -> Result<()> {
    let text = fs::read_to_string(cfg).with_context(|| format!("read {}", cfg.display()))?;
    let config: Value =
        serde_json::from_str(&text).with_context(|| format!("parse {}", cfg.display()))?;
    let credential = config["machineCredential"].as_str().unwrap_or_default();
    if credential.starts_with("tbdm_") {
        println!(
            "  machineCredential set: boots the deployment it reads from {}",
            config["backendUrl"].as_str().unwrap_or("?")
        );
    } else {
        println!("  machineCredential unset: boots the cached artifact, if any");
    }

    let cache = prof.join(ARTIFACT_CACHE_DIRECTORY);
    let (Ok(identity), Ok(document)) = (
        fs::read_to_string(cache.join("identity.json")),
        fs::read(cache.join("document.json")),
    ) else {
        println!("  cached artifact: none");
        return Ok(());
    };
    let identity: Value = serde_json::from_str(&identity).unwrap_or(Value::Null);
    let recorded = identity["artifact_sha256"].as_str().unwrap_or_default();
    let intact = sha256_hex(&document) == recorded;
    println!(
        "  cached artifact: {} (sha256 {recorded}{})",
        identity["artifact_id"].as_str().unwrap_or("?"),
        if intact {
            ""
        } else {
            " — the document does not match it; the mod will refuse it"
        }
    );
    let parsed: Value = serde_json::from_slice(&document).unwrap_or(Value::Null);
    let slots = parsed["slots"].as_array().cloned().unwrap_or_default();
    let mut factions: BTreeMap<String, u32> = BTreeMap::new();
    for slot in &slots {
        let faction = slot["faction"].as_str().unwrap_or("?");
        *factions.entry(faction.to_string()).or_insert(0) += 1;
    }
    let seats: Vec<String> = factions.iter().map(|(k, v)| format!("{k} {v}")).collect();
    println!("  {} seats — {}", slots.len(), seats.join(", "));
    Ok(())
}

#[cfg(test)]
#[path = "tests/mission_test/tests.rs"]
mod tests;
