//! T-165.8 — the mathematical phase gate (port of `scripts/map-assets/verify-phase.mjs`):
//! G1-G12 global invariants + P1-*/PH-P2-* phase gates + D/F density-forest gates + E6/G4/I6
//! determinism on the STAGED raw export and the COMMITTED objects/ artifacts. Phases are
//! CUMULATIVE — phase-scoped gates filter committed rows to the requested phase's kinds while
//! catalog-scope gates run on the whole committed set; E6 rebuilds at the COMMITTED
//! importPhaseMax (in-process double scratch build — the Node script spawned itself).

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::{Value, json};

use super::build::{
    CHUNK_SIZE_M, build_roads_from_topo_opt, build_world_objects_opt, gunzip, phase_kinds,
};
use super::classify::{Classifier, load_rules, stream_raw_entities};
use super::jsval::round2;
use crate::forest::{self, Tree, derive_forest_regions};
use crate::geometry::{cell_of, check_anchors, chunk_key};
use crate::serve::repo_root;
use crate::{density, geometry};

const MAX_CHUNK_AGGREGATE_BYTES: u64 = 40 * 1024 * 1024;

pub struct SchemaSet {
    registry: jsonschema::Registry<'static>,
    schemas: HashMap<&'static str, Value>,
}

pub const MAP_OBJECT_SCHEMAS: [&str; 9] = [
    "map-object-enums",
    "map-object-prefab",
    "map-object-instance",
    "map-object-region",
    "map-object-roads",
    "map-object-catalog",
    "map-object-resolved",
    "map-object-type-inventory",
    "terrain-registry",
];

impl SchemaSet {
    pub fn load() -> Result<SchemaSet> {
        let dir = repo_root().join("packages/tbd-schema/schema");
        let mut registered: Vec<(String, Value)> = Vec::new();
        let mut schemas = HashMap::new();
        for name in MAP_OBJECT_SCHEMAS {
            let doc: Value = serde_json::from_str(
                &std::fs::read_to_string(dir.join(format!("{name}.schema.json")))
                    .with_context(|| name.to_string())?,
            )?;
            let id = doc["$id"].as_str().unwrap_or_default().to_string();
            registered.push((id, doc.clone()));
            schemas.insert(name, doc);
        }
        let registry = jsonschema::Registry::new()
            .extend(
                registered
                    .into_iter()
                    .map(|(id, doc)| (id, jsonschema::Resource::from_contents(doc))),
            )
            .map_err(|e| anyhow::anyhow!("registry: {e}"))?
            .prepare()
            .map_err(|e| anyhow::anyhow!("registry prepare: {e}"))?;
        Ok(SchemaSet { registry, schemas })
    }

    pub fn validator(&self, name: &str) -> Result<jsonschema::Validator> {
        let doc = self
            .schemas
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("unknown schema {name}"))?;
        jsonschema::options()
            .with_registry(&self.registry)
            .build(doc)
            .map_err(|e| anyhow::anyhow!("compile {name}: {e}"))
    }
}

pub fn gunzip_json(p: &Path) -> Result<Value> {
    let raw = gunzip(&std::fs::read(p).with_context(|| p.display().to_string())?)?;
    Ok(serde_json::from_slice(&raw)?)
}

struct Gate {
    id: String,
    label: String,
    errs: Vec<String>,
    err_count: usize,
}

#[derive(Default)]
struct Gates(Vec<Gate>);

