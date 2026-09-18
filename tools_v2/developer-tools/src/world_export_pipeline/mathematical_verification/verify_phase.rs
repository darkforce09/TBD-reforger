use super::*;

/// The full verify-phase gate run. Returns the process exit code.
pub fn verify_phase(terrain: &str, phase: &str) -> Result<u8> {
    let Some(kinds) = phase_kinds(phase) else {
        eprintln!("verify-phase: phase '{phase}' not implemented");
        return Ok(1);
    };
    let phase_kind_set: HashSet<&str> = kinds.iter().copied().collect();
    let density_phase = phase == "P2_trees";

    let root = repo_root();
    let terrain_dir = root.join("packages/map-assets").join(terrain);
    let objects_dir = terrain_dir.join("objects");
    let chunks_dir = objects_dir.join("chunks");
    let staging = terrain_dir.join("staging/export");
    let raw_path = staging.join("raw-entities.jsonl");
    if !raw_path.exists() {
        eprintln!(
            "verify-phase: staged raw missing ({}) — run cargo xtask map export-terrain first",
            raw_path.display()
        );
        return Ok(2);
    }

    let schemas = SchemaSet::load()?;
    let v_prefab = schemas.validator("map-object-prefab")?;
    let v_instance = schemas.validator("map-object-instance")?;
    let v_roads = schemas.validator("map-object-roads")?;
    let v_resolved = schemas.validator("map-object-resolved")?;
    let v_inventory = schemas.validator("map-object-type-inventory")?;
    let v_region = schemas.validator("map-object-region")?;

    let registry: Value = serde_json::from_str(&std::fs::read_to_string(
        root.join("packages/map-assets/terrain-registry.json"),
    )?)?;
    let world_size_m = registry["terrains"]
        .as_array()
        .and_then(|a| a.iter().find(|t| t["terrainId"] == terrain))
        .and_then(|t| t["worldBoundsM"][2].as_f64())
        .unwrap_or(0.0);

    let prefabs_doc = gunzip_json(&objects_dir.join("prefabs.json.gz"))?;
    let prefabs = prefabs_doc["prefabs"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let chunk_manifest: Value =
        serde_json::from_str(&std::fs::read_to_string(chunks_dir.join("manifest.json"))?)?;
    let roads_doc = gunzip_json(&objects_dir.join("roads.json.gz"))?;
    let inventory: Value = serde_json::from_str(&std::fs::read_to_string(
        objects_dir.join("type-inventory.json"),
    )?)?;
    let manifest: Value =
        serde_json::from_str(&std::fs::read_to_string(terrain_dir.join("manifest.json"))?)?;

    // ---- stream staged raw once: G11 parity + P1-4 anchor pool + D2 rock rows ----
    let rules = load_rules()?;
    let mut classify = Classifier::new(&rules);
    let mut raw_phase_count = 0u64;
    let mut raw_kind_counts: HashMap<String, u64> = HashMap::new();
    let mut anchor_pool: Vec<Value> = Vec::new();
    let mut raw_rock_rows: Vec<(f64, f64)> = Vec::new();
    let guid_ok = |rn: &str| {
        rn.len() >= 18
            && rn.starts_with('{')
            && rn.as_bytes()[17] == b'}'
            && rn.as_bytes()[1..17]
                .iter()
                .all(|c| c.is_ascii_digit() || (b'A'..=b'F').contains(c))
    };
    stream_raw_entities(&raw_path, |row| {
        let rn = row["resourceName"].as_str().unwrap_or("");
        if rn.is_empty() {
            return;
        }
        let cls = classify.classify(rn);
        let x = round2(row["x"].as_f64().unwrap_or(0.0));
        let y = round2(row["z"].as_f64().unwrap_or(0.0));
        let in_bounds = x >= 0.0 && x <= world_size_m && y >= 0.0 && y <= world_size_m;
        if density_phase && cls.kind == "rock" && in_bounds && !phase_kind_set.contains("rock") {
            raw_rock_rows.push((x, y));
        }
        if !phase_kind_set.contains(cls.kind.as_str()) {
            return;
        }
        if cls.class == "composition" || cls.class == "buildingpart" {
            return;
        }
        if !guid_ok(rn) || !in_bounds {
            return;
        }
        raw_phase_count += 1;
        *raw_kind_counts.entry(cls.kind.clone()).or_insert(0) += 1;
        if phase == "P1_buildings" {
            anchor_pool.push(json!({
                "resourceName": rn, "x": row["x"], "y": row["y"], "z": row["z"],
                "headingDeg": row["headingDeg"].as_f64().or_else(|| row["pitchDeg"].as_f64()).unwrap_or(0.0),
            }));
        }
    })?;

    // ---- load all chunk rows once ----
    let mut chunk_aggregate_bytes = 0u64;
    let mut rows_by_key: Vec<(String, Vec<Value>)> = Vec::new();
    let mut files: Vec<String> = std::fs::read_dir(&chunks_dir)?
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|f| f.ends_with(".json.gz"))
        .collect();
    files.sort();
    for f in &files {
        let p = chunks_dir.join(f);
        chunk_aggregate_bytes += std::fs::metadata(&p)?.len();
        let doc = gunzip_json(&p)?;
        rows_by_key.push((
            f.trim_end_matches(".json.gz").to_string(),
            doc["instances"].as_array().cloned().unwrap_or_default(),
        ));
    }
    let actual_instance_count: usize = rows_by_key.iter().map(|(_, r)| r.len()).sum();

    let mut committed_kind_counts: HashMap<String, u64> = HashMap::new();
    for (_, rows) in &rows_by_key {
        for row in rows {
            let k = row[0]
                .as_u64()
                .and_then(|i| prefabs.get(i as usize))
                .and_then(|p| p["kind"].as_str())
                .unwrap_or("")
                .to_string();
            *committed_kind_counts.entry(k).or_insert(0) += 1;
        }
    }
    let committed_phase_count: u64 = committed_kind_counts
        .iter()
        .filter(|(k, _)| phase_kind_set.contains(k.as_str()))
        .map(|(_, n)| n)
        .sum();

    let mut g = Gates::default();
    let _first_err = |v: &jsonschema::Validator, val: &Value| -> String {
        v.iter_errors(val)
            .next()
            .map(|e| e.to_string())
            .unwrap_or_default()
    };

    // ---- G1 schema validity ----
    artifact_integrity::verify(
        &mut g,
        artifact_integrity::ArtifactData {
            prefabs: &prefabs,
            rows_by_key: &rows_by_key,
            roads_doc: &roads_doc,
            inventory: &inventory,
            chunk_manifest: &chunk_manifest,
        },
        artifact_integrity::ArtifactValidators {
            v_prefab: &v_prefab,
            v_instance: &v_instance,
            v_roads: &v_roads,
            v_inventory: &v_inventory,
            v_resolved: &v_resolved,
        },
    );

    // ---- G6 chunk partition + G8 bounds ----
    {
        let mut g6 = Vec::new();
        let mut g8 = Vec::new();
        for (key, rows) in &rows_by_key {
            let mut it = key.split('_').map(|v| v.parse::<i64>().unwrap_or(-1));
            let (cx, cy) = (it.next().unwrap_or(-1), it.next().unwrap_or(-1));
            for (i, row) in rows.iter().enumerate() {
                let (x, y) = (
                    row[1].as_f64().unwrap_or(0.0),
                    row[2].as_f64().unwrap_or(0.0),
                );
                let (px, py) = (
                    cell_of(x, CHUNK_SIZE_M, world_size_m),
                    cell_of(y, CHUNK_SIZE_M, world_size_m),
                );
                if px != cx || py != cy {
                    g6.push(format!(
                        "chunk {key}[{i}]: ({x}, {y}) partitions to {}",
                        chunk_key(px, py)
                    ));
                }
                if x < 0.0 || x > world_size_m || y < 0.0 || y > world_size_m {
                    g8.push(format!("chunk {key}[{i}]: ({x}, {y}) outside world bounds"));
                }
            }
        }
        g.gate("G6", "chunk partition (clamp(floor(coord/512)))", g6);
        g.gate("G8", "world bounds 0 <= x,y <= maxX", g8);
    }

    // ---- G7 count identities ----
    {
        let mut errs = Vec::new();
        let sidecar_sum: u64 = chunk_manifest["cells"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|c| c["instanceCount"].as_u64().unwrap_or(0))
                    .sum()
            })
            .unwrap_or(0);
        if sidecar_sum != actual_instance_count as u64 {
            errs.push(format!(
                "sidecar sum {sidecar_sum} != actual rows {actual_instance_count}"
            ));
        }
        if manifest["objects"]["instanceCount"].as_u64() != Some(actual_instance_count as u64) {
            errs.push(format!(
                "manifest.objects.instanceCount {} != actual {actual_instance_count}",
                manifest["objects"]["instanceCount"]
            ));
        }
        if manifest["objects"]["prefabCount"].as_u64() != Some(prefabs.len() as u64) {
            errs.push(format!(
                "manifest.objects.prefabCount {} != prefabs {}",
                manifest["objects"]["prefabCount"],
                prefabs.len()
            ));
        }
        if inventory["levels"]["totalInstances"].as_u64() != Some(actual_instance_count as u64) {
            errs.push(format!(
                "inventory totalInstances {} != actual {actual_instance_count}",
                inventory["levels"]["totalInstances"]
            ));
        }
        if inventory["levels"]["uniquePrefabs"].as_u64() != Some(prefabs.len() as u64) {
            errs.push(format!(
                "inventory uniquePrefabs {} != prefabs {}",
                inventory["levels"]["uniquePrefabs"],
                prefabs.len()
            ));
        }
        g.gate(
            "G7",
            "count identities (sidecar = files = manifest = inventory)",
            errs,
        );
    }

    // ---- G9 / G10 prefab field sanity ----
    {
        let g9: Vec<String> = prefabs
            .iter()
            .filter(|p| {
                !matches!(
                    p["gameplay"]["cover"]["type"].as_str(),
                    Some("none" | "soft" | "hard")
                )
            })
            .map(|p| {
                format!(
                    "prefab {}: cover '{}'",
                    p["prefabId"], p["gameplay"]["cover"]["type"]
                )
            })
            .collect();
        let mut g10 = Vec::new();
        for p in &prefabs {
            if !p["spatial"]["heightM"].as_f64().is_some_and(|h| h >= 0.0) {
                g10.push(format!(
                    "prefab {}: heightM {}",
                    p["prefabId"], p["spatial"]["heightM"]
                ));
            }
            let he = &p["spatial"]["halfExtentsM"];
            if he.is_object()
                && !(he["x"].as_f64().is_some_and(|v| v >= 0.0)
                    && he["y"].as_f64().is_some_and(|v| v >= 0.0)
                    && he["z"].as_f64().is_some_and(|v| v >= 0.0))
            {
                g10.push(format!("prefab {}: negative halfExtentsM", p["prefabId"]));
            }
        }
        g.gate("G9", "gameplay.cover.type enum", g9);
        g.gate("G10", "spatial positive (heightM, halfExtentsM)", g10);
    }

    // ---- G11 raw <-> catalog parity ----
    {
        let errs = if raw_phase_count == committed_phase_count {
            vec![]
        } else {
            vec![format!(
                "raw phase-filtered count {raw_phase_count} != committed phase-kind instances {committed_phase_count}"
            )]
        };
        g.gate(
            "G11",
            &format!("raw <-> catalog count parity for {phase} filter"),
            errs,
        );
    }

    // ---- P1 gates ----
    if phase == "P1_buildings" {
        run_p1_gates(
            &mut g,
            &prefabs,
            &inventory,
            &manifest,
            &anchor_pool,
            &rows_by_key,
            world_size_m,
        );
    }

    // ---- PH-P2 + D + F ----
    if density_phase {
        run_p2_gates(
            &mut g,
            &prefabs,
            &inventory,
            &rows_by_key,
            &raw_kind_counts,
            &committed_kind_counts,
            &raw_rock_rows,
            &objects_dir,
            world_size_m,
            terrain,
            &v_region,
        )?;
    }

    // ---- roads (Q1 pulled forward) ----
    {
        let mut errs = Vec::new();
        let segs = roads_doc["roadSegments"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        if segs.is_empty() {
            errs.push("roads.json.gz has 0 segments".into());
        }
        for (i, s) in segs.iter().enumerate() {
            if s["points"].as_array().map(Vec::len).unwrap_or(0) < 2 {
                errs.push(format!("segment {i} {}: < 2 points", s["id"]));
            }
        }
        g.gate(
            "R-P1",
            "roads present (segments > 0, polylines >= 2 points)",
            errs,
        );
    }

    // ---- P5 fence census ----
    if phase == "P5_props" {
        let fence_ids: HashSet<u64> = prefabs
            .iter()
            .filter(|p| p["kind"] == "prop" && p["class"] == "fence")
            .filter_map(|p| p["prefabId"].as_u64())
            .collect();
        let mut fence_inst = 0u64;
        for (_, rows) in &rows_by_key {
            for row in rows {
                if row[0].as_u64().is_some_and(|id| fence_ids.contains(&id)) {
                    fence_inst += 1;
                }
            }
        }
        let mut errs = Vec::new();
        if fence_ids.is_empty() {
            errs.push("no fence prefabs (G1)".into());
        }
        if fence_inst == 0 {
            errs.push("no fence instances (G2)".into());
        }
        g.gate(
            "P5-1",
            "fence prefabs > 0 and fence instances > 0 (T-152.4 G1/G2)",
            errs,
        );
    }

    // ---- size guard ----
    g.gate(
        "SIZE",
        &format!(
            "chunk gz aggregate <= {} MB (forces LFS decision before P2)",
            MAX_CHUNK_AGGREGATE_BYTES / 1024 / 1024
        ),
        if chunk_aggregate_bytes <= MAX_CHUNK_AGGREGATE_BYTES {
            vec![]
        } else {
            vec![format!(
                "aggregate {:.1} MB",
                chunk_aggregate_bytes as f64 / 1024.0 / 1024.0
            )]
        },
    );

    // ---- E6 / G4 / I6 determinism: double scratch build + committed byte-compare ----
    {
        let mut errs = Vec::new();
        let rebuild_phase = manifest["objects"]["importPhaseMax"]
            .as_str()
            .unwrap_or(phase)
            .to_string();
        let s1 = tempdir("tbd-vp1-")?;
        let s2 = tempdir("tbd-vp2-")?;
        let run = || -> Result<()> {
            for out in [&s1, &s2] {
                build_world_objects_opt(terrain, &rebuild_phase, Some(out), false, false, true)?;
                build_roads_from_topo_opt(terrain, Some(out), false, true)?;
            }
            Ok(())
        };
        match run() {
            Err(e) => errs.push(format!(
                "scratch build failed: {}",
                e.to_string().chars().take(200).collect::<String>()
            )),
            Ok(()) => {
                let f1 = list_files(&s1.join("objects"))?;
                let f2 = list_files(&s2.join("objects"))?;
                if f1 != f2 {
                    errs.push("scratch builds produced different file sets".into());
                }
                for rel in &f1 {
                    let b1 = std::fs::read(s1.join("objects").join(rel))?;
                    if b1 != std::fs::read(s2.join("objects").join(rel))? {
                        errs.push(format!("nondeterministic: {rel}"));
                    }
                    let committed = objects_dir.join(rel);
                    if !committed.exists() {
                        errs.push(format!("committed missing: objects/{rel}"));
                    } else if b1 != std::fs::read(&committed)? {
                        errs.push(format!("committed stale vs rebuild: objects/{rel}"));
                    }
                }
            }
        }
        let _ = std::fs::remove_dir_all(&s1);
        let _ = std::fs::remove_dir_all(&s2);
        g.gate("E6", "determinism — double scratch build byte-identical AND committed artifacts current (G4 + I6)", errs);
    }

    // ---- report ----
    let mut failures = 0usize;
    for gate in &g.0 {
        if gate.err_count == 0 {
            println!("  PASS  {} — {}", gate.id, gate.label);
        } else {
            failures += gate.err_count;
            println!(
                "  FAIL  {} — {} ({} error(s))",
                gate.id, gate.label, gate.err_count
            );
            for e in &gate.errs {
                println!("        {e}");
            }
        }
    }
    if failures > 0 {
        eprintln!("\nmap-verify-phase: FAIL — {terrain} {phase} ({failures} error(s))");
        return Ok(1);
    }
    println!(
        "\nmap-verify-phase: OK — {terrain} {phase} ({} prefabs, {actual_instance_count} instances, {} chunks, {} road segments, chunk gz {:.0} KB)",
        prefabs.len(),
        rows_by_key.len(),
        roads_doc["roadSegments"]
            .as_array()
            .map(Vec::len)
            .unwrap_or(0),
        chunk_aggregate_bytes as f64 / 1024.0
    );
    Ok(0)
}
