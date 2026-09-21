use super::*;
use crate::repository_layout::map_scratch_dir;

pub fn copy_world_export_profile(
    terrain: &str,
    full: bool,
    profile: Option<String>,
    src: Option<String>,
    meta: Option<String>,
) -> Result<u8> {
    let root = repo_root();
    let profile_dir = profile
        .map(PathBuf::from)
        .or_else(|| std::env::var("PROFILE").ok().map(PathBuf::from))
        .or_else(|| {
            std::env::var("ENFUSION_PROFILE_PATH")
                .ok()
                .map(PathBuf::from)
        })
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var("HOME").unwrap_or_default())
                .join("Documents/Games/ArmaReforgerWorkbench/profile")
        });
    let src_jsonl = src.map(PathBuf::from).unwrap_or_else(|| {
        profile_dir.join(if full {
            "TBD_WorldExport_full.jsonl"
        } else {
            "TBD_WorldExport_subregion.jsonl"
        })
    });
    let src_meta = meta.map(PathBuf::from).unwrap_or_else(|| {
        profile_dir.join(if full {
            "TBD_WorldExport_full_meta.json"
        } else {
            "TBD_WorldExport_meta.json"
        })
    });
    let dest_dir = map_scratch_dir(&root, terrain).join(if full { "export" } else { "spike" });
    let dest_jsonl = dest_dir.join("raw-entities.jsonl");
    let dest_meta = dest_dir.join("export-meta.json");
    let dest_stamp = dest_dir.join("staged-meta.json");

    if !src_jsonl.exists() {
        eprintln!(
            "copy-world-export-profile: source jsonl not found: {}",
            src_jsonl.display()
        );
        eprintln!(
            "  Run the TBD_TerrainWorldExportPlugin in Workbench first, or pass --src / --profile."
        );
        return Ok(1);
    }
    if full && !src_meta.exists() {
        eprintln!(
            "copy-world-export-profile: --full refused — completion-sentinel meta missing: {}",
            src_meta.display()
        );
        eprintln!(
            "  The plugin writes meta only after the JSONL closes; a missing meta = crashed/partial run."
        );
        return Ok(1);
    }
    // Count source FIRST — never overwrite a staged export with an empty jsonl.
    let mut line_count = 0u64;
    {
        let f = std::fs::File::open(&src_jsonl)?;
        for line in std::io::BufReader::new(f).lines() {
            if !line?.trim().is_empty() {
                line_count += 1;
            }
        }
    }
    if line_count == 0 {
        eprintln!(
            "copy-world-export-profile: refusing empty write — source jsonl has 0 rows: {}",
            src_jsonl.display()
        );
        return Ok(1);
    }
    std::fs::create_dir_all(&dest_dir)?;
    std::fs::copy(&src_jsonl, &dest_jsonl)?;
    if full {
        let meta_doc: Value = serde_json::from_str(&std::fs::read_to_string(&src_meta)?)?;
        if meta_doc["keptCount"].as_u64() != Some(line_count) {
            let _ = std::fs::remove_file(&dest_jsonl);
            eprintln!(
                "copy-world-export-profile: --full refused — meta.keptCount {} != staged line count {line_count} (truncated copy?). Staged jsonl removed.",
                meta_doc["keptCount"]
            );
            return Ok(1);
        }
        std::fs::copy(&src_meta, &dest_meta)?;
        // Provenance: when the plugin wrote no `workbenchVersion`, stamp the Steam
        // build id of the Workbench that produced the export (`appmanifest_1874910.acf` above the
        // profile's `steamapps/` — Arma Reforger Tools). `build-objects --patch-manifest` copies
        // it into `objects.workbenchVersion`; a Workbench outside Steam leaves the key absent.
        if meta_doc.get("workbenchVersion").is_none()
            && let Some(build) = steam_build_id(&src_meta)
        {
            let mut doc = meta_doc.clone();
            doc["workbenchVersion"] = json!(format!("Arma Reforger Tools (Steam build {build})"));
            std::fs::write(&dest_meta, serde_json::to_string_pretty(&doc)? + "\n")?;
        }
        let stamp = json!({
            "terrain": terrain,
            "stagedAt": iso_from_system_time(std::time::SystemTime::now()),
            "keptCount": line_count,
            "source": src_jsonl.to_string_lossy(),
        });
        std::fs::write(&dest_stamp, serde_json::to_string_pretty(&stamp)? + "\n")?;
        println!(
            "copy-world-export-profile: {terrain} FULL — staged {line_count} rows → {}; meta + stagedAt stamp written",
            dest_jsonl.display()
        );
    } else if src_meta.exists() {
        std::fs::copy(&src_meta, &dest_meta)?;
        println!(
            "copy-world-export-profile: {terrain} — copied {line_count} rows → {}; meta → {}",
            dest_jsonl.display(),
            dest_meta.display()
        );
    } else if dest_meta.exists() {
        // Refuse lossy synth overwrite of an existing real/prior meta.
        eprintln!(
            "copy-world-export-profile: refusing lossy write — source meta missing and dest meta already exists at {}; left jsonl updated, meta untouched",
            dest_meta.display()
        );
        return Ok(1);
    } else {
        let synth = json!({ "source": src_jsonl.to_string_lossy(), "copiedRows": line_count });
        std::fs::write(&dest_meta, serde_json::to_string_pretty(&synth)? + "\n")?;
        println!(
            "copy-world-export-profile: {terrain} — copied {line_count} rows → {}; meta synthesized (plugin wrote none) → {}",
            dest_jsonl.display(),
            dest_meta.display()
        );
    }
    Ok(0)
}

/// The `buildid` of Arma Reforger Tools from the Steam app manifest that owns `path`
/// (`…/steamapps/appmanifest_1874910.acf`), `None` outside a Steam library.
pub(super) fn steam_build_id(path: &Path) -> Option<String> {
    let steamapps = path
        .ancestors()
        .find(|a| a.file_name().is_some_and(|n| n == "steamapps"))?;
    let acf = std::fs::read_to_string(steamapps.join("appmanifest_1874910.acf")).ok()?;
    acf.lines().find_map(|l| {
        let l = l.trim();
        let rest = l.strip_prefix("\"buildid\"")?;
        Some(rest.trim().trim_matches('"').to_string())
    })
}
