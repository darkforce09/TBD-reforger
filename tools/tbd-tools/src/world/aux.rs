//! T-165.8 — the export-lane auxiliaries: `validate-export-artifacts.mjs` (map-export-validate),
//! `census-types.mjs` (map-census), the T-090.3.0 spike gates (`verify-spike-k1`,
//! `census-spike`, `verify-spike-ops-log`), `copy-world-export-profile.mjs`, and
//! `raw-u16-to-dem-png.mjs` (T-091.0 DEM repack). Ports preserve stdout shapes + exit codes.

use std::collections::{HashMap, HashSet};
use std::io::BufRead as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use map_engine_core::dem::raw as dem_raw;
use map_engine_core::world::binary::chunk_container::TbdeHeader;
use serde_json::{Map, Value, json};

use super::build::CHUNK_SIZE_M;
use super::classify::{Classifier, Rules, load_rules};
use super::gates::{SchemaSet, gunzip_json};
use crate::geometry::cell_of;
use crate::serve::repo_root;
use crate::{density, forest};

/// `new Date(ms).toISOString()` — UTC with milliseconds, e.g. `2026-07-04T23:43:38.437Z`.
pub fn iso_from_system_time(t: std::time::SystemTime) -> String {
    let d = t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
    let (secs, millis) = (d.as_secs() as i64, d.subsec_millis());
    let days = secs.div_euclid(86400);
    let tod = secs.rem_euclid(86400);
    let (hh, mm, ss) = (tod / 3600, (tod % 3600) / 60, tod % 60);
    // civil-from-days (Howard Hinnant's algorithm)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d_ = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d_:02}T{hh:02}:{mm:02}:{ss:02}.{millis:03}Z")
}

fn classified_rows(rules: &Rules, path: &Path) -> Result<Vec<(Value, String, String, bool)>> {
    // classifyRawEntitiesJsonl semantics: collect-and-skip parse errors (spike-sized inputs).
    let mut classify = Classifier::new(rules);
    let text = std::fs::read_to_string(path)?;
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(row) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let cls = classify.classify(row["resourceName"].as_str().unwrap_or(""));
        out.push((row, cls.kind, cls.class, cls.matched));
    }
    Ok(out)
}

fn is_finite(v: &Value) -> bool {
    v.as_f64().is_some_and(f64::is_finite)
}

fn entry_is_k1_building(row: &Value, kind: &str) -> bool {
    kind == "building"
        && row["resourceName"].as_str().is_some_and(|s| !s.is_empty())
        && ["x", "y", "z", "yawDeg", "pitchDeg", "rollDeg"]
            .iter()
            .all(|k| is_finite(&row[*k]))
}

/* ─────────────────────────── verify-spike-k1 ─────────────────────────── */

pub fn verify_spike_k1(terrain: &str) -> Result<u8> {
    let raw = repo_root()
        .join("packages/map-assets")
        .join(terrain)
        .join("staging/spike/raw-entities.jsonl");
    if !raw.exists() {
        eprintln!(
            "verify-spike-k1: FAIL — raw-entities.jsonl not found: {}",
            raw.display()
        );
        return Ok(1);
    }
    let rules = load_rules()?;
    let entries = classified_rows(&rules, &raw)?;
    match entries
        .iter()
        .find(|(row, kind, _, _)| entry_is_k1_building(row, kind))
    {
        Some((row, _, _, _)) => {
            println!(
                "verify-spike-k1: PASS (K1) — building row: {}",
                row["resourceName"].as_str().unwrap_or("")
            );
            Ok(0)
        }
        None => {
            eprintln!(
                "verify-spike-k1: FAIL (K1) — no building-classified row with complete transform among {} rows",
                entries.len()
            );
            Ok(1)
        }
    }
}

/* ─────────────────────────── census-spike ─────────────────────────── */

/// T-278 — was a local 8-kind array missing T-244's `vehicle`; `census_spike` does
/// `by_kind.get_mut(kind).unwrap_or_else(|| panic!("kind {kind}"))`, so one wreck prefab inside
/// the spike region panicked the census. Single source now.
const ALL_KINDS: [&str; 9] = super::INSTANCE_KINDS;

