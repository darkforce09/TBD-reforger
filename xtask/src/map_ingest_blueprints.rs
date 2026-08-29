//! Phase B (building blueprints) — `cargo xtask map ingest-blueprints`.
//!
//! Copies Workbench-exported building blueprints from the profile export dir into
//! `packages/map-assets/everon/prefabs/buildings/`, validating each file by a serde round-trip
//! through `map_engine_core::building_blueprint::BuildingBlueprint` (the exact contract the
//! `/debug/building-viewer` bench and the LOS raycaster consume — a file that ingests is a file
//! the viewer can render).
//!
//! Source resolution: `--src <dir>` override, else the first existing of the known profile
//! locations (the same candidates `apps/mod/tbd-export/tools/export-roads-to-png.mjs` probes), searched two levels
//! deep for `prefabs/buildings/*.json` (the map segment is derived Workbench-side and has taken
//! the shapes `everon/` and `$tbd_export:worlds/` in real runs).
//!
//! Exit: 0 = all matched files ingested · 1 = nothing matched or any file failed validation.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::root::find_repo_root;

const PROFILE_CANDIDATES: [&str; 2] = [
    ".local/share/Steam/steamapps/compatdata/1874910/pfx/drive_c/users/steamuser/Documents/My Games/ArmaReforgerWorkbench/profile/TBD_Export",
    "Games/ArmaReforger-Base/TBD_Export",
];

pub fn run(args: &[String]) -> Result<u8> {
    let root = find_repo_root()?;
    let mut src_override: Option<PathBuf> = None;
    let mut filter = String::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--src" if i + 1 < args.len() => {
                src_override = Some(PathBuf::from(&args[i + 1]));
                i += 2;
            }
            "--filter" if i + 1 < args.len() => {
                filter = args[i + 1].clone();
                i += 2;
            }
            other => {
                eprintln!(
                    "ingest-blueprints: unknown arg {other} (usage: [--src <dir>] [--filter <substr>])"
                );
                return Ok(1);
            }
        }
    }

    let sources = find_profile_subdirs(src_override.as_deref(), "prefabs/buildings");
    if sources.is_empty() {
        eprintln!("ingest-blueprints: no prefabs/buildings dir found under any profile candidate");
        for c in PROFILE_CANDIDATES {
            eprintln!("  probed: ~/{c}/**/prefabs/buildings");
        }
        return Ok(1);
    }

    let dest = root.join("packages/map-assets/everon/prefabs/buildings");
    fs::create_dir_all(&dest)?;

    let mut ok = 0usize;
    let mut failed = 0usize;
    let mut matched = 0usize;
    for src_dir in &sources {
        let Ok(entries) = fs::read_dir(src_dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !name.ends_with(".json") {
                continue;
            }
            let stem = name.trim_end_matches(".json");
            if stem.is_empty() {
                eprintln!("  SKIP empty-slug file {}", path.display());
                continue;
            }
            if !filter.is_empty() && !name.contains(&filter) {
                continue;
            }
            matched += 1;
            let text = fs::read_to_string(&path)?;
            match serde_json::from_str::<map_engine_core::building_blueprint::BuildingBlueprint>(
                &text,
            ) {
                Ok(bp) => {
                    let walls: usize = bp.levels.iter().map(|l| l.walls.len()).sum();
                    let windows: usize = bp.levels.iter().map(|l| l.windows.len()).sum();
                    let doors: usize = bp.levels.iter().map(|l| l.doors.len()).sum();
                    fs::copy(&path, dest.join(name))?;
                    println!(
                        "  OK  {name}: {} levels · {walls} walls · {doors} doors · {windows} windows",
                        bp.levels.len()
                    );
                    ok += 1;
                }
                Err(e) => {
                    eprintln!("  FAIL {name}: {e}");
                    failed += 1;
                }
            }
        }
    }

    println!(
        "ingest-blueprints: {ok} ingested, {failed} failed, {matched} matched (filter: {})",
        if filter.is_empty() { "<none>" } else { &filter }
    );
    if failed > 0 || matched == 0 {
        return Ok(1);
    }
    Ok(0)
}

/// Every existing `<candidate>/<map-segment>/<subdir>` dir (map segment searched one level
/// deep — Workbench has produced both `everon` and `$tbd_export:worlds` shapes). Shared with
/// `map blueprint-from-voxels`, which walks the same profiles for `prefabs/dumps`.
pub(crate) fn find_profile_subdirs(src_override: Option<&Path>, subdir: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(dir) = src_override {
        if dir.is_dir() {
            out.push(dir.to_path_buf());
        }
        return out;
    }
    let Some(home) = std::env::var_os("HOME") else {
        return out;
    };
    let home = PathBuf::from(home);
    for cand in PROFILE_CANDIDATES {
        let base = home.join(cand);
        if !base.is_dir() {
            continue;
        }
        let Ok(maps) = fs::read_dir(&base) else {
            continue;
        };
        for map_dir in maps.flatten() {
            let p = map_dir.path().join(subdir);
            if p.is_dir() {
                out.push(p);
            }
        }
    }
    out
}
