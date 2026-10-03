//! **Role:** `validate-exports`: the committed export artifacts against their shapes, gates E2 and
//! the inventory gates.
//! **Position:** the `validate-exports` subcommand; runs `cargo xtask schema type-inventory` and
//! `cargo xtask map export-terrain` through `process_runner`.
//! **Signals & state:** none held.
//! **Invariants:** every gate prints its verdict; the exit code counts the failures.

use super::*;
use ::repository_layout::{
    map_scratch_dir, terrain_assets_dir, terrain_dir, terrain_registry_path,
};

/// `validate-exports`: checks the committed export artifacts against their declared shapes and
/// runs gates E2 and the inventory gates; returns the exit code.
pub fn validate_export_artifacts() -> Result<u8> {
    let root = find_repository_root()?;
    let schemas = SchemaSet::load()?;
    let v_prefab = schemas.validator("map-object-prefab")?;
    let v_instance = schemas.validator("map-object-instance")?;
    let v_roads = schemas.validator("map-object-roads")?;
    let v_region = schemas.validator("map-object-region")?;
    let v_registry = schemas.validator("terrain-registry")?;

    let mut failures = 0usize;
    let fail = |msg: String| {
        println!("  FAIL  {msg}");
    };
    let pass = |msg: String| println!("  PASS  {msg}");

    let registry: Value =
        serde_json::from_str(&std::fs::read_to_string(terrain_registry_path(&root))?)?;
    if !v_registry.is_valid(&registry) {
        failures += 1;
        fail("terrain-registry.json schema: invalid".into());
    } else {
        pass("terrain-registry.json schema valid".into());
    }

    let terrains = registry["terrains"].as_array().cloned().unwrap_or_default();
    for t in &terrains {
        let tid = t["terrainId"].as_str().unwrap_or("");
        let terrain_dir = terrain_dir(&root, tid);
        let manifest_path =
            terrain_assets_dir(&root).join(t["manifestPath"].as_str().unwrap_or(""));
        if !manifest_path.exists() {
            pass(format!(
                "{tid}: no manifest (status {}) — skipped",
                t["status"].as_str().unwrap_or("")
            ));
            continue;
        }
        let manifest: Value = serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
        if manifest["objects"]["prefabsPath"].is_null() {
            pass(format!(
                "{tid}: manifest has no objects export yet — skipped"
            ));
            continue;
        }
        let world_size_m = t["worldBoundsM"][2].as_f64().unwrap_or(0.0);
        let objects = &manifest["objects"];

        let prefabs_doc =
            gunzip_json(&terrain_dir.join(objects["prefabsPath"].as_str().unwrap_or("")))?;
        let prefabs = prefabs_doc["prefabs"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let bad = prefabs.iter().filter(|p| !v_prefab.is_valid(p)).count();
        if bad == 0 {
            pass(format!("{tid}: {} prefab rows schema-valid", prefabs.len()));
        } else {
            failures += 1;
            fail(format!("{tid}: {bad} invalid prefab rows"));
        }

        let chunks_dir = terrain_dir.join(objects["chunksPath"].as_str().unwrap_or(""));
        let sidecar: Value =
            serde_json::from_str(&std::fs::read_to_string(chunks_dir.join("manifest.json"))?)?;
        let chunk_size = sidecar["chunkSizeM"].as_f64().unwrap_or(CHUNK_SIZE_M);
        let mut row_total = 0u64;
        let mut chunk_errs = 0u64;
        let mut tree_rows: Vec<(f64, f64)> = Vec::new();
        for c in sidecar["cells"].as_array().cloned().unwrap_or_default() {
            let doc = gunzip_json(&terrain_dir.join(c["path"].as_str().unwrap_or("")))?;
            let rows = doc["instances"].as_array().cloned().unwrap_or_default();
            row_total += rows.len() as u64;
            if rows.len() as u64 != c["instanceCount"].as_u64().unwrap_or(0) {
                chunk_errs += 1;
            }
            for row in &rows {
                // invalid row OR (valid but mispartitioned) — short-circuit keeps the
                // else-if semantics of the .mjs (never double-counts one row).
                if !v_instance.is_valid(row)
                    || cell_of(row[1].as_f64().unwrap_or(0.0), chunk_size, world_size_m)
                        != c["cx"].as_i64().unwrap_or(-1)
                    || cell_of(row[2].as_f64().unwrap_or(0.0), chunk_size, world_size_m)
                        != c["cy"].as_i64().unwrap_or(-1)
                {
                    chunk_errs += 1;
                }
                if row[0]
                    .as_u64()
                    .and_then(|i| prefabs.get(i as usize))
                    .is_some_and(|p| p["kind"] == "tree")
                {
                    tree_rows.push((
                        row[1].as_f64().unwrap_or(0.0),
                        row[2].as_f64().unwrap_or(0.0),
                    ));
                }
            }
        }
        let cell_count = sidecar["cells"].as_array().map(Vec::len).unwrap_or(0);
        if chunk_errs == 0 {
            pass(format!(
                "{tid}: {cell_count} chunks, {row_total} rows valid + partition-correct"
            ));
        } else {
            failures += 1;
            fail(format!("{tid}: {chunk_errs} chunk row/partition error(s)"));
        }
        if Some(row_total) == objects["instanceCount"].as_u64() {
            pass(format!(
                "{tid}: manifest.objects.instanceCount = {row_total}"
            ));
        } else {
            failures += 1;
            fail(format!(
                "{tid}: manifest.objects.instanceCount {} != chunk rows {row_total}",
                objects["instanceCount"]
            ));
        }
        if Some(prefabs.len() as u64) == objects["prefabCount"].as_u64() {
            pass(format!(
                "{tid}: manifest.objects.prefabCount = {}",
                objects["prefabCount"]
            ));
        } else {
            failures += 1;
            fail(format!(
                "{tid}: manifest.objects.prefabCount {} != {}",
                objects["prefabCount"],
                prefabs.len()
            ));
        }

        let roads_doc =
            gunzip_json(&terrain_dir.join(objects["roadsPath"].as_str().unwrap_or("")))?;
        let seg_count = roads_doc["roadSegments"]
            .as_array()
            .map(Vec::len)
            .unwrap_or(0);
        if v_roads.is_valid(&roads_doc) && seg_count > 0 {
            pass(format!("{tid}: roads.json.gz valid ({seg_count} segments)"));
        } else {
            failures += 1;
            fail(format!("{tid}: roads.json.gz invalid or empty"));
        }

        let inventory: Value = serde_json::from_str(&std::fs::read_to_string(
            terrain_dir.join(objects["typeInventoryPath"].as_str().unwrap_or("")),
        )?)?;

        if objects["densityPath"].is_string() {
            let density_dir = terrain_dir.join(objects["densityPath"].as_str().unwrap_or(""));
            let grid_cells = (world_size_m / chunk_size).round() as usize;
            let on_disk = if density_dir.exists() {
                std::fs::read_dir(&density_dir)?
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_name().to_string_lossy().ends_with(".bin"))
                    .count()
            } else {
                0
            };
            let mut d_errs = 0u64;
            if on_disk != grid_cells * grid_cells {
                d_errs += 1;
                println!(
                    "        {tid}: density file count {on_disk} != {}",
                    grid_cells * grid_cells
                );
            }
            if objects["densityCellM"].as_u64() != Some(u64::from(density::DENSITY_CELL_M)) {
                d_errs += 1;
                println!(
                    "        {tid}: manifest densityCellM {} != {}",
                    objects["densityCellM"],
                    density::DENSITY_CELL_M
                );
            }
            let (tree_grid, tree_size) =
                density::accumulate_corners(tree_rows.iter().copied(), world_size_m);
            'outer: for cy in 0..grid_cells {
                for cx in 0..grid_cells {
                    if d_errs >= 8 {
                        break 'outer;
                    }
                    let p = density_dir.join(format!("{cx}_{cy}.bin"));
                    if !p.exists() {
                        d_errs += 1;
                        continue;
                    }
                    let buf = std::fs::read(&p)?;
                    let Ok(dec) = world_file_formats::density::tbdd::decode_tbdd(&buf) else {
                        d_errs += 1;
                        continue;
                    };
                    if buf.len() != density::TBDD_FILE_BYTES
                        || dec.version != density::TBDD_VERSION
                        || dec.cell_m != density::DENSITY_CELL_M
                        || dec.cols != density::DENSITY_COLS
                        || dec.rows != density::DENSITY_ROWS
                        || dec.channels.len() != density::DENSITY_CHANNELS.len()
                    {
                        d_errs += 1;
                        continue;
                    }
                    let expect = density::slice_chunk_corners(&tree_grid, tree_size, cx, cy);
                    for (k, want) in expect.iter().enumerate() {
                        if dec.channels[0][k] != *want {
                            d_errs += 1;
                            println!(
                                "        {tid}: density {cx}_{cy} tree channel differs from committed chunks at corner {k}"
                            );
                            break;
                        }
                    }
                }
            }
            if d_errs == 0 {
                pass(format!(
                    "{tid}: {on_disk} density bins valid (header + tree channel == committed chunks)"
                ));
            } else {
                failures += 1;
                fail(format!("{tid}: {d_errs} density error(s)"));
            }
        }

        if objects["regionsPath"].is_string() {
            let doc =
                gunzip_json(&terrain_dir.join(objects["regionsPath"].as_str().unwrap_or("")))?;
            let regions = doc["regions"].as_array().cloned().unwrap_or_default();
            let mut r_errs = 0u64;
            for r in &regions {
                if !v_region.is_valid(r) {
                    r_errs += 1;
                }
            }
            let region_tree_sum: u64 = regions
                .iter()
                .map(|r| r["treeCount"].as_u64().unwrap_or(0))
                .sum();
            let unassigned = inventory["unassignedTrees"].as_u64().unwrap_or(0);
            let inv_tree = inventory["byKind"]["tree"]["instances"]
                .as_u64()
                .unwrap_or(0);
            if region_tree_sum + unassigned != inv_tree {
                r_errs += 1;
                println!(
                    "        {tid}: F2 {region_tree_sum} + {unassigned} != tree instances {inv_tree}"
                );
            }
            if inventory["byRegionKind"]["forest"]["count"].as_u64() != Some(regions.len() as u64) {
                r_errs += 1;
            }
            if inventory["byRegionKind"]["forest"]["treeCount"].as_u64() != Some(region_tree_sum) {
                r_errs += 1;
            }
            if r_errs == 0 {
                pass(format!(
                    "{tid}: forest-regions.json.gz valid ({} regions, F2 exact)",
                    regions.len()
                ));
            } else {
                failures += 1;
                fail(format!("{tid}: {r_errs} forest-region error(s)"));
            }
        }
    }

    // Inventory gates (I1-I7 subset) — delegate to `cargo xtask schema type-inventory`.
    // Output captured (the Node script spawned with stdio:pipe) — surfaced only on failure.
    let inv_gate = process_runner::Run::new("cargo")
        .args(["run", "-q", "-p", "xtask", "--", "schema", "type-inventory"])
        .cwd(&root)
        .output()?;
    if inv_gate.code == 0 {
        pass("verify-type-inventory (I-gates) OK".into());
    } else {
        failures += 1;
        fail(format!(
            "verify-type-inventory: {} {}",
            inv_gate.stdout.trim(),
            inv_gate.stderr.trim()
        ));
    }

    // ---- E2 — identical script path for every terrain ----
    if terrains.len() >= 2 {
        pass(format!("E2a: registry has {} terrains", terrains.len()));
    } else {
        failures += 1;
        fail("E2a: registry needs >= 2 terrain rows".into());
    }

    let other = terrains.iter().find(|t| {
        !map_scratch_dir(&root, t["terrainId"].as_str().unwrap_or(""))
            .join("export/raw-entities.jsonl")
            .exists()
    });
    match other {
        Some(t) => {
            let tid = t["terrainId"].as_str().unwrap_or("");
            // The export stage runs as `cargo run -q -p xtask -- map export-terrain …`
            // (inherits CARGO_TARGET_DIR when set — same pin as Makefile / checkrun gates).
            let export = process_runner::Run::new("cargo")
                .args([
                    "run",
                    "-q",
                    "-p",
                    "xtask",
                    "--",
                    "map",
                    "export-terrain",
                    tid,
                    "--phase",
                    "P1_buildings",
                ])
                .cwd(&root)
                .output();
            // The export's own output is discarded; a signal reads as no exit code.
            let code = match export {
                Ok(output) => Some(output.code),
                Err(verification_core::NotRun::Signalled { .. }) => None,
                Err(not_run) => return Err(not_run.into()),
            };
            if code == Some(2) {
                pass(format!(
                    "E2b: xtask map export-terrain {tid} -> exit 2 (operator-instructions branch, same code path)"
                ));
            } else {
                failures += 1;
                fail(format!(
                    "E2b: xtask map export-terrain {tid} expected exit 2, got {code:?}"
                ));
            }
        }
        None => pass("E2b: every terrain already staged — branch untestable (OK)".into()),
    }

    {
        // E2c: terrain ids must flow from argv/registry — no literal id in the pipeline sources.
        // The pipeline is Rust; scanned set is the Rust modules + the xtask
        // orchestrator (topo.rs is excluded — its per-terrain CONFIG
        // TABLE is the sanctioned place for ids).
        let sources = [
            "tools/map_assets/world_export_pipeline/src/chunk_partitioner.rs",
            "tools/map_assets/world_export_pipeline/src/mathematical_verification.rs",
            "tools/map_assets/world_export_pipeline/src/export_preparation.rs",
            "tools/map_assets/world_export_pipeline/src/export_terrain_driver.rs",
            "tools/map_assets/world_export_pipeline/src/polygon_geometry.rs",
            "tools/map_assets/world_export_pipeline/src/vegetation_density.rs",
            "tools/map_assets/world_export_pipeline/src/forest_contours.rs",
        ];
        let mut offenders = Vec::new();
        for s in sources {
            let text = std::fs::read_to_string(root.join(s)).with_context(|| s.to_string())?;
            for t in &terrains {
                let tid = t["terrainId"].as_str().unwrap_or("");
                for (i, line) in text.lines().enumerate() {
                    if line.contains(tid) && !line.contains("E2c-allow") {
                        offenders.push(format!("{s}:{} literal '{tid}'", i + 1));
                    }
                }
            }
        }
        if offenders.is_empty() {
            pass("E2c: no literal terrain ids in pipeline scripts".into());
        } else {
            failures += 1;
            fail(format!("E2c: {}", offenders.join("; ")));
        }
    }

    let _ = forest::REGION_CELL_M;
    if failures > 0 {
        eprintln!("\nmap-export-validate: FAIL ({failures})");
        return Ok(1);
    }
    println!("\nmap-export-validate: OK");
    Ok(0)
}