pub fn census_spike(terrain: &str) -> Result<u8> {
    let staging = repo_root()
        .join("packages/map-assets")
        .join(terrain)
        .join("staging/spike");
    let raw = staging.join("raw-entities.jsonl");
    let out_path = staging.join("type-inventory-spike.json");
    if !raw.exists() {
        eprintln!(
            "census-spike: raw-entities.jsonl not found: {} — run the export plugin + copy-world-export-profile first",
            raw.display()
        );
        return Ok(1);
    }
    let rules = load_rules()?;
    let entries = classified_rows(&rules, &raw)?;

    let mut by_kind: HashMap<&str, (HashSet<String>, u64)> = ALL_KINDS
        .iter()
        .map(|k| (*k, (HashSet::new(), 0u64)))
        .collect();
    let mut all_prefabs: HashSet<String> = HashSet::new();
    let mut building_classes: Vec<(String, (HashSet<String>, u64))> = Vec::new();
    let mut building_idx: HashMap<String, usize> = HashMap::new();
    let mut unmatched: HashSet<String> = HashSet::new();
    for (row, kind, class, matched) in &entries {
        let rn = row["resourceName"].as_str().unwrap_or("").to_string();
        let bucket = by_kind
            .get_mut(kind.as_str())
            .unwrap_or_else(|| panic!("kind {kind}"));
        bucket.1 += 1;
        if !rn.is_empty() {
            bucket.0.insert(rn.clone());
            all_prefabs.insert(rn.clone());
        }
        if !matched && !rn.is_empty() {
            unmatched.insert(rn.clone());
        }
        if kind == "building" {
            let i = *building_idx.entry(class.clone()).or_insert_with(|| {
                building_classes.push((class.clone(), (HashSet::new(), 0)));
                building_classes.len() - 1
            });
            building_classes[i].1.1 += 1;
            if !rn.is_empty() {
                building_classes[i].1.0.insert(rn);
            }
        }
    }
    let mut by_kind_out = Map::new();
    for k in ALL_KINDS {
        let (prefabs, instances) = &by_kind[k];
        let mut m = Map::from_iter([
            ("prefabTypes".to_string(), json!(prefabs.len())),
            ("instances".to_string(), json!(instances)),
        ]);
        if k == "road" {
            m.insert("segments".into(), json!(0)); // spike does not extract road polylines
        }
        by_kind_out.insert(k.to_string(), Value::Object(m));
    }
    let mut by_building_class = Map::new();
    for (cls, (prefabs, instances)) in &building_classes {
        by_building_class.insert(
            cls.clone(),
            json!({ "prefabTypes": prefabs.len(), "instances": instances }),
        );
    }
    let total_instances = entries.len();
    // T-537: refuse writing an empty spike inventory over a prior census artifact.
    super::refuse_empty_write(
        "census-spike type-inventory",
        total_instances == 0,
        "zero classified instances — refusing empty type-inventory overwrite",
    )?;
    let generated_at = iso_from_system_time(std::fs::metadata(&raw)?.modified()?);
    let inventory = json!({
        "schemaVersion": "1.0.0",
        "terrainId": terrain,
        "censusStatus": "partial",
        "generatedAt": generated_at,
        "importPhaseMax": "spike_subregion",
        "sourceExportPath": "staging/spike/raw-entities.jsonl",
        "levels": { "uniquePrefabs": all_prefabs.len(), "totalInstances": total_instances },
        "byKind": by_kind_out,
        "byBuildingClass": by_building_class,
        "byRoadClass": {},
        "bySpeciesClass": {},
        "needsReview": { "prefabTypes": unmatched.len(), "prefabs": [] },
    });
    std::fs::write(&out_path, serde_json::to_string_pretty(&inventory)? + "\n")?;

    let mut failures: Vec<String> = Vec::new();
    let kind_sum: u64 = ALL_KINDS.iter().map(|k| by_kind[k].1).sum();
    if kind_sum != total_instances as u64 {
        failures.push(format!(
            "I1 kind sum {kind_sum} !== totalInstances {total_instances}"
        ));
    }
    let class_sum: u64 = building_classes.iter().map(|(_, (_, n))| n).sum();
    if class_sum != by_kind["building"].1 {
        failures.push(format!(
            "I2 byBuildingClass sum {class_sum} !== byKind.building.instances {}",
            by_kind["building"].1
        ));
    }
    let k1_pass = entries
        .iter()
        .any(|(row, kind, _, _)| entry_is_k1_building(row, kind));
    let k1b = by_kind["building"].1 >= 1;
    if k1_pass != k1b {
        failures.push(format!(
            "K1/K1b classify drift: verify-spike-k1={k1_pass} but byKind.building.instances>=1={k1b}"
        ));
    }
    if !failures.is_empty() {
        eprintln!(
            "census-spike: FAIL ({}) — wrote {}",
            failures.len(),
            out_path.display()
        );
        for f in &failures {
            eprintln!("  {f}");
        }
        return Ok(1);
    }
    println!(
        "census-spike: OK (K1b) — {total_instances} instances, {} prefabs; building={}, tree={}, road={}, needsReview={} → {}",
        all_prefabs.len(),
        by_kind["building"].1,
        by_kind["tree"].1,
        by_kind["road"].1,
        unmatched.len(),
        out_path.display()
    );
    Ok(0)
}

/* ─────────────────────────── verify-spike-ops-log ─────────────────────────── */