impl Gates {
    fn gate(&mut self, id: &str, label: &str, errs: Vec<String>) {
        self.0.push(Gate {
            id: id.to_string(),
            label: label.to_string(),
            err_count: errs.len(),
            errs: errs.into_iter().take(8).collect(),
        });
    }
}

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
            "verify-phase: staged raw missing ({}) — run make map-export first",
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
    let first_err = |v: &jsonschema::Validator, val: &Value| -> String {
        v.iter_errors(val)
            .next()
            .map(|e| e.to_string())
            .unwrap_or_default()
    };

    // ---- G1 schema validity ----
    {
        let mut errs = Vec::new();
        for p in &prefabs {
            if !v_prefab.is_valid(p) {
                errs.push(format!(
                    "prefab {}: {}",
                    p["prefabId"],
                    first_err(&v_prefab, p)
                ));
            }
        }
        for (key, rows) in &rows_by_key {
            for (i, row) in rows.iter().enumerate() {
                if !v_instance.is_valid(row) {
                    errs.push(format!("chunk {key}[{i}]: {}", first_err(&v_instance, row)));
                } else if row.as_array().map(Vec::len) != Some(5) || !row[0].is_number() {
                    errs.push(format!("chunk {key}[{i}]: not a 5-number tuple"));
                }
            }
        }
        if !v_roads.is_valid(&roads_doc) {
            errs.push(format!(
                "roads.json.gz: {}",
                first_err(&v_roads, &roads_doc)
            ));
        }
        if !v_inventory.is_valid(&inventory) {
            errs.push(format!(
                "type-inventory.json: {}",
                first_err(&v_inventory, &inventory)
            ));
        }
        g.gate(
            "G1",
            "schema valid (prefabs, chunk rows, roads, inventory)",
            errs,
        );
    }

    // ---- G2 resolved materialization ----
    {
        let mut errs = Vec::new();
        for (key, rows) in &rows_by_key {
            for (i, row) in rows.iter().enumerate() {
                let Some(p) = row[0].as_u64().and_then(|id| prefabs.get(id as usize)) else {
                    continue; // G3's finding
                };
                let resolved = json!({
                    "id": format!("{key}:{i}"),
                    "prefabId": p["prefabId"], "resourceName": p["resourceName"],
                    "kind": p["kind"], "class": p["class"],
                    "label": p["label"].as_str().unwrap_or(""),
                    "taxonomyPath": p["ai"]["taxonomyPath"], "summary": p["ai"]["summary"],
                    "x": row[1], "y": row[2], "z": row[3], "rotationDeg": row[4],
                    "spatial": p["spatial"], "gameplay": p["gameplay"],
                    "tags": p["tags"].as_array().cloned().unwrap_or_default(),
                });
                if !v_resolved.is_valid(&resolved) {
                    errs.push(format!(
                        "resolved {key}:{i}: {}",
                        first_err(&v_resolved, &resolved)
                    ));
                }
            }
        }
        g.gate(
            "G2",
            "all instances materialize to valid ResolvedWorldObject",
            errs,
        );
    }

    // ---- G3 / G12 prefab bijection + orphans ----
    {
        let mut errs = Vec::new();
        let mut referenced = vec![0u64; prefabs.len()];
        for (key, rows) in &rows_by_key {
            for (i, row) in rows.iter().enumerate() {
                match row[0].as_i64() {
                    Some(id) if id >= 0 && (id as usize) < prefabs.len() => {
                        referenced[id as usize] += 1
                    }
                    other => errs.push(format!(
                        "chunk {key}[{i}]: prefabId {} out of range",
                        other.map_or_else(|| row[0].to_string(), |v| v.to_string())
                    )),
                }
            }
        }
        g.gate("G3", "prefabId bijection (0 <= id < prefabs.length)", errs);
        let orphans: Vec<String> = prefabs
            .iter()
            .enumerate()
            .filter(|(i, p)| {
                referenced[*i] == 0
                    && !p["tags"]
                        .as_array()
                        .is_some_and(|t| t.iter().any(|x| x == "prefabOnly"))
            })
            .map(|(_, p)| {
                format!(
                    "prefab {} {} has 0 instances",
                    p["prefabId"],
                    p["resourceName"].as_str().unwrap_or("")
                )
            })
            .collect();
        g.gate("G12", "no orphan prefabs", orphans);
    }

    // ---- G5 derived-id uniqueness + sidecar consistency ----
    {
        let mut errs = Vec::new();
        let cells = chunk_manifest["cells"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let sidecar_keys: HashSet<String> = cells
            .iter()
            .map(|c| {
                chunk_key(
                    c["cx"].as_i64().unwrap_or(-1),
                    c["cy"].as_i64().unwrap_or(-1),
                )
            })
            .collect();
        if sidecar_keys.len() != cells.len() {
            errs.push("chunks/manifest.json: duplicate (cx,cy) cells".into());
        }
        let by_key: HashMap<&str, usize> = rows_by_key
            .iter()
            .map(|(k, r)| (k.as_str(), r.len()))
            .collect();
        for c in &cells {
            let key = chunk_key(
                c["cx"].as_i64().unwrap_or(-1),
                c["cy"].as_i64().unwrap_or(-1),
            );
            match by_key.get(key.as_str()) {
                None => errs.push(format!("sidecar cell {key}: chunk file missing")),
                Some(&n) if n as u64 != c["instanceCount"].as_u64().unwrap_or(0) => {
                    errs.push(format!(
                        "sidecar cell {key}: instanceCount {} != actual {n}",
                        c["instanceCount"]
                    ))
                }
                _ => {}
            }
        }
        for (key, _) in &rows_by_key {
            if !sidecar_keys.contains(key) {
                errs.push(format!("chunk file {key} not in sidecar manifest"));
            }
        }
        g.gate(
            "G5",
            "derived instance ids unique (sidecar <-> files consistent)",
            errs,
        );
    }

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

fn tempdir(prefix: &str) -> Result<PathBuf> {
    // mkdtemp equivalent without a dep: pid+counter suffix under the system tmpdir.
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let d = std::env::temp_dir().join(format!(
        "{prefix}{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::SeqCst)
    ));
    std::fs::create_dir_all(&d)?;
    Ok(d)
}

fn list_files(dir: &Path) -> Result<Vec<String>> {
    fn walk(dir: &Path, base: &Path, acc: &mut Vec<String>) -> Result<()> {
        for e in std::fs::read_dir(dir)? {
            let e = e?;
            let p = e.path();
            if p.is_dir() {
                walk(&p, base, acc)?;
            } else {
                acc.push(p.strip_prefix(base).unwrap().to_string_lossy().into_owned());
            }
        }
        Ok(())
    }
    let mut acc = Vec::new();
    walk(dir, dir, &mut acc)?;
    acc.sort();
    Ok(acc)
}

#[allow(clippy::too_many_arguments)]
fn run_p1_gates(
    g: &mut Gates,
    prefabs: &[Value],
    inventory: &Value,
    manifest: &Value,
    anchor_pool: &[Value],
    rows_by_key: &[(String, Vec<Value>)],
    world_size_m: f64,
) {
    let buildings: Vec<&Value> = prefabs.iter().filter(|p| p["kind"] == "building").collect();
    {
        let allowed: HashSet<&str> = phase_kinds(
            manifest["objects"]["importPhaseMax"]
                .as_str()
                .unwrap_or("P1_buildings"),
        )
        .unwrap_or(&["building"])
        .iter()
        .copied()
        .collect();
        let mut errs: Vec<String> = prefabs
            .iter()
            .filter(|p| !allowed.contains(p["kind"].as_str().unwrap_or("")))
            .map(|p| {
                format!(
                    "prefab {} kind={} outside importPhaseMax kinds",
                    p["prefabId"],
                    p["kind"].as_str().unwrap_or("")
                )
            })
            .collect();
        if buildings.is_empty() {
            errs.push("no kind=building prefabs in catalog".into());
        }
        g.gate(
            "P1-1",
            "building prefabs present; catalog kinds within committed importPhaseMax",
            errs,
        );
    }
    {
        let exempt = |p: &Value| {
            p["tags"]
                .as_array()
                .is_some_and(|t| t.iter().any(|x| x == "ruin-open"))
                || p["class"] == "tent"
        };
        let hard = buildings
            .iter()
            .filter(|p| p["gameplay"]["cover"]["type"] == "hard" || exempt(p))
            .count();
        let pct = if buildings.is_empty() {
            1.0
        } else {
            hard as f64 / buildings.len() as f64
        };
        let errs = if pct >= 0.995 {
            vec![]
        } else {
            vec![format!(
                "only {:.2}% hard: {}",
                pct * 100.0,
                buildings
                    .iter()
                    .filter(|p| p["gameplay"]["cover"]["type"] != "hard" && !exempt(p))
                    .map(|p| p["resourceName"].as_str().unwrap_or("").to_string())
                    .take(5)
                    .collect::<Vec<_>>()
                    .join(", ")
            )]
        };
        g.gate(
            "P1-2",
            "cover=hard >= 99.5% (ruin-open + tent exceptions allowed)",
            errs,
        );
    }
    g.gate(
        "P1-3",
        "footprint or OBB volume > 0 per building prefab",
        buildings
            .iter()
            .filter(|p| {
                let fp = p["spatial"]["footprintM2"].as_f64().unwrap_or(0.0);
                let he = &p["spatial"]["halfExtentsM"];
                let vol = he["x"].as_f64().unwrap_or(0.0)
                    * he["y"].as_f64().unwrap_or(0.0)
                    * he["z"].as_f64().unwrap_or(0.0);
                !(fp > 0.0 || vol > 0.0)
            })
            .map(|p| {
                format!(
                    "prefab {} {}",
                    p["prefabId"],
                    p["resourceName"].as_str().unwrap_or("")
                )
            })
            .collect(),
    );
    {
        // P1-4 — K=32 deterministic anchors (sort, even spacing, min/max x, boundary rows).
        const K: usize = 32;
        let mut pool = anchor_pool.to_vec();
        pool.sort_by(|a, b| {
            a["resourceName"]
                .as_str()
                .cmp(&b["resourceName"].as_str())
                .then(
                    a["x"]
                        .as_f64()
                        .partial_cmp(&b["x"].as_f64())
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
                .then(
                    a["z"]
                        .as_f64()
                        .partial_cmp(&b["z"].as_f64())
                        .unwrap_or(std::cmp::Ordering::Equal),
                )
        });
        let errs = if pool.is_empty() {
            vec!["no building rows in staged raw".to_string()]
        } else {
            let mut picks: HashSet<usize> = HashSet::new();
            for i in 0..K {
                picks.insert(
                    ((i as f64 * (pool.len() - 1) as f64) / (K - 1) as f64).round() as usize,
                );
            }
            let mut by_x: Vec<usize> = (0..pool.len()).collect();
            by_x.sort_by(|&a, &b| {
                pool[a]["x"]
                    .as_f64()
                    .partial_cmp(&pool[b]["x"].as_f64())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            picks.insert(by_x[0]);
            picks.insert(*by_x.last().unwrap());
            let mut boundary = 0;
            for (i, r) in pool.iter().enumerate() {
                if boundary >= 4 {
                    break;
                }
                let rx = round2(r["x"].as_f64().unwrap_or(0.0));
                let rz = round2(r["z"].as_f64().unwrap_or(0.0));
                if rx % CHUNK_SIZE_M < 1.0 || rz % CHUNK_SIZE_M < 1.0 {
                    picks.insert(i);
                    boundary += 1;
                }
            }
            let mut idx: Vec<usize> = picks.into_iter().collect();
            idx.sort_unstable();
            let anchors: Vec<Value> = idx.into_iter().map(|i| pool[i].clone()).collect();
            let by_key: HashMap<String, &Vec<Value>> =
                rows_by_key.iter().map(|(k, r)| (k.clone(), r)).collect();
            check_anchors(
                &anchors,
                prefabs,
                |cx, cy| {
                    by_key
                        .get(&geometry::chunk_key(cx, cy))
                        .map(|rows| json!({ "instances": rows }))
                },
                CHUNK_SIZE_M,
                world_size_m,
                2.0,
            )
        };
        g.gate(
            "P1-4",
            &format!(
                "K=32 anchor sample <= 2 m via committed chunks ({} anchors)",
                std::cmp::min(32 + 6, anchor_pool.len())
            ),
            errs,
        );
    }
    {
        let mut errs = Vec::new();
        let classes = inventory["byBuildingClass"]
            .as_object()
            .map(|m| m.len())
            .unwrap_or(0);
        if classes == 0 {
            errs.push("byBuildingClass empty".into());
        }
        let unknown = inventory["byBuildingClass"]["unknown"]["instances"]
            .as_f64()
            .unwrap_or(0.0);
        let raw_total = inventory["byKind"]["building"]["instances"]
            .as_f64()
            .unwrap_or(0.0);
        let total = if raw_total == 0.0 { 1.0 } else { raw_total };
        if unknown / total >= 0.005 {
            errs.push(format!("byBuildingClass.unknown {unknown}/{total} >= 0.5%"));
        }
        g.gate(
            "P1-6",
            "byBuildingClass populated; unknown < 0.5% of building instances",
            errs,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn run_p2_gates(
    g: &mut Gates,
    prefabs: &[Value],
    inventory: &Value,
    rows_by_key: &[(String, Vec<Value>)],
    raw_kind_counts: &HashMap<String, u64>,
    committed_kind_counts: &HashMap<String, u64>,
    raw_rock_rows: &[(f64, f64)],
    objects_dir: &Path,
    world_size_m: f64,
    terrain: &str,
    v_region: &jsonschema::Validator,
) -> Result<()> {
    let phase_kind_set: HashSet<&str> = ["building", "tree", "water"].into_iter().collect();
    let tree_prefabs: Vec<&Value> = prefabs.iter().filter(|p| p["kind"] == "tree").collect();
    let mut tree_rows: Vec<(f64, f64, String)> = Vec::new();
    for (_, rows) in rows_by_key {
        for row in rows {
            if let Some(p) = row[0].as_u64().and_then(|i| prefabs.get(i as usize))
                && p["kind"] == "tree"
            {
                tree_rows.push((
                    row[1].as_f64().unwrap_or(0.0),
                    row[2].as_f64().unwrap_or(0.0),
                    p["class"].as_str().unwrap_or("").to_string(),
                ));
            }
        }
    }

    {
        let mut errs: Vec<String> = prefabs
            .iter()
            .filter(|p| !phase_kind_set.contains(p["kind"].as_str().unwrap_or("")))
            .map(|p| {
                format!(
                    "prefab {} kind={}",
                    p["prefabId"],
                    p["kind"].as_str().unwrap_or("")
                )
            })
            .collect();
        if tree_prefabs.is_empty() {
            errs.push("no kind=tree prefabs in catalog".into());
        }
        if committed_kind_counts.get("building").copied().unwrap_or(0) == 0 {
            errs.push("cumulative rule broken: 0 building instances in P2 catalog".into());
        }
        g.gate(
            "PH-P2-1",
            "cumulative P1+P2 catalog; kinds subset {building, tree}; trees present",
            errs,
        );
    }
    g.gate(
        "PH-P2-2",
        "tree prefabs cover=soft (dead exception)",
        tree_prefabs
            .iter()
            .filter(|p| p["gameplay"]["cover"]["type"] != "soft" && p["class"] != "dead")
            .map(|p| {
                format!(
                    "prefab {} {} cover={}",
                    p["prefabId"],
                    p["resourceName"].as_str().unwrap_or(""),
                    p["gameplay"]["cover"]["type"]
                )
            })
            .collect(),
    );
    {
        let tall = tree_prefabs
            .iter()
            .filter(|p| p["spatial"]["heightM"].as_f64().unwrap_or(0.0) >= 2.0)
            .count();
        let pct = if tree_prefabs.is_empty() {
            0.0
        } else {
            tall as f64 / tree_prefabs.len() as f64
        };
        g.gate(
            "PH-P2-3",
            "heightM >= 2 for >= 95% of tree prefabs",
            if pct >= 0.95 {
                vec![]
            } else {
                vec![format!("only {:.2}% >= 2 m", pct * 100.0)]
            },
        );
    }
    {
        let raw_tree = raw_kind_counts.get("tree").copied().unwrap_or(0);
        let committed_tree = committed_kind_counts.get("tree").copied().unwrap_or(0);
        g.gate(
            "PH-P2-4",
            "G11 count conservation for kind=tree only",
            if raw_tree == committed_tree {
                vec![]
            } else {
                vec![format!(
                    "raw tree count {raw_tree} != committed tree instances {committed_tree}"
                )]
            },
        );
    }

    let (tree_grid, tree_size) =
        density::accumulate_corners(tree_rows.iter().map(|(x, y, _)| (*x, *y)), world_size_m);
    let (rock_grid, rock_size) =
        density::accumulate_corners(raw_rock_rows.iter().copied(), world_size_m);

    {
        let sum: u64 = tree_grid.iter().map(|&v| u64::from(v)).sum();
        let committed_tree = committed_kind_counts.get("tree").copied().unwrap_or(0);
        g.gate(
            "PH-P2-5",
            "density insert identity (sum of global tree corners = tree instances)",
            if sum == committed_tree {
                vec![]
            } else {
                vec![format!(
                    "corner sum {sum} != tree instances {committed_tree}"
                )]
            },
        );
    }

    {
        let mut d1 = Vec::new();
        let mut d2 = Vec::new();
        let density_dir = objects_dir.join("density");
        let grid_cells = (world_size_m / CHUNK_SIZE_M).round() as usize;
        let mut expected: HashSet<String> = HashSet::new();
        for cy in 0..grid_cells {
            for cx in 0..grid_cells {
                expected.insert(chunk_key(cx as i64, cy as i64));
            }
        }
        if density_dir.exists() {
            for f in std::fs::read_dir(&density_dir)? {
                let name = f?.file_name().into_string().unwrap_or_default();
                if name.ends_with(".bin") && !expected.contains(name.trim_end_matches(".bin")) {
                    d1.push(format!("unexpected density file {name}"));
                }
            }
        }
        let mut keys: Vec<&String> = expected.iter().collect();
        keys.sort();
        for key in keys {
            let p = density_dir.join(format!("{key}.bin"));
            if !p.exists() {
                d1.push(format!("missing density file {key}.bin"));
                continue;
            }
            let buf = std::fs::read(&p)?;
            if buf.len() != density::TBDD_FILE_BYTES {
                d1.push(format!(
                    "{key}.bin: {} bytes, want {}",
                    buf.len(),
                    density::TBDD_FILE_BYTES
                ));
                continue;
            }
            let dec = match map_engine_core::geometry::tbdd::decode_tbdd(&buf) {
                Ok(d) => d,
                Err(e) => {
                    d1.push(format!("{key}.bin: {e}"));
                    continue;
                }
            };
            if dec.version != density::TBDD_VERSION
                || dec.cell_m != density::DENSITY_CELL_M
                || dec.cols != density::DENSITY_COLS
                || dec.rows != density::DENSITY_ROWS
                || dec.channels.len() != density::DENSITY_CHANNELS.len()
            {
                d1.push(format!("{key}.bin: header mismatch"));
                continue;
            }
            let mut it = key.split('_').map(|v| v.parse::<usize>().unwrap_or(0));
            let (cx, cy) = (it.next().unwrap_or(0), it.next().unwrap_or(0));
            let rebuilt = map_engine_core::geometry::tbdd::encode_tbdd(
                density::DENSITY_CELL_M,
                density::DENSITY_COLS,
                density::DENSITY_ROWS,
                &[
                    &density::slice_chunk_corners(&tree_grid, tree_size, cx, cy),
                    &density::slice_chunk_corners(&rock_grid, rock_size, cx, cy),
                ],
            );
            if buf != rebuilt {
                d2.push(format!(
                    "{key}.bin differs from recompute (committed chunks + raw rocks)"
                ));
            }
        }
        g.gate(
            "D1",
            &format!(
                "density files complete ({} cells), TBDD header + size exact",
                expected.len()
            ),
            d1,
        );
        g.gate(
            "D2",
            "density byte-identical to recompute from committed chunks + staged raw rocks",
            d2,
        );
    }

    {
        let regions_path = objects_dir.join("forest-regions.json.gz");
        if !regions_path.exists() {
            g.gate(
                "F1",
                "forest regions present + rows schema-valid",
                vec!["objects/forest-regions.json.gz missing".into()],
            );
        } else {
            let doc = gunzip_json(&regions_path)?;
            let regions = doc["regions"].as_array().cloned().unwrap_or_default();
            let mut f1 = Vec::new();
            for (i, r) in regions.iter().enumerate() {
                if !v_region.is_valid(r) {
                    f1.push(format!("region[{i}] {}: invalid", r["id"]));
                }
            }
            g.gate(
                "F1",
                &format!(
                    "forest regions present + {} rows schema-valid",
                    regions.len()
                ),
                f1,
            );

            let mut f2 = Vec::new();
            let region_tree_sum: u64 = regions
                .iter()
                .map(|r| r["treeCount"].as_u64().unwrap_or(0))
                .sum();
            let inv_tree = inventory["byKind"]["tree"]["instances"]
                .as_u64()
                .unwrap_or(0);
            let inv_region = &inventory["byRegionKind"]["forest"];
            if inv_region.is_null() {
                f2.push("inventory.byRegionKind.forest missing".into());
            } else {
                if inv_region["treeCount"].as_u64() != Some(region_tree_sum) {
                    f2.push(format!(
                        "inventory forest.treeCount {} != regions file sum {region_tree_sum}",
                        inv_region["treeCount"]
                    ));
                }
                if inv_region["count"].as_u64() != Some(regions.len() as u64) {
                    f2.push(format!(
                        "inventory forest.count {} != regions {}",
                        inv_region["count"],
                        regions.len()
                    ));
                }
            }
            let unassigned = inventory["unassignedTrees"].as_u64().unwrap_or(0);
            if region_tree_sum + unassigned != inv_tree {
                f2.push(format!("F2 identity broken: {region_tree_sum} + {unassigned} != byKind.tree.instances {inv_tree}"));
            }
            if inv_tree != committed_kind_counts.get("tree").copied().unwrap_or(0) {
                f2.push(format!(
                    "inventory tree instances {inv_tree} != committed tree rows {}",
                    committed_kind_counts.get("tree").copied().unwrap_or(0)
                ));
            }
            g.gate(
                "F2",
                "forest.treeCount + unassignedTrees = byKind.tree.instances (exact)",
                f2,
            );

            let mut f6 = Vec::new();
            let trees: Vec<Tree> = tree_rows
                .iter()
                .map(|(x, y, class)| Tree {
                    x: *x,
                    y: *y,
                    class: class.clone(),
                })
                .collect();
            let redo = derive_forest_regions(&trees, world_size_m, terrain);
            if Value::Array(redo.regions.clone()) != Value::Array(regions.clone()) {
                f6.push("re-derived regions differ from committed rings/aggregates".into());
            }
            if redo.unassigned_trees != inventory["unassignedTrees"].as_u64().unwrap_or(u64::MAX) {
                f6.push(format!(
                    "re-derived unassignedTrees {} != inventory {}",
                    redo.unassigned_trees, inventory["unassignedTrees"]
                ));
            }
            let _ = forest::REGION_CELL_M;
            g.gate(
                "F6",
                "Path B derivation reproducible from committed chunk tree instances",
                f6,
            );
        }
    }
    Ok(())
}
