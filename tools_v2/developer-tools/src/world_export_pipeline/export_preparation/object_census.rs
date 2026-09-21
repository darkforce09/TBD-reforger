use super::*;
use crate::repository_layout::{map_scratch_dir, terrain_dir};

pub(super) fn classified_rows(
    rules: &Rules,
    path: &Path,
) -> Result<Vec<(Value, String, String, bool)>> {
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

pub(super) fn is_finite(v: &Value) -> bool {
    v.as_f64().is_some_and(f64::is_finite)
}

pub(super) fn entry_is_k1_building(row: &Value, kind: &str) -> bool {
    kind == "building"
        && row["resourceName"].as_str().is_some_and(|s| !s.is_empty())
        && ["x", "y", "z", "yawDeg", "pitchDeg", "rollDeg"]
            .iter()
            .all(|k| is_finite(&row[*k]))
}

pub fn verify_spike_k1(terrain: &str) -> Result<u8> {
    let raw = map_scratch_dir(&repo_root(), terrain).join("spike/raw-entities.jsonl");
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

pub fn census_spike(terrain: &str) -> Result<u8> {
    let staging = map_scratch_dir(&repo_root(), terrain).join("spike");
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
    super::super::refuse_empty_write(
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
        "sourceExportPath": format!("assets_v2/scratch/{terrain}/spike/raw-entities.jsonl"),
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

pub(super) fn spawn_type_inventory_gate() -> Result<bool> {
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
    let inventory_path = terrain_dir(&root, terrain).join("objects/type-inventory.json");
    if !inventory_path.exists() {
        eprintln!("map-census: missing {}", inventory_path.display());
        return Ok(1);
    }
    if !spawn_type_inventory_gate()? {
        return Ok(1);
    }
    let inv: Value = serde_json::from_str(&std::fs::read_to_string(&inventory_path)?)?;
    let full = map_scratch_dir(&root, terrain).join("export/raw-entities.jsonl");
    let spike = map_scratch_dir(&root, terrain).join("spike/raw-entities.jsonl");
    if inv["censusStatus"] == "pending_export" {
        if full.exists() {
            eprintln!(
                "map-census: full-map export exists but censusStatus is still pending_export — run full classify + census implementation (T-090.2/.3)"
            );
            return Ok(1);
        }
        if spike.exists() {
            println!(
                "map-census: {terrain} censusStatus=pending_export — T-090.3.0 spike subregion export present (assets_v2/scratch/{terrain}/spike); full-map census still pending (expected)"
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
        crate::repository_layout::object_type_inventory(&root, terrain),
        serde_json::to_string_pretty(&inv)? + "\n",
    )?;
    Ok(0)
}