pub fn verify_spike_ops_log(terrain: &str) -> Result<u8> {
    let root = repo_root();
    let ops_path = root
        .join(".ai/artifacts")
        .join(format!("map_export_{terrain}.json"));
    let staging = root
        .join("packages/map-assets")
        .join(terrain)
        .join("staging/spike");
    let raw_path = staging.join("raw-entities.jsonl");
    if !ops_path.exists() {
        eprintln!(
            "verify-spike-ops-log: FAIL — ops log not found: {}",
            ops_path.display()
        );
        return Ok(1);
    }
    let ops: Value = match serde_json::from_str(&std::fs::read_to_string(&ops_path)?) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("verify-spike-ops-log: FAIL — ops log is not valid JSON: {e}");
            return Ok(1);
        }
    };
    let mut fail: Vec<String> = Vec::new();
    let is_str = |v: &Value| v.as_str().is_some_and(|s| !s.is_empty());
    let size_gt0 =
        |p: &Path| p.exists() && std::fs::metadata(p).map(|m| m.len() > 0).unwrap_or(false);
    let resolve_artifact = |p: &Value| -> Option<PathBuf> {
        let s = p.as_str()?;
        if s.is_empty() {
            return None;
        }
        let cands = [
            if Path::new(s).is_absolute() {
                Some(PathBuf::from(s))
            } else {
                None
            },
            Some(root.join(s)),
            Some(staging.join(s)),
        ];
        for c in cands.into_iter().flatten() {
            if c.exists() {
                return Some(c);
            }
        }
        Some(if Path::new(s).is_absolute() {
            PathBuf::from(s)
        } else {
            root.join(s)
        })
    };

    for k in [
        "schemaVersion",
        "terrainId",
        "slice",
        "generatedAt",
        "subregionBBoxM",
        "probes",
        "gates",
        "handednessRemap",
        "forestSource",
        "tileFindings",
        "sampleRows",
        "mcpToolsUsed",
    ] {
        if ops.get(k).is_none() {
            fail.push(format!("missing required key: {k}"));
        }
    }
    if ops.get("terrainId").is_some() && ops["terrainId"] != terrain {
        fail.push(format!("terrainId {} !== {terrain}", ops["terrainId"]));
    }
    if ops.get("slice").is_some() && ops["slice"] != "T-090.3.0" {
        fail.push(format!("slice {} !== T-090.3.0", ops["slice"]));
    }
    if ops.get("subregionBBoxM").is_some()
        && !(ops["subregionBBoxM"]
            .as_array()
            .is_some_and(|a| a.len() == 4 && a.iter().all(is_finite)))
    {
        fail.push("subregionBBoxM must be [minX,minY,maxX,maxY] finite numbers".into());
    }

    let gates = &ops["gates"];
    for g in ["K1", "K1b", "K2", "K3", "K4", "K5", "K6", "K7"] {
        if gates[g] != "pass" && gates[g] != "fail" {
            fail.push(format!(
                "gates.{g} must be \"pass\" or \"fail\" (lowercase), got {}",
                gates[g]
            ));
        }
    }
    let is_pass = |g: &str| gates[g] == "pass";

    if is_pass("K6") {
        let h = &ops["handednessRemap"];
        if !is_str(&h["enfusionBasis"]) {
            fail.push("K6 pass requires handednessRemap.enfusionBasis non-empty".into());
        }
        if !is_str(&h["editorToExport"]) {
            fail.push("K6 pass requires handednessRemap.editorToExport non-empty".into());
        }
        if h["sampleEntity"]
            .as_object()
            .is_none_or(serde_json::Map::is_empty)
        {
            fail.push("K6 pass requires handednessRemap.sampleEntity non-empty".into());
        }
    }
    if is_pass("K5") {
        if ops["forestSource"] != "engine-mask" && ops["forestSource"] != "derived-hull-mandated" {
            fail.push(format!(
                "K5 pass requires forestSource ∈ {{engine-mask, derived-hull-mandated}}, got {}",
                ops["forestSource"]
            ));
        }
        let ev = if !ops["probes"]["S5"]["evidence"].is_null() {
            &ops["probes"]["S5"]["evidence"]
        } else {
            &ops["probes"]["S5"]["note"]
        };
        if !is_str(ev) {
            fail.push("K5 pass requires probes.S5.evidence (MCP citation) non-empty".into());
        }
    }

    let rules = load_rules()?;
    let raw_entries = if raw_path.exists() {
        Some(classified_rows(&rules, &raw_path)?)
    } else {
        None
    };
    let sample_rows = ops["sampleRows"].as_array().cloned().unwrap_or_default();
    if !ops["sampleRows"].is_array() || sample_rows.len() != 3 {
        fail.push(format!(
            "sampleRows must be exactly 3 (got {})",
            if ops["sampleRows"].is_array() {
                sample_rows.len().to_string()
            } else {
                "non-array".into()
            }
        ));
    }
    match &raw_entries {
        Some(entries) => {
            for (i, s) in sample_rows.iter().enumerate() {
                if !is_str(&s["resourceName"]) || !["x", "y", "z"].iter().all(|k| is_finite(&s[*k]))
                {
                    fail.push(format!("sampleRows[{i}] needs resourceName + finite x,y,z"));
                    continue;
                }
                let hit = entries.iter().any(|(r, _, _, _)| {
                    r["resourceName"] == s["resourceName"]
                        && (r["x"].as_f64().unwrap_or(f64::MAX) - s["x"].as_f64().unwrap()).abs()
                            <= 0.001
                        && (r["y"].as_f64().unwrap_or(f64::MAX) - s["y"].as_f64().unwrap()).abs()
                            <= 0.001
                        && (r["z"].as_f64().unwrap_or(f64::MAX) - s["z"].as_f64().unwrap()).abs()
                            <= 0.001
                });
                if !hit {
                    fail.push(format!(
                        "sampleRows[{i}] ({}) does not resolve to any raw-entities.jsonl line within 0.001 m",
                        s["resourceName"].as_str().unwrap_or("")
                    ));
                }
            }
        }
        None if !sample_rows.is_empty() => {
            fail.push(format!(
                "cannot resolve sampleRows — raw-entities.jsonl missing: {}",
                raw_path.display()
            ));
        }
        None => {}
    }

    if is_pass("K2") {
        let real_obb = raw_entries.as_ref().is_some_and(|entries| {
            entries.iter().any(|(r, kind, _, _)| {
                kind == "building"
                    && r["halfExtentsM"]
                        .as_array()
                        .is_some_and(|a| a.len() == 3 && a.iter().all(is_finite))
            })
        });
        let kind_default = ops["probes"]["S2"]["obbDecision"] == "kind-default"
            && is_str(&ops["probes"]["S2"]["mcpEvidence"]);
        if !real_obb && !kind_default {
            fail.push("K2 pass requires a building row with numeric halfExtentsM[3] OR probes.S2.obbDecision==='kind-default' + probes.S2.mcpEvidence".into());
        }
    }

    let sat = &ops["tileFindings"]["satellite"];
    let sat_file = resolve_artifact(&sat["path"]);
    if is_pass("K3") {
        if !is_str(&sat["path"]) || !sat_file.as_deref().is_some_and(size_gt0) {
            fail.push(format!(
                "K3 pass requires tileFindings.satellite.path to be a >0-byte file (looked at {})",
                sat_file
                    .map(|p| p.display().to_string())
                    .unwrap_or_default()
            ));
        }
    } else if gates["K3"] == "fail" {
        if is_str(&sat["path"]) && sat_file.as_deref().is_some_and(size_gt0) {
            fail.push("K3 fail but a satellite tile file is present — inconsistent".into());
        } else if !(sat["escalate"] == true && is_str(&sat["evidence"])) {
            fail.push("K3 fail requires no tile OR tileFindings.satellite.escalate===true with non-empty evidence".into());
        }
    }

    let map_t = &ops["tileFindings"]["map"];
    let map_file = resolve_artifact(&map_t["path"]);
    let n9 = "synthesized-cartographic required";
    let has_n9 = map_t["synthesizedCartographicRequired"] == true
        && (map_t["note"].as_str().unwrap_or("").contains(n9)
            || ops["probes"]["S4"]["note"]
                .as_str()
                .unwrap_or("")
                .contains(n9));
    if is_pass("K4") {
        let has_tile = is_str(&map_t["path"]) && map_file.as_deref().is_some_and(size_gt0);
        if !has_tile && !has_n9 {
            fail.push(format!("K4 pass requires a >0-byte map tile OR synthesizedCartographicRequired + literal \"{n9}\" note"));
        }
    } else if gates["K4"] == "fail"
        && is_str(&map_t["path"])
        && map_file.as_deref().is_some_and(size_gt0)
    {
        fail.push("K4 fail but a map tile file is present — inconsistent".into());
    }

    if !fail.is_empty() {
        eprintln!("verify-spike-ops-log: FAIL ({})", fail.len());
        for f in &fail {
            eprintln!("  {f}");
        }
        return Ok(1);
    }
    println!("verify-spike-ops-log: OK (K7 + K2/K3/K4 gate↔artifact)");
    Ok(0)
}

