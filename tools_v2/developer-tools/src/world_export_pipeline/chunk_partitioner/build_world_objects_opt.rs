use super::*;

/// `quiet` suppresses the summary print (the E6 in-process scratch builds — the Node gate
/// spawned children with stdio:pipe).
#[allow(clippy::fn_params_excessive_bools)]
pub fn build_world_objects_opt(
    terrain: &str,
    phase: &str,
    out_base: Option<&Path>,
    patch_manifest: bool,
    ops_log: bool,
    quiet: bool,
) -> Result<BuildSummary> {
    let object_partitioning::PreparedWorldObjects {
        world_size_m,
        terrain_dir,
        out_base,
        objects_dir,
        export_meta,
        staged_at,
        rules,
        raw_census,
        no_prefab_count,
        no_prefab_classes,
        out_of_bounds,
        rows_with_scale,
        kept,
        density_phase,
        rock_rows,
        rock_out_of_bounds,
        line_count,
        prefabs,
        chunks,
        rows_wide,
        rows_wide_by_kind,
        sorted_chunk_keys,
    } = object_partitioning::prepare_world_objects(terrain, phase, out_base)?;
    let mut classify = Classifier::new(&rules);

    // ---- density grids + forest regions (P2+) ----
    let density_dir = objects_dir.join("density");
    let mut density_summary: Option<Value> = None;
    let mut regions_result: Option<forest::ForestDerivation> = None;
    if density_phase {
        let tree_rows: Vec<&KeptRow> = kept.iter().filter(|k| k.kind == "tree").collect();
        let (tree_grid, tree_size) =
            density::accumulate_corners(tree_rows.iter().map(|k| (k.x, k.y)), world_size_m);
        let (rock_grid, rock_size) =
            density::accumulate_corners(rock_rows.iter().copied(), world_size_m);
        let tree_corner_sum: u64 = tree_grid.iter().map(|&v| u64::from(v)).sum();
        let rock_corner_sum: u64 = rock_grid.iter().map(|&v| u64::from(v)).sum();
        if tree_corner_sum != tree_rows.len() as u64 {
            eprintln!(
                "build-world-objects: FATAL — density tree corner sum {tree_corner_sum} != tree instances {}",
                tree_rows.len()
            );
            std::process::exit(1);
        }
        if rock_corner_sum != rock_rows.len() as u64 {
            eprintln!(
                "build-world-objects: FATAL — density rock corner sum {rock_corner_sum} != rock rows {}",
                rock_rows.len()
            );
            std::process::exit(1);
        }
        let _ = clear_density_dir_if_rebuilding(&density_dir, true)?;
        std::fs::create_dir_all(&density_dir)?;
        // T-176 A2 — the tree channel written to disk is the canopy-blurred grid (density.rs
        // `box_blur_corners`); the raw `tree_grid` above stays only for the PH-P2 sum identity.
        // Blur the GLOBAL grid before slicing so adjacent chunks share identical border corners
        // (no seams). Rocks stay raw counts.
        let tree_canopy =
            density::box_blur_corners(&tree_grid, tree_size, density::CANOPY_KERNEL_RADIUS_CELLS);
        let grid_cells = (world_size_m / CHUNK_SIZE_M).round() as usize;
        let mut density_bytes = 0u64;
        for cy in 0..grid_cells {
            for cx in 0..grid_cells {
                let tree_ch = density::slice_chunk_corners(&tree_canopy, tree_size, cx, cy);
                let rock_ch = density::slice_chunk_corners(&rock_grid, rock_size, cx, cy);
                let buf = website_map_engine::io::density::tbdd::encode_tbdd(
                    density::DENSITY_CELL_M,
                    density::DENSITY_COLS,
                    density::DENSITY_ROWS,
                    &[&tree_ch, &rock_ch],
                );
                density_bytes += buf.len() as u64;
                std::fs::write(
                    density_dir.join(format!("{}.bin", chunk_key(cx as i64, cy as i64))),
                    buf,
                )?;
            }
        }
        let trees: Vec<Tree> = tree_rows
            .iter()
            .map(|k| Tree {
                x: k.x,
                y: k.y,
                class: classify.classify(&k.resource_name).class,
            })
            .collect();
        let mut derived = derive_forest_regions(&trees, world_size_m, terrain);
        // T-149 — the rings out of `trace_rings` are raw marching-squares output on the 32 m
        // region lattice: 100% of their segments are axis-aligned. Round them against the 8 m
        // canopy field (the same `tree_canopy` grid the TBDD tiles above were sliced from) before
        // they are written. `smooth_regions` reports per-region vertex counts and area drift.
        let canopy =
            |x: f64, y: f64| f64::from(density::sample_corners(&tree_canopy, tree_size, x, y));
        let smoothed = forest_smoothing::smooth_regions(&mut derived.regions, Some(&canopy));
        forest_smoothing::log_reports(terrain, &smoothed);
        let mut regions_doc = Map::new();
        regions_doc.insert("schemaVersion".into(), json!("1.0.0"));
        regions_doc.insert("terrainId".into(), json!(terrain));
        regions_doc.insert("generatedAt".into(), json!(staged_at));
        regions_doc.insert("cellM".into(), js_num(forest::REGION_CELL_M));
        regions_doc.insert("densityThreshold".into(), json!(forest::DENSITY_THRESHOLD));
        regions_doc.insert(
            "minComponentCells".into(),
            json!(forest::MIN_COMPONENT_CELLS),
        );
        regions_doc.insert("dominantShare".into(), json!(forest::DOMINANT_SHARE));
        regions_doc.insert("regions".into(), Value::Array(derived.regions.clone()));
        std::fs::write(
            objects_dir.join("forest-regions.json.gz"),
            gz9(compact(&Value::Object(regions_doc)).as_bytes())?,
        )?;
        density_summary = Some(json!({
            "cellM": js_num(f64::from(density::DENSITY_CELL_M)),
            "files": grid_cells * grid_cells,
            "bytes": density_bytes,
            "treeCornerSum": tree_corner_sum,
            "rockCornerSum": rock_corner_sum,
            "rockRawRows": rock_rows.len(),
            "rockOutOfBounds": rock_out_of_bounds,
        }));
        regions_result = Some(derived);
    } else {
        // T-378: a non-density `--phase` must NOT wipe committed density bins.
        let cleared = clear_density_dir_if_rebuilding(&density_dir, false)?;
        debug_assert!(!cleared);
        if density_dir.exists() {
            eprintln!(
                "build-world-objects: leaving existing densify tree intact \
                 (non-density phase; {} present)",
                density_dir.display()
            );
        }
    }

    // ---- census (catalog scope) ----
    let mut inst_by_prefab = vec![0u64; prefabs.len()];
    for list in chunks.values() {
        for row in list {
            inst_by_prefab[row.id] += 1;
        }
    }
    // T-278: this was a local 8-kind array missing T-244's `vehicle`, so the
    // `expect("kind bucket")` below panicked on the first wreck prefab — the reason re-running
    // the export could not have activated the vehicle lane. Single source now.
    let kind_order = super::super::INSTANCE_KINDS;
    let mut by_kind: Map<String, Value> = kind_order
        .iter()
        .map(|k| {
            let mut m = Map::from_iter([
                ("prefabTypes".to_string(), json!(0)),
                ("instances".to_string(), json!(0)),
            ]);
            if *k == "road" {
                // Read from the committed roads.json.gz, never asserted as zero — see `road_census`.
                m.insert(
                    "segments".into(),
                    json!(road_census(&objects_dir).map(|(n, _)| n).unwrap_or(0)),
                );
            }
            (k.to_string(), Value::Object(m))
        })
        .collect();
    let mut by_building_class: Map<String, Value> = Map::new();
    let mut by_species_class: Map<String, Value> = Map::new();
    let rules_arr = rules.doc["rules"].as_array().cloned().unwrap_or_default();
    for (i, p) in prefabs.iter().enumerate() {
        let kind = p["kind"].as_str().unwrap_or_default();
        let class = p["class"].as_str().unwrap_or_default();
        let bk = by_kind
            .get_mut(kind)
            .and_then(Value::as_object_mut)
            .expect("kind bucket");
        *bk.get_mut("prefabTypes").unwrap() = json!(bk["prefabTypes"].as_u64().unwrap() + 1);
        *bk.get_mut("instances").unwrap() =
            json!(bk["instances"].as_u64().unwrap() + inst_by_prefab[i]);
        let target = match kind {
            "building" => Some(&mut by_building_class),
            "tree" | "vegetation" => Some(&mut by_species_class),
            _ => None,
        };
        if let Some(target) = target {
            let bucket = target
                .entry(class.to_string())
                .or_insert_with(|| json!({ "prefabTypes": 0, "instances": 0 }));
            let b = bucket.as_object_mut().unwrap();
            *b.get_mut("prefabTypes").unwrap() = json!(b["prefabTypes"].as_u64().unwrap() + 1);
            *b.get_mut("instances").unwrap() =
                json!(b["instances"].as_u64().unwrap() + inst_by_prefab[i]);
            let iz = rules_arr
                .iter()
                .find(|r| r["kind"] == kind && r["class"] == class)
                .map(|r| r["render"]["importanceZoom"].clone())
                .unwrap_or(Value::Null);
            if iz.is_number() {
                b.insert("importanceZoom".into(), iz);
            }
        }
    }
    let sort_map = |m: Map<String, Value>| -> Map<String, Value> {
        let mut keys: Vec<String> = m.keys().cloned().collect();
        keys.sort();
        keys.into_iter()
            .map(|k| (k.clone(), m[&k].clone()))
            .collect()
    };
    let total_instances = kept.len();
    let mut needs_review: Vec<(String, u64, String)> = raw_census
        .iter()
        .filter(|(_, _, _, _, matched)| !matched)
        .map(|(rn, count, kind, class, _)| {
            (
                rn.clone(),
                *count,
                format!("unclassified (fallback {kind}/{class}) — excluded from {phase} catalog"),
            )
        })
        .collect();
    needs_review.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    let mut inventory = Map::new();
    inventory.insert("schemaVersion".into(), json!("1.0.0"));
    inventory.insert("terrainId".into(), json!(terrain));
    inventory.insert("censusStatus".into(), json!("partial"));
    inventory.insert("generatedAt".into(), json!(staged_at));
    inventory.insert("importPhaseMax".into(), json!(phase));
    inventory.insert(
        "sourceExportPath".into(),
        json!(format!(
            "assets_v2/scratch/{terrain}/export/raw-entities.jsonl"
        )),
    );
    inventory.insert(
        "levels".into(),
        json!({ "uniquePrefabs": prefabs.len(), "totalInstances": total_instances }),
    );
    inventory.insert("byKind".into(), Value::Object(by_kind));
    inventory.insert(
        "byBuildingClass".into(),
        Value::Object(sort_map(by_building_class)),
    );
    inventory.insert(
        "byRoadClass".into(),
        road_census(&objects_dir)
            .map(|(_, by)| Value::Object(by))
            .unwrap_or_else(|| json!({})),
    );
    inventory.insert(
        "bySpeciesClass".into(),
        Value::Object(sort_map(by_species_class)),
    );
    inventory.insert(
        "needsReview".into(),
        json!({
            "prefabTypes": needs_review.len(),
            "prefabs": needs_review.iter().map(|(rn, c, reason)| json!({
                "resourceName": rn, "instanceCount": c, "reason": reason,
            })).collect::<Vec<_>>(),
        }),
    );
    if let Some(r) = &regions_result {
        let tree_count: u64 = r
            .regions
            .iter()
            .map(|reg| reg["treeCount"].as_u64().unwrap_or(0))
            .sum();
        inventory.insert(
            "byRegionKind".into(),
            json!({ "forest": { "count": r.regions.len(), "treeCount": tree_count } }),
        );
        inventory.insert("unassignedTrees".into(), json!(r.unassigned_trees));
    }
    std::fs::write(
        objects_dir.join("type-inventory.json"),
        pretty_nl(&Value::Object(inventory)),
    )?;

    // T-935.11 — dual emission: the rkyv twins of the three catalogue JSONs, built by re-reading
    // the files just written (so they equal the loader's decode by construction) and read back
    // through the SPA's own entry points before this returns. The JSON stays authoritative until
    // T-935.13 flips the manifest.
    for (path, bytes) in catalog_emit::emit_catalog_archives(&out_base)? {
        if !quiet {
            println!(
                "build-world-objects: rkyv → {} ({bytes} bytes)",
                path.display()
            );
        }
    }

    // ---- manifest patch (real terrain dir only) ----
    if patch_manifest {
        let manifest_path = terrain_dir.join("manifest.json");
        let mut manifest: Value = serde_json::from_str(&std::fs::read_to_string(&manifest_path)?)?;
        let obj = manifest["objects"]
            .as_object_mut()
            .expect("manifest.objects");
        let set = |obj: &mut Map<String, Value>, k: &str, v: Value| {
            obj.insert(k.to_string(), v);
        };
        // T-090.12.1 — objects schemaVersion 1.1.0: chunk rows carry [.., pitch, roll, scale] when
        // non-trivial. `transforms` names what every row can carry, `scaleSource` whether the
        // export provided a scale at all (a pre-v2 export is unit scale everywhere), and
        // `workbenchVersion` is copied from the export meta when the plugin wrote one.
        set(obj, "schemaVersion", json!("1.1.0"));
        set(obj, "format", json!("catalog-v1"));
        set(
            obj,
            "transforms",
            json!(if rows_with_scale > 0 {
                "yaw+pitch+roll+scale"
            } else {
                "yaw+pitch+roll"
            }),
        );
        set(
            obj,
            "scaleSource",
            json!(if rows_with_scale > 0 {
                "GetScale"
            } else {
                "absent"
            }),
        );
        match export_meta["workbenchVersion"].as_str() {
            Some(v) => set(obj, "workbenchVersion", json!(v)),
            None => {
                obj.remove("workbenchVersion");
            }
        }
        set(obj, "prefabsPath", json!("objects/prefabs.json.gz"));
        set(obj, "prefabCount", json!(prefabs.len()));
        set(obj, "instanceCount", json!(total_instances));
        set(obj, "chunksPath", json!("objects/chunks"));
        set(obj, "chunkSizeM", js_num(CHUNK_SIZE_M));
        set(obj, "roadsPath", json!("objects/roads.json.gz"));
        set(
            obj,
            "typeInventoryPath",
            json!("objects/type-inventory.json"),
        );
        set(obj, "importPhaseMax", json!(phase));
        let idx = PHASE_ORDER.iter().position(|p| *p == phase).unwrap_or(0);
        set(
            obj,
            "importPhaseShipped",
            json!(PHASE_ORDER[..=idx].to_vec()),
        );
        set(obj, "exportedAt", json!(staged_at));
        if density_phase {
            set(obj, "regionsPath", json!("objects/forest-regions.json.gz"));
            set(obj, "densityPath", json!("objects/density"));
            set(
                obj,
                "densityCellM",
                js_num(f64::from(density::DENSITY_CELL_M)),
            );
            set(
                obj,
                "lod",
                json!({
                    "schemaVersion": "1.0.0",
                    "refZoom": 3,
                    "gates": {
                        "tree": 0, "building": -2.5, "buildingBadge": 1, "forestOutline": -1.5,
                        "forestFillMax": 1, "vegetation": 1.5, "rockLarge": 1, "prop": 3,
                    },
                }),
            );
        }
        std::fs::write(&manifest_path, pretty_nl(&manifest))?;
    }

    // T-090.12.1 — the row-width census: rows carrying a non-trivial transform trailer.
    if !quiet {
        println!(
            "build-world-objects: transforms — {rows_wide} of {} rows 8-wide (by kind {:?}) · {rows_with_scale} raw rows carried scale",
            kept.len(),
            rows_wide_by_kind
        );
    }
    // ---- summary + ops log ----
    let mut top_classes = no_prefab_classes.clone();
    top_classes.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let summary = json!({
        "slice": if density_phase { "T-090.3.2" } else { "T-090.3.1" },
        "phase": phase,
        "stagedAt": staged_at,
        "rawLineCount": line_count,
        "rawUniqueResourceNames": raw_census.len(),
        "transforms": { "rowsWide": rows_wide, "rowsWideByKind": rows_wide_by_kind, "rowsWithScale": rows_with_scale },
        "noPrefab": {
            "count": no_prefab_count,
            "topClassNames": top_classes.iter().take(10).map(|(cn, c)| json!({ "className": cn, "count": c })).collect::<Vec<_>>(),
        },
        "outOfBounds": out_of_bounds,
        "catalog": { "prefabCount": prefabs.len(), "instanceCount": total_instances, "chunkCount": sorted_chunk_keys.len() },
        "unclassifiedRawTypes": needs_review.len(),
    });
    if ops_log {
        let ops_path = repo_root()
            .join(".ai/artifacts")
            .join(format!("map_export_{terrain}.json"));
        let mut ops: Value = if ops_path.exists() {
            serde_json::from_str(&std::fs::read_to_string(&ops_path)?)?
        } else {
            json!({ "terrainId": terrain })
        };
        if !ops["fullExport"].is_object() {
            ops["fullExport"] = json!({});
        }
        ops["fullExport"]["objects"] = summary.clone();
        if density_phase {
            let r = regions_result.as_ref().unwrap();
            let ds = density_summary.clone().unwrap();
            let tree_count: u64 = r
                .regions
                .iter()
                .map(|reg| reg["treeCount"].as_u64().unwrap_or(0))
                .sum();
            if !ops["fullExport"]["phases"].is_object() {
                ops["fullExport"]["phases"] = json!({});
            }
            ops["fullExport"]["phases"][phase] = json!({
                "slice": "T-090.3.2",
                "stagedAt": staged_at,
                "density": ds,
                "forestRegions": {
                    "cellM": js_num(forest::REGION_CELL_M),
                    "densityThreshold": forest::DENSITY_THRESHOLD,
                    "minComponentCells": forest::MIN_COMPONENT_CELLS,
                    "dominantShare": forest::DOMINANT_SHARE,
                    "regionCount": r.regions.len(),
                    "treeCount": tree_count,
                    "unassignedTrees": r.unassigned_trees,
                    "denseCellCount": r.dense_cell_count,
                    "componentCount": r.component_count,
                    "keptComponentCount": r.kept_component_count,
                },
            });
        }
        std::fs::write(&ops_path, pretty_nl(&ops))?;
    }
    if !quiet {
        println!(
            "build-world-objects: {terrain} {phase} — {}",
            compact(&summary)
        );
    }
    Ok(BuildSummary { summary })
}