/* ─────────────────────────── census-types (map-census) ─────────────────────────── */

fn spawn_type_inventory_gate() -> Result<bool> {
    // The I-gates live in `xtask schema type-inventory` (T-165.1) — the Rust replacement for
    // the spawned verify-type-inventory.mjs.
    let status = std::process::Command::new("cargo")
        .args(["run", "-q", "-p", "xtask", "--", "schema", "type-inventory"])
        .current_dir(repo_root())
        .status()?;
    Ok(status.success())
}

pub fn census_types(terrain: &str) -> Result<u8> {
    let root = repo_root();
    let inventory_path = root
        .join("packages/map-assets")
        .join(terrain)
        .join("objects/type-inventory.json");
    if !inventory_path.exists() {
        eprintln!("map-census: missing {}", inventory_path.display());
        return Ok(1);
    }
    if !spawn_type_inventory_gate()? {
        return Ok(1);
    }
    let inv: Value = serde_json::from_str(&std::fs::read_to_string(&inventory_path)?)?;
    let full = root
        .join("packages/map-assets")
        .join(terrain)
        .join("staging/export/raw-entities.jsonl");
    let spike = root
        .join("packages/map-assets")
        .join(terrain)
        .join("staging/spike/raw-entities.jsonl");
    if inv["censusStatus"] == "pending_export" {
        if full.exists() {
            eprintln!(
                "map-census: full-map export exists but censusStatus is still pending_export — run full classify + census implementation (T-090.2/.3)"
            );
            return Ok(1);
        }
        if spike.exists() {
            println!(
                "map-census: {terrain} censusStatus=pending_export — T-090.3.0 spike subregion export present (staging/spike); full-map census still pending (expected)"
            );
            return Ok(0);
        }
        println!(
            "map-census: {terrain} censusStatus=pending_export — exact counts unknown until Workbench export + classify (see t090_world_object_type_inventory.md)"
        );
        return Ok(0);
    }
    println!(
        "map-census: {terrain} censusStatus={} — validation only (compute path T-090.2/.3)",
        inv["censusStatus"].as_str().unwrap_or("")
    );
    std::fs::write(
        root.join(".ai/artifacts")
            .join(format!("type_inventory_{terrain}.json")),
        serde_json::to_string_pretty(&inv)? + "\n",
    )?;
    Ok(0)
}

/* ─────────────────────────── validate-export-artifacts (map-export-validate) ─────────────────────────── */

pub fn validate_export_artifacts() -> Result<u8> {
    let root = repo_root();
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

    let registry: Value = serde_json::from_str(&std::fs::read_to_string(
        root.join("packages/map-assets/terrain-registry.json"),
    )?)?;
    if !v_registry.is_valid(&registry) {
        failures += 1;
        fail("terrain-registry.json schema: invalid".into());
    } else {
        pass("terrain-registry.json schema valid".into());
    }

    let terrains = registry["terrains"].as_array().cloned().unwrap_or_default();
    for t in &terrains {
        let tid = t["terrainId"].as_str().unwrap_or("");
        let terrain_dir = root.join("packages/map-assets").join(tid);
        let manifest_path = root
            .join("packages/map-assets")
            .join(t["manifestPath"].as_str().unwrap_or(""));
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
                    let Ok(dec) = map_engine_core::geometry::tbdd::decode_tbdd(&buf) else {
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

    // Inventory gates (I1-I7 subset) — delegate to the Rust xtask gate (was verify-type-inventory.mjs).
    // Output captured (the Node script spawned with stdio:pipe) — surfaced only on failure.
    let inv_gate = std::process::Command::new("cargo")
        .args(["run", "-q", "-p", "xtask", "--", "schema", "type-inventory"])
        .current_dir(repo_root())
        .output()?;
    if inv_gate.status.success() {
        pass("verify-type-inventory (I-gates) OK".into());
    } else {
        failures += 1;
        fail(format!(
            "verify-type-inventory: {} {}",
            String::from_utf8_lossy(&inv_gate.stdout).trim(),
            String::from_utf8_lossy(&inv_gate.stderr).trim()
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
        !root
            .join("packages/map-assets")
            .join(t["terrainId"].as_str().unwrap_or(""))
            .join("staging/export/raw-entities.jsonl")
            .exists()
    });
    match other {
        Some(t) => {
            let tid = t["terrainId"].as_str().unwrap_or("");
            // T-869: export-terrain.sh → `cargo run -q -p xtask -- map export-terrain …`
            // (inherits CARGO_TARGET_DIR when set — same pin as Makefile / checkrun gates).
            let status = std::process::Command::new("cargo")
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
                .current_dir(repo_root())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .status()?;
            if status.code() == Some(2) {
                pass(format!(
                    "E2b: xtask map export-terrain {tid} -> exit 2 (operator-instructions branch, same code path)"
                ));
            } else {
                failures += 1;
                fail(format!(
                    "E2b: xtask map export-terrain {tid} expected exit 2, got {:?}",
                    status.code()
                ));
            }
        }
        None => pass("E2b: every terrain already staged — branch untestable (OK)".into()),
    }

    {
        // E2c: terrain ids must flow from argv/registry — no literal id in the pipeline sources.
        // T-165.8 / T-869: the pipeline is Rust; scanned set is the Rust modules + the xtask
        // orchestrator (topo.rs is excluded like decode-topo.mjs was — its per-terrain CONFIG
        // TABLE is the sanctioned place for ids).
        let sources = [
            "tools/tbd-tools/src/world/build.rs",
            "tools/tbd-tools/src/world/gates.rs",
            "tools/tbd-tools/src/world/aux.rs",
            "xtask/src/gate_export_terrain.rs",
            "tools/tbd-tools/src/geometry.rs",
            "tools/tbd-tools/src/density.rs",
            "tools/tbd-tools/src/forest.rs",
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

/* ─────────────────────────── copy-world-export-profile ─────────────────────────── */

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
    let dest_dir = root
        .join("packages/map-assets")
        .join(terrain)
        .join("staging")
        .join(if full { "export" } else { "spike" });
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
    // T-537: count source FIRST — never overwrite a staged export with an empty jsonl.
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
        // T-090.12.1b — provenance: when the plugin wrote no `workbenchVersion`, stamp the Steam
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
        // T-537: refuse lossy synth overwrite of an existing real/prior meta.
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

/* ─────────────────────────── raw-u16-to-dem-png (T-091.0) ─────────────────────────── */

/// The `buildid` of Arma Reforger Tools from the Steam app manifest that owns `path`
/// (`…/steamapps/appmanifest_1874910.acf`), `None` outside a Steam library.
fn steam_build_id(path: &Path) -> Option<String> {
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

/// The plugin's fixed V4 encoding range (`TBD_MapExportDEM.c` `DEFAULT_HMIN`/`DEFAULT_HMAX`), used
/// when a meta file predates the `heightRange*` keys. Everon's shipped manifest carries exactly
/// these, so the `.dem` a re-export writes stays comparable with the committed PNG.
const DEM_DEFAULT_MIN_M: f64 = -204.78;
const DEM_DEFAULT_MAX_M: f64 = 375.53;

/// T-935.4 — write `dem/elevation.dem`: a `TBDE` header (spec §3.2) then `width * height` `u16`
/// samples little-endian, row-major, row 0 = north edge.
///
/// The samples are written **verbatim** — the same quantised values the 16-bit PNG carries — so the
/// two files decode to the identical grid and only the container differs. `min_m` / `max_m` are the
/// encoding range the plugin quantised against; the header stores them as `scale_m =
/// (max - min) / 65535` and `offset_m = min`, both `f32` (spec §3.2), which is where the only
/// precision difference from the manifest's `f64` height range comes from.
///
/// # Errors
/// When the grid does not match `width * height`, when those overflow `usize`, or on write failure.
pub fn write_elevation_dem(
    path: &Path,
    width: u32,
    height: u32,
    min_m: f64,
    max_m: f64,
    samples: &[u16],
) -> Result<()> {
    // `as f32` here and nowhere else: the format stores the range in f32, so the narrowing is
    // named at the boundary instead of hiding inside the header constructor's caller.
    #[allow(clippy::cast_possible_truncation)]
    let header = TbdeHeader::new(width, height, min_m as f32, max_m as f32);
    let expected = header
        .sample_count()
        .with_context(|| format!("TBDE {width}x{height} overflows usize"))?;
    anyhow::ensure!(
        samples.len() == expected,
        "TBDE {width}x{height} needs {expected} samples, got {}",
        samples.len()
    );
    let bytes = dem_raw::to_bytes(&header, samples);
    std::fs::write(path, &bytes).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

pub fn raw_u16_to_dem_png(raster_path: &Path, meta_path: &Path, out_path: &Path) -> Result<u8> {
    let meta: Value = serde_json::from_str(&std::fs::read_to_string(meta_path)?)?;
    let (w, h) = (
        meta["widthPx"].as_u64().unwrap_or(0) as usize,
        meta["heightPx"].as_u64().unwrap_or(0) as usize,
    );
    if w == 0 || h == 0 {
        eprintln!("Bad meta dims {w}x{h}");
        return Ok(1);
    }
    println!("Parsing raster {} ({w}x{h})...", raster_path.display());
    let buf = std::fs::read(raster_path)?;
    let mut raster = vec![0u16; w * h];
    let mut idx = 0usize;
    let mut cur = 0u32;
    let mut in_num = false;
    for &c in &buf {
        if c.is_ascii_digit() {
            cur = cur * 10 + u32::from(c - b'0');
            in_num = true;
        } else if in_num {
            raster[idx] = cur as u16;
            idx += 1;
            cur = 0;
            in_num = false;
        }
    }
    if in_num {
        raster[idx] = cur as u16;
        idx += 1;
    }
    if idx != w * h {
        eprintln!("FAIL parsed {idx} values, expected {}", w * h);
        return Ok(1);
    }
    let (u_min, u_max) = raster
        .iter()
        .fold((65535u16, 0u16), |(lo, hi), &v| (lo.min(v), hi.max(v)));
    println!("Parsed {idx} samples; u16 range [{u_min}, {u_max}]");

    println!("Deflating IDAT...");
    let mut be = Vec::with_capacity(w * h * 2);
    for v in &raster {
        be.extend_from_slice(&v.to_be_bytes());
    }
    {
        let file = std::fs::File::create(out_path)?;
        let wtr = std::io::BufWriter::new(file);
        let mut enc = png::Encoder::new(wtr, w as u32, h as u32);
        enc.set_color(png::ColorType::Grayscale);
        enc.set_depth(png::BitDepth::Sixteen);
        enc.set_compression(png::Compression::Best);
        enc.write_header()?.write_image_data(&be)?;
    }
    let png_len = std::fs::metadata(out_path)?.len();
    println!("Wrote {} ({png_len} bytes)", out_path.display());

    // Self-check: decode back, verify IHDR + 3 round-trip pixels.
    let dec = png::Decoder::new(std::fs::File::open(out_path)?);
    let mut reader = dec.read_info().context("png read_info")?;
    let info = reader.info();
    if info.bit_depth != png::BitDepth::Sixteen
        || info.color_type != png::ColorType::Grayscale
        || info.width as usize != w
        || info.height as usize != h
    {
        eprintln!(
            "FAIL IHDR check: depth={:?} colorType={:?} {}x{}",
            info.bit_depth, info.color_type, info.width, info.height
        );
        return Ok(1);
    }
    let mut data = vec![0u8; reader.output_buffer_size()];
    reader.next_frame(&mut data)?;
    for (x, y) in [(0usize, 0usize), (w - 1, h - 1), (w >> 1, h >> 1)] {
        let off = (y * w + x) * 2;
        let got = u16::from_be_bytes([data[off], data[off + 1]]);
        let want = raster[y * w + x];
        if got != want {
            eprintln!("FAIL round-trip ({x},{y}): got {got} want {want}");
            return Ok(1);
        }
    }
    println!("OK  IHDR bitDepth=16 colorType=0 dims match; round-trip pixels OK");

    // T-935.4 dual emission (spec §7 wave 2): the same `raster` also goes out as
    // `dem/elevation.dem` beside the PNG. Both are written every run until T-935.13 flips the
    // manifest — the loader picks by `manifest.dem.raw`, so deleting either write before then
    // blinds one reader. Nothing above this line changed.
    let dem_path = out_path.with_file_name("elevation.dem");
    let (min_m, max_m) = (
        meta["heightRangeMinM"]
            .as_f64()
            .unwrap_or(DEM_DEFAULT_MIN_M),
        meta["heightRangeMaxM"]
            .as_f64()
            .unwrap_or(DEM_DEFAULT_MAX_M),
    );
    write_elevation_dem(&dem_path, w as u32, h as u32, min_m, max_m, &raster)?;
    let dem_len = std::fs::metadata(&dem_path)?.len();
    println!(
        "Wrote {} ({dem_len} bytes; TBDE {w}x{h} u16 LE, range [{min_m}, {max_m}] m)",
        dem_path.display()
    );
    Ok(0)
}

/* ─────────────────────────── export-terrain phase gate ─────────────────────────── */

/// The export-terrain.sh phase gate (was an inline `node - <<EOF` heredoc): the requested
/// phase must not exceed the registry's importPhaseMax (phased-import rule).
pub fn phase_gate(terrain: &str, phase: &str) -> Result<u8> {
    const ORDER: [&str; 10] = [
        "P1_buildings",
        "P2_trees",
        "P3_vegetation",
        "P4_rocks",
        "P5_props",
        "P6_roads_highway",
        "P7_roads_paved",
        "P8_roads_dirt",
        "P9_roads_path",
        "P10_full",
    ];
    let reg: Value = serde_json::from_str(&std::fs::read_to_string(
        repo_root().join("packages/map-assets/terrain-registry.json"),
    )?)?;
    let Some(row) = reg["terrains"]
        .as_array()
        .and_then(|a| a.iter().find(|t| t["terrainId"] == terrain))
    else {
        eprintln!("export-terrain: terrain '{terrain}' not in terrain-registry.json");
        return Ok(1);
    };
    let Some(p_idx) = ORDER.iter().position(|p| *p == phase) else {
        eprintln!("export-terrain: unknown phase '{phase}'");
        return Ok(1);
    };
    let max = row["importPhaseMax"].as_str().unwrap_or("");
    let max_idx = ORDER.iter().position(|p| *p == max);
    if max_idx.is_none() || p_idx > max_idx.unwrap() {
        eprintln!(
            "export-terrain: phase {phase} blocked — registry importPhaseMax={} (advance only after map-verify-phase PASS + registry bump)",
            if max.is_empty() { "(none)" } else { max }
        );
        return Ok(1);
    }
    Ok(0)
}

/* ─────────────────────────── catalog-sap-cells (T-090.1.2) ─────────────────────────── */

/// Enumerate Everon SAP supertexture cells → staging/sap/cell-catalog.json (fast index; the
/// full decode + fail-fast lives in the stitch step).
pub fn catalog_sap_cells(terrain: &str) -> Result<u8> {
    use super::edds::{CELL_COUNT, CELL_M, CELL_PX, GRID, WORLD_M, cell_grid, cell_path};
    use super::pak::PakVfs;
    if terrain != "everon" {
        // E2c-allow: the SAP lane is Eden-only this slice (matches the .mjs guard)
        eprintln!("only everon supported this slice (got {terrain})"); // E2c-allow
        return Ok(1);
    }
    let out_dir = repo_root().join("packages/map-assets/everon/staging/sap"); // E2c-allow
    let vfs = PakVfs::open_default()?;
    let cells = super::edds::list_eden_cells(&vfs);
    if cells.len() as u32 != CELL_COUNT {
        eprintln!(
            "FAIL: found {} Eden cells, expected {CELL_COUNT}",
            cells.len()
        );
        return Ok(1);
    }
    let entries: Vec<Value> = cells
        .iter()
        .map(|(n, _)| {
            let (gx, gy) = cell_grid(*n);
            json!({
                "id": n,
                "eddsPath": cell_path(*n),
                "gridX": gx,
                "gridY": gy,
                "widthPx": CELL_PX,
                "heightPx": CELL_PX,
                "worldMinX": gx * CELL_M,
                "worldMinZ": gy * CELL_M,
                "pixelX": gx * CELL_PX,
                "pixelY": (GRID - 1 - gy) * CELL_PX,
            })
        })
        .collect();
    let generated_at = {
        let full = iso_from_system_time(std::time::SystemTime::now());
        // toISOString().replace(/\.\d+Z$/, "Z") — seconds precision.
        format!("{}Z", &full[..19])
    };
    let catalog = json!({
        "terrain": terrain,
        "slice": "T-090.1.2",
        "generatedAt": generated_at,
        "grid": GRID,
        "cellCount": entries.len(),
        "cellMeters": CELL_M,
        "cellPx": CELL_PX,
        "metersPerPixel": super::jsval::js_num(f64::from(CELL_M) / f64::from(CELL_PX)),
        "worldBounds": [0, 0, WORLD_M, WORLD_M],
        "orthoPx": [GRID * CELL_PX, GRID * CELL_PX],
        "gridMapping": "row-major N=y*50+x; cell gridY=0 = world Z=0 (south); ortho north-up (south at image bottom, pixelY=(49-gridY)*256)",
        "source": "sap-supertexture-stitch",
        "cells": entries,
    });
    // T-537: refuse an empty cell catalog overwrite.
    super::refuse_empty_write(
        "catalog-sap-cells",
        catalog["cellCount"].as_u64() != Some(u64::from(CELL_COUNT))
            || catalog["cells"].as_array().map(|a| a.len()).unwrap_or(0) == 0,
        &format!(
            "cellCount {} != expected {CELL_COUNT} — refusing empty/partial cell-catalog.json overwrite",
            catalog["cellCount"]
        ),
    )?;
    std::fs::create_dir_all(&out_dir)?;
    let out = out_dir.join("cell-catalog.json");
    std::fs::write(&out, serde_json::to_string_pretty(&catalog)? + "\n")?;
    println!(
        "wrote {} ({} cells, ortho {}x{})",
        out.display(),
        catalog["cellCount"],
        GRID * CELL_PX,
        GRID * CELL_PX
    );
    Ok(0)
}

#[cfg(test)]
mod elevation_dem_tests {
    use std::path::PathBuf;

    use map_engine_core::dem::png_decode::decode_png_gray16;
    use map_engine_core::dem::raw::RawDem;
    use map_engine_core::world::binary::chunk_container::HEADER_BYTES;

    use super::*;

    /// A distinct, non-square grid: a width/height swap anywhere in the emit or the read is a
    /// different file, and both `u16` endpoints are present.
    const W: u32 = 4;
    const H: u32 = 3;
    fn grid() -> Vec<u16> {
        vec![
            0, 1, 65535, 32768, 40000, 7, 60000, 100, 12345, 2, 511, 65534,
        ]
    }

    fn tmpdir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("t935-4-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("tempdir");
        dir
    }

    /// The emitter's frame: 32-byte header, then exactly `2 * width * height` bytes, and the file
    /// reads back through the loader's own parser as the samples that went in.
    #[test]
    fn write_elevation_dem_frames_the_grid_and_round_trips() {
        let dir = tmpdir("frame");
        let p = dir.join("elevation.dem");
        let s = grid();
        write_elevation_dem(&p, W, H, -204.78, 375.53, &s).expect("write");

        let bytes = std::fs::read(&p).expect("read");
        assert_eq!(
            bytes.len(),
            HEADER_BYTES + 2 * (W as usize) * (H as usize),
            "file length must be 32 + 2 x width x height"
        );
        assert_eq!(&bytes[..4], b"TBDE");
        // The wire layout, byte for byte: version, flags, width, height, then sample 0 LE.
        assert_eq!(&bytes[4..6], &1_u16.to_le_bytes());
        assert_eq!(&bytes[6..8], &0_u16.to_le_bytes());
        assert_eq!(&bytes[8..12], &W.to_le_bytes());
        assert_eq!(&bytes[12..16], &H.to_le_bytes());
        assert_eq!(&bytes[HEADER_BYTES..HEADER_BYTES + 2], &s[0].to_le_bytes());
        // …and sample 2 (65535) proves the payload is LE, not BE.
        assert_eq!(
            &bytes[HEADER_BYTES + 4..HEADER_BYTES + 6],
            &65535_u16.to_le_bytes()
        );

        let dem = RawDem::parse(&bytes).expect("parse");
        assert_eq!((dem.width(), dem.height()), (W, H));
        assert_eq!(dem.samples, s);
        #[allow(clippy::cast_possible_truncation)]
        let want_scale = ((375.53_f64 - -204.78_f64) as f32) / 65535.0;
        assert_eq!(dem.header.offset_m, -204.78_f64 as f32);
        assert!(
            (dem.header.scale_m - want_scale).abs() <= f32::EPSILON,
            "{} vs {want_scale}",
            dem.header.scale_m
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A sample count that disagrees with the header dims is refused, and nothing is written — the
    /// alternative is a file whose header lies about its own payload.
    #[test]
    fn write_elevation_dem_refuses_a_grid_that_is_not_width_times_height() {
        let dir = tmpdir("mismatch");
        let p = dir.join("elevation.dem");
        let err = write_elevation_dem(&p, W, H, 0.0, 1.0, &grid()[..11]).expect_err("must refuse");
        assert!(
            format!("{err:#}").contains("needs 12 samples, got 11"),
            "{err:#}"
        );
        assert!(!p.exists(), "nothing may be written for a rejected grid");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **The dual-emission acceptance test.** One `raw_u16_to_dem_png` run over a synthetic ASCII
    /// raster must write both files, and the `.dem` must decode to exactly the grid the `.png`
    /// decodes to — through the two independent decoders, not through the emitter's own memory.
    #[test]
    fn raw_u16_to_dem_png_also_emits_elevation_dem_with_the_same_grid() {
        let dir = tmpdir("dual");
        let s = grid();
        let raster = dir.join("heightmap.txt");
        std::fs::write(
            &raster,
            s.chunks(W as usize)
                .map(|row| row.iter().map(u16::to_string).collect::<Vec<_>>().join(" "))
                .collect::<Vec<_>>()
                .join("\n")
                + "\n",
        )
        .expect("raster");
        let meta = dir.join("meta.json");
        std::fs::write(
            &meta,
            serde_json::to_string(&json!({
                "widthPx": W, "heightPx": H,
                "heightRangeMinM": -204.78, "heightRangeMaxM": 375.53,
            }))
            .expect("meta json"),
        )
        .expect("meta");
        let png = dir.join("everon-dem-16bit.png");

        assert_eq!(
            raw_u16_to_dem_png(&raster, &meta, &png).expect("convert"),
            0,
            "the converter must succeed"
        );

        let dem_path = dir.join("elevation.dem");
        assert!(png.exists(), "the PNG emit must be untouched");
        assert!(dem_path.exists(), "elevation.dem must be written beside it");

        let (png_raster, pw, ph) =
            decode_png_gray16(&std::fs::read(&png).expect("read png")).expect("png decode");
        let dem = RawDem::parse(&std::fs::read(&dem_path).expect("read dem")).expect("dem parse");
        assert_eq!((dem.width(), dem.height()), (pw, ph), "dims must agree");
        assert_eq!(dem.samples, png_raster, "the two files must carry one grid");
        assert_eq!(dem.samples, s, "…and it must be the raster's own values");
        // The header carries the meta's range, so a reader needs no manifest to get metres.
        assert_eq!(dem.header.offset_m, -204.78_f64 as f32);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A meta without the `heightRange*` keys falls back to the plugin's fixed V4 range rather
    /// than quantising against 0..0 and flattening the terrain.
    #[test]
    fn elevation_dem_falls_back_to_the_v4_range_when_meta_omits_it() {
        let dir = tmpdir("v4");
        let s = grid();
        let raster = dir.join("heightmap.txt");
        std::fs::write(
            &raster,
            s.iter().map(u16::to_string).collect::<Vec<_>>().join(" "),
        )
        .expect("raster");
        let meta = dir.join("meta.json");
        std::fs::write(
            &meta,
            serde_json::to_string(&json!({ "widthPx": W, "heightPx": H })).expect("meta json"),
        )
        .expect("meta");
        let png = dir.join("d.png");
        assert_eq!(
            raw_u16_to_dem_png(&raster, &meta, &png).expect("convert"),
            0
        );
        let dem = RawDem::parse(&std::fs::read(dir.join("elevation.dem")).expect("read"))
            .expect("dem parse");
        assert_eq!(dem.header.offset_m, DEM_DEFAULT_MIN_M as f32);
        assert_eq!(
            dem.header.scale_m,
            ((DEM_DEFAULT_MAX_M - DEM_DEFAULT_MIN_M) as f32) / 65535.0
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// **The everon spot check.** Decodes the committed 6400x6400 DEM PNG, emits the `.dem` from
    /// its raster and re-reads it, asserting all 40,960,000 samples are identical and metres agree
    /// to within the `f32` rounding of the header's scale/offset.
    ///
    /// `#[ignore]` because `packages/map-assets/**/*.png` is git-LFS: in a slice worktree the file
    /// on disk is a 133-byte pointer, and a test that reads one would be red for a reason that has
    /// nothing to do with this code. Run it where the payload is hydrated:
    /// `cargo test -p tbd-tools elevation_dem -- --ignored --nocapture`.
    #[test]
    #[ignore = "needs the git-lfs everon DEM payload; run explicitly with -- --ignored"]
    fn everon_elevation_dem_matches_the_shipped_png() {
        use map_engine_core::dem::sample::uint16_to_meters;

        let root = repo_root();
        let png = root.join("packages/map-assets/everon/dem/everon-dem-16bit.png");
        let bytes = std::fs::read(&png).unwrap_or_else(|e| panic!("{}: {e}", png.display()));
        assert!(
            bytes.len() > 1_000_000,
            "{} is {} bytes — this is the git-lfs pointer, not the DEM; \
             `git lfs pull --include {}` first",
            png.display(),
            bytes.len(),
            png.display()
        );
        let (raster, w, h) = decode_png_gray16(&bytes).expect("png decode");
        assert_eq!((w, h), (6400, 6400), "everon DEM dims");

        let manifest: Value = serde_json::from_str(
            &std::fs::read_to_string(root.join("packages/map-assets/everon/manifest.json"))
                .expect("manifest"),
        )
        .expect("manifest json");
        let min_m = manifest["dem"]["heightRangeMinM"].as_f64().expect("min");
        let max_m = manifest["dem"]["heightRangeMaxM"].as_f64().expect("max");

        let dir = tmpdir("everon");
        let p = dir.join("elevation.dem");
        write_elevation_dem(&p, w, h, min_m, max_m, &raster).expect("write");
        let on_disk = std::fs::read(&p).expect("read");
        assert_eq!(
            on_disk.len(),
            HEADER_BYTES + 2 * 6400 * 6400,
            "everon .dem is 32 + 81,920,000 bytes"
        );
        let dem = RawDem::parse(&on_disk).expect("parse");
        assert_eq!(
            dem.samples, raster,
            "all 40,960,000 samples must be identical"
        );

        let mut worst = 0.0_f64;
        for y in 0..h {
            for x in 0..w {
                let v = raster[(y * w + x) as usize];
                let png_m = uint16_to_meters(f64::from(v), min_m, max_m) as f32;
                let raw_m = dem.metres(x, y).expect("in range");
                worst = worst.max((f64::from(raw_m) - f64::from(png_m)).abs());
            }
        }
        println!(
            "everon spot check: {}x{} samples bit-identical; worst metre delta {worst:e} m \
             (quantisation step {} m)",
            w,
            h,
            (max_m - min_m) / 65535.0
        );
        assert!(worst <= 1e-4, "worst metre delta {worst}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
