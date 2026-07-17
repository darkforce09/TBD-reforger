//! T-165.8 — catalog-v1 world-object build + roads (ports of `build-world-objects.mjs` and
//! `build-roads-from-topo.mjs`). Content-identical to the Node pipeline: identical JSON bytes
//! before compression (js_num integral-number semantics, identical key order via preserve_order,
//! identical sorts), gzip level 9 (flate2 — the N5 one-time re-encode swaps the committed gz
//! container bytes; decompressed content is the proven-equal contract).

use std::collections::HashMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use serde_json::{Map, Value, json};

use super::classify::{Classifier, load_rules, stream_raw_entities};
use super::jsval::{js_normalize, js_num, norm_heading, round2};
use super::pak::PakVfs;
use super::topo::{TOPO_AIRFIELD, TOPO_RIVER, TOPO_ROAD_A, TOPO_ROAD_B, TOPO_STREAM, decode_topo};
use crate::forest::{Tree, derive_forest_regions};
use crate::geometry::{cell_of, chunk_key};
use crate::serve::repo_root;
use crate::{density, forest};

pub const CHUNK_SIZE_M: f64 = 512.0;

pub const PHASE_ORDER: [&str; 5] = [
    "P1_buildings",
    "P2_trees",
    "P3_vegetation",
    "P4_rocks",
    "P5_props",
];

/// Cumulative kind filter per import phase (t090_phased_object_import.md).
pub fn phase_kinds(phase: &str) -> Option<&'static [&'static str]> {
    Some(match phase {
        "P1_buildings" => &["building"],
        "P2_trees" => &["building", "tree", "water"],
        "P3_vegetation" => &["building", "tree", "water", "vegetation"],
        "P4_rocks" => &["building", "tree", "water", "vegetation", "rock"],
        "P5_props" => &["building", "tree", "water", "vegetation", "rock", "prop"],
        _ => return None,
    })
}

pub fn terrain_row(terrain: &str) -> Result<Value> {
    let reg: Value = serde_json::from_str(&std::fs::read_to_string(
        repo_root().join("packages/map-assets/terrain-registry.json"),
    )?)?;
    reg["terrains"]
        .as_array()
        .and_then(|a| a.iter().find(|t| t["terrainId"] == terrain).cloned())
        .ok_or_else(|| anyhow::anyhow!("terrain '{terrain}' not in terrain-registry.json"))
}

pub fn gz9(bytes: &[u8]) -> Result<Vec<u8>> {
    let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::new(9));
    enc.write_all(bytes)?;
    Ok(enc.finish()?)
}

pub fn gunzip(bytes: &[u8]) -> Result<Vec<u8>> {
    use std::io::Read as _;
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(bytes).read_to_end(&mut out)?;
    Ok(out)
}

fn compact(v: &Value) -> String {
    serde_json::to_string(v).expect("json")
}

fn pretty_nl(v: &Value) -> String {
    serde_json::to_string_pretty(v).expect("json") + "\n"
}

type ChunkRow = (usize, f64, f64, f64, f64);

struct KeptRow {
    resource_name: String,
    kind: String,
    x: f64,
    y: f64,
    z: f64,
    rot: f64,
}

pub struct BuildSummary {
    pub summary: Value,
}

/// The full build-world-objects.mjs port. `out_base = None` → the real terrain dir.
pub fn build_world_objects(
    terrain: &str,
    phase: &str,
    out_base: Option<&Path>,
    patch_manifest: bool,
    ops_log: bool,
) -> Result<BuildSummary> {
    build_world_objects_opt(terrain, phase, out_base, patch_manifest, ops_log, false)
}

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
    let Some(kinds) = phase_kinds(phase) else {
        bail!(
            "phase '{phase}' not implemented (have: {})",
            PHASE_ORDER.join(", ")
        );
    };
    let phase_kind_set: std::collections::HashSet<&str> = kinds.iter().copied().collect();
    let t = terrain_row(terrain)?;
    let b = t["worldBoundsM"].as_array().cloned().unwrap_or_default();
    let (min_x, min_y) = (b[0].as_f64().unwrap_or(-1.0), b[1].as_f64().unwrap_or(-1.0));
    let (max_x, max_y) = (b[2].as_f64().unwrap_or(0.0), b[3].as_f64().unwrap_or(1.0));
    if min_x != 0.0 || min_y != 0.0 || max_x != max_y {
        bail!("worldBoundsM unsupported (expect square, origin 0)");
    }
    let world_size_m = max_x;

    let terrain_dir = repo_root().join("packages/map-assets").join(terrain);
    let staging = terrain_dir.join("staging/export");
    let raw_path = staging.join("raw-entities.jsonl");
    let export_meta_path = staging.join("export-meta.json");
    let stamp_path = staging.join("staged-meta.json");
    for p in [&raw_path, &export_meta_path, &stamp_path] {
        if !p.exists() {
            eprintln!(
                "build-world-objects: missing {} — stage the Workbench export first (copy-world-export-profile --full)",
                p.display()
            );
            std::process::exit(2);
        }
    }
    let out_base: PathBuf = out_base
        .map(Path::to_path_buf)
        .unwrap_or_else(|| terrain_dir.clone());
    let objects_dir = out_base.join("objects");
    let chunks_dir = objects_dir.join("chunks");

    let export_meta: Value = serde_json::from_str(&std::fs::read_to_string(&export_meta_path)?)?;
    let stamp: Value = serde_json::from_str(&std::fs::read_to_string(&stamp_path)?)?;
    let staged_at = stamp["stagedAt"].as_str().unwrap_or_default().to_string();

    // ---- single streaming pass ----
    let rules = load_rules()?;
    let mut classify = Classifier::new(&rules);
    // resourceName -> (count, kind, class, matched), insertion-ordered like the JS Map.
    let mut raw_census: Vec<(String, u64, String, String, bool)> = Vec::new();
    let mut raw_census_idx: HashMap<String, usize> = HashMap::new();
    let mut no_prefab_count = 0u64;
    let mut no_prefab_classes: Vec<(String, u64)> = Vec::new();
    let mut no_prefab_idx: HashMap<String, usize> = HashMap::new();
    let mut out_of_bounds = 0u64;
    let mut kept: Vec<KeptRow> = Vec::new();
    let density_phase = phase_kind_set.contains("tree");
    let rock_in_phase = phase_kind_set.contains("rock");
    let mut rock_rows: Vec<(f64, f64)> = Vec::new();
    let mut rock_out_of_bounds = 0u64;
    const HE_SAMPLE_CAP: usize = 9;
    let mut he_samples: HashMap<String, Vec<[f64; 3]>> = HashMap::new();
    let guid_ok = |rn: &str| {
        rn.len() >= 18
            && rn.starts_with('{')
            && rn.as_bytes()[17] == b'}'
            && rn.as_bytes()[1..17]
                .iter()
                .all(|c| c.is_ascii_digit() || (b'A'..=b'F').contains(c))
    };

    let line_count = stream_raw_entities(&raw_path, |row| {
        let rn = row["resourceName"].as_str().unwrap_or("");
        if rn.is_empty() {
            no_prefab_count += 1;
            let cn = row["className"].as_str().unwrap_or("?").to_string();
            match no_prefab_idx.get(&cn) {
                Some(&i) => no_prefab_classes[i].1 += 1,
                None => {
                    no_prefab_idx.insert(cn.clone(), no_prefab_classes.len());
                    no_prefab_classes.push((cn, 1));
                }
            }
            return;
        }
        let cls = classify.classify(rn);
        match raw_census_idx.get(rn) {
            Some(&i) => raw_census[i].1 += 1,
            None => {
                raw_census_idx.insert(rn.to_string(), raw_census.len());
                raw_census.push((
                    rn.to_string(),
                    1,
                    cls.kind.clone(),
                    cls.class.clone(),
                    cls.matched,
                ));
            }
        }
        if density_phase && cls.kind == "rock" && !rock_in_phase {
            let rx = round2(row["x"].as_f64().unwrap_or(0.0));
            let ry = round2(row["z"].as_f64().unwrap_or(0.0));
            if rx < 0.0 || rx > world_size_m || ry < 0.0 || ry > world_size_m {
                rock_out_of_bounds += 1;
            } else {
                rock_rows.push((rx, ry));
            }
            return;
        }
        if !phase_kind_set.contains(cls.kind.as_str()) {
            return;
        }
        if cls.class == "composition" || cls.class == "buildingpart" {
            return;
        }
        if !guid_ok(rn) {
            return;
        }
        let heading = row["headingDeg"]
            .as_f64()
            .or_else(|| row["pitchDeg"].as_f64())
            .unwrap_or(0.0);
        let x = round2(row["x"].as_f64().unwrap_or(0.0));
        let y = round2(row["z"].as_f64().unwrap_or(0.0)); // map.y = engine z (north)
        if x < 0.0 || x > world_size_m || y < 0.0 || y > world_size_m {
            out_of_bounds += 1;
            return;
        }
        kept.push(KeptRow {
            resource_name: rn.to_string(),
            kind: cls.kind.clone(),
            x,
            y,
            z: round2(row["y"].as_f64().unwrap_or(0.0)),
            rot: norm_heading(heading),
        });
        if let Some(he) = row["halfExtentsM"].as_array()
            && he.len() == 3
            && he
                .iter()
                .all(|v| v.as_f64().is_some_and(|f| f.is_finite() && f >= 0.0))
        {
            let s = he_samples.entry(rn.to_string()).or_default();
            if s.len() < HE_SAMPLE_CAP {
                s.push([
                    he[0].as_f64().unwrap(),
                    he[1].as_f64().unwrap(),
                    he[2].as_f64().unwrap(),
                ]);
            }
        }
    })?;

    if let Some(kc) = export_meta["keptCount"].as_u64()
        && kc != line_count
    {
        eprintln!(
            "build-world-objects: FATAL — raw line count {line_count} != export-meta keptCount {kc} (truncated staging?)"
        );
        std::process::exit(1);
    }

    // ---- prefab table (deduped, sorted by resourceName — G4) ----
    let mut phase_prefab_names: Vec<String> = {
        let mut set: Vec<&str> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for k in &kept {
            if seen.insert(k.resource_name.as_str()) {
                set.push(&k.resource_name);
            }
        }
        set.into_iter().map(str::to_string).collect()
    };
    phase_prefab_names.sort();
    let prefab_id_by_name: HashMap<&str, usize> = phase_prefab_names
        .iter()
        .enumerate()
        .map(|(i, n)| (n.as_str(), i))
        .collect();

    let label_of = |rn: &str| -> String {
        let base = rn.rsplit('/').next().unwrap_or(rn);
        base.strip_suffix(".et").unwrap_or(base).to_string()
    };
    let median = |mut vals: Vec<f64>| -> f64 {
        vals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        vals[vals.len() / 2]
    };

    let mut prefabs: Vec<Value> = Vec::with_capacity(phase_prefab_names.len());
    for (i, rn) in phase_prefab_names.iter().enumerate() {
        let cls = classify.classify(rn);
        let rule = rules.rule(cls.rule_idx).clone();
        // Measured spatial (T-090.3.3): per-axis median of sampled engine halfExtents,
        // remapped to map axes; degenerate medians fall back to the rule template.
        let spatial = match he_samples.get(rn) {
            Some(samples) if !samples.is_empty() => {
                let ex = median(samples.iter().map(|s| s[0]).collect());
                let ey_up = median(samples.iter().map(|s| s[1]).collect());
                let ez_north = median(samples.iter().map(|s| s[2]).collect());
                if ex <= 0.01 || ey_up <= 0.01 || ez_north <= 0.01 {
                    rule["spatial"].clone()
                } else {
                    let hx = round2(ex);
                    let hy = round2(ez_north);
                    let hv = round2(ey_up);
                    let mut m = Map::new();
                    m.insert("model".into(), json!("obb"));
                    m.insert(
                        "pivot".into(),
                        rule["spatial"]["pivot"]
                            .as_str()
                            .map_or(json!("center"), Value::from),
                    );
                    m.insert(
                        "halfExtentsM".into(),
                        Value::Object(Map::from_iter([
                            ("x".to_string(), js_num(hx)),
                            ("y".to_string(), js_num(hy)),
                            ("z".to_string(), js_num(hv)),
                        ])),
                    );
                    m.insert("heightM".into(), js_num(round2(2.0 * hv)));
                    m.insert("footprintM2".into(), js_num(round2(4.0 * hx * hy)));
                    Value::Object(m)
                }
            }
            _ => rule["spatial"].clone(),
        };
        let mut ai = Map::new();
        ai.insert("summary".into(), rule["ai"]["summary"].clone());
        ai.insert("taxonomyPath".into(), rule["ai"]["taxonomyPath"].clone());
        ai.insert("classificationSource".into(), json!("rules-v1/prefab-name"));
        ai.insert(
            "confidence".into(),
            if rule["ai"]["confidence"].is_null() {
                json!(0.5)
            } else {
                rule["ai"]["confidence"].clone()
            },
        );
        ai.insert("needsReview".into(), json!(!cls.matched));
        let mut row = Map::new();
        row.insert("prefabId".into(), json!(i));
        row.insert("resourceName".into(), json!(rn));
        row.insert("kind".into(), json!(cls.kind));
        row.insert("class".into(), json!(cls.class));
        row.insert("label".into(), json!(label_of(rn)));
        row.insert("ai".into(), Value::Object(ai));
        row.insert("spatial".into(), spatial);
        row.insert("gameplay".into(), rule["gameplay"].clone());
        if !rule["render"].is_null() {
            row.insert("render".into(), rule["render"].clone());
        }
        if !rule["tags"].is_null() {
            row.insert("tags".into(), rule["tags"].clone());
        }
        prefabs.push(Value::Object(row));
    }

    // ---- chunk partition (round-then-partition on stored values) ----
    let mut chunks: HashMap<String, Vec<ChunkRow>> = HashMap::new();
    for k in &kept {
        let cx = cell_of(k.x, CHUNK_SIZE_M, world_size_m);
        let cy = cell_of(k.y, CHUNK_SIZE_M, world_size_m);
        chunks.entry(chunk_key(cx, cy)).or_default().push((
            prefab_id_by_name[k.resource_name.as_str()],
            k.x,
            k.y,
            k.z,
            k.rot,
        ));
    }
    for list in chunks.values_mut() {
        list.sort_by(|a, b| {
            a.1.partial_cmp(&b.1)
                .unwrap()
                .then(a.2.partial_cmp(&b.2).unwrap())
                .then(a.0.cmp(&b.0))
        });
    }
    let mut sorted_chunk_keys: Vec<String> = chunks.keys().cloned().collect();
    sorted_chunk_keys.sort_by_key(|k| {
        let mut it = k.split('_').map(|v| v.parse::<i64>().unwrap_or(0));
        (it.next().unwrap_or(0), it.next().unwrap_or(0))
    });

    // ---- write artifacts ----
    let _ = std::fs::remove_dir_all(&chunks_dir);
    std::fs::create_dir_all(&chunks_dir)?;
    let mut prefabs_doc =
        json!({ "schemaVersion": "1.0.0", "terrainId": terrain, "prefabs": prefabs });
    // Rule-copied subtrees may carry float-authored integers (e.g. heightM: 1.0) — normalize
    // to JS number semantics so the JSON bytes match the Node pipeline.
    js_normalize(&mut prefabs_doc);
    std::fs::write(
        objects_dir.join("prefabs.json.gz"),
        gz9(compact(&prefabs_doc).as_bytes())?,
    )?;

    let mut cells: Vec<Value> = Vec::new();
    for key in &sorted_chunk_keys {
        let list = &chunks[key];
        let rows: Vec<Value> = list
            .iter()
            .map(|(id, x, y, z, rot)| {
                Value::Array(vec![
                    js_num(*id as f64),
                    js_num(*x),
                    js_num(*y),
                    js_num(*z),
                    js_num(*rot),
                ])
            })
            .collect();
        let doc = json!({ "instances": rows });
        std::fs::write(
            chunks_dir.join(format!("{key}.json.gz")),
            gz9(compact(&doc).as_bytes())?,
        )?;
        let mut it = key.split('_').map(|v| v.parse::<i64>().unwrap_or(0));
        let (cx, cy) = (it.next().unwrap_or(0), it.next().unwrap_or(0));
        cells.push(json!({
            "cx": cx, "cy": cy, "path": format!("objects/chunks/{key}.json.gz"),
            "instanceCount": list.len(),
        }));
    }
    std::fs::write(
        chunks_dir.join("manifest.json"),
        pretty_nl(&json!({ "chunkSizeM": js_num(CHUNK_SIZE_M), "cells": cells })),
    )?;

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
        let _ = std::fs::remove_dir_all(&density_dir);
        std::fs::create_dir_all(&density_dir)?;
        let grid_cells = (world_size_m / CHUNK_SIZE_M).round() as usize;
        let mut density_bytes = 0u64;
        for cy in 0..grid_cells {
            for cx in 0..grid_cells {
                let tree_ch = density::slice_chunk_corners(&tree_grid, tree_size, cx, cy);
                let rock_ch = density::slice_chunk_corners(&rock_grid, rock_size, cx, cy);
                let buf = map_engine_core::geometry::tbdd::encode_tbdd(
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
        let derived = derive_forest_regions(&trees, world_size_m, terrain);
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
        let _ = std::fs::remove_dir_all(&density_dir);
    }

    // ---- census (catalog scope) ----
    let mut inst_by_prefab = vec![0u64; prefabs.len()];
    for list in chunks.values() {
        for row in list {
            inst_by_prefab[row.0] += 1;
        }
    }
    let kind_order = [
        "building",
        "tree",
        "vegetation",
        "rock",
        "prop",
        "utility",
        "water",
        "road",
    ];
    let mut by_kind: Map<String, Value> = kind_order
        .iter()
        .map(|k| {
            let mut m = Map::from_iter([
                ("prefabTypes".to_string(), json!(0)),
                ("instances".to_string(), json!(0)),
            ]);
            if *k == "road" {
                m.insert("segments".into(), json!(0));
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
        json!("staging/export/raw-entities.jsonl"),
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
    inventory.insert("byRoadClass".into(), json!({}));
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
        set(obj, "schemaVersion", json!("1.0.0"));
        set(obj, "format", json!("catalog-v1"));
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

    // ---- summary + ops log ----
    let mut top_classes = no_prefab_classes.clone();
    top_classes.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let summary = json!({
        "slice": if density_phase { "T-090.3.2" } else { "T-090.3.1" },
        "phase": phase,
        "stagedAt": staged_at,
        "rawLineCount": line_count,
        "rawUniqueResourceNames": raw_census.len(),
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

/// build-roads-from-topo.mjs port. Determinism: records sorted by (type, first x, first y,
/// vertexCount); ids assigned after the sort; points rounded to 2 dp; gzip level 9.
pub fn build_roads_from_topo(
    terrain: &str,
    out_base: Option<&Path>,
    ops_log: bool,
) -> Result<Value> {
    build_roads_from_topo_opt(terrain, out_base, ops_log, false)
}

pub fn build_roads_from_topo_opt(
    terrain: &str,
    out_base: Option<&Path>,
    ops_log: bool,
    quiet: bool,
) -> Result<Value> {
    let vfs = PakVfs::open_default()?;
    let topo = decode_topo(&vfs, terrain)?;
    let road_class = |ty: u8| -> Option<&'static str> {
        match ty {
            TOPO_AIRFIELD => Some("runway"),
            TOPO_RIVER => Some("highway_paved"),
            TOPO_STREAM => Some("road_paved"),
            TOPO_ROAD_A => Some("road_dirt"),
            TOPO_ROAD_B => Some("track"),
            _ => None,
        }
    };
    struct Rec {
        ty: u8,
        points: Vec<(f64, f64)>,
    }
    let mut records: Vec<Rec> = topo
        .records
        .iter()
        .filter(|r| road_class(r.rec_type).is_some())
        .map(|r| {
            let mut points = Vec::with_capacity(r.verts.len() / 2);
            for i in (0..r.verts.len()).step_by(2) {
                points.push((
                    round2(f64::from(r.verts[i])),
                    round2(topo.world_size_m - f64::from(r.verts[i + 1])),
                ));
            }
            Rec {
                ty: r.rec_type,
                points,
            }
        })
        .collect();
    records.sort_by(|a, b| {
        a.ty.cmp(&b.ty)
            .then(a.points[0].0.partial_cmp(&b.points[0].0).unwrap())
            .then(a.points[0].1.partial_cmp(&b.points[0].1).unwrap())
            .then(a.points.len().cmp(&b.points.len()))
    });
    let segments: Vec<Value> = records
        .iter()
        .enumerate()
        .map(|(i, r)| {
            json!({
                "id": format!("road-{terrain}-{i:04}"),
                "roadClass": road_class(r.ty).unwrap(),
                "points": r.points.iter().map(|(x, y)| Value::Array(vec![js_num(*x), js_num(*y)])).collect::<Vec<_>>(),
            })
        })
        .collect();
    let doc = json!({ "schemaVersion": "1.0.0", "terrainId": terrain, "roadSegments": segments });
    let out_base: PathBuf = out_base
        .map(Path::to_path_buf)
        .unwrap_or_else(|| repo_root().join("packages/map-assets").join(terrain));
    let objects_dir = out_base.join("objects");
    std::fs::create_dir_all(&objects_dir)?;
    std::fs::write(
        objects_dir.join("roads.json.gz"),
        gz9(compact(&doc).as_bytes())?,
    )?;

    let mut by_class: Map<String, Value> = Map::new();
    for r in &records {
        let c = road_class(r.ty).unwrap();
        let n = by_class.get(c).and_then(Value::as_u64).unwrap_or(0);
        by_class.insert(c.to_string(), json!(n + 1));
    }
    let summary = json!({
        "slice": "T-090.3.3",
        "source": "decode-topo section 1",
        "classMappingProvisional": false,
        "classByTopoType": { "0": "runway", "1": "highway_paved", "2": "road_paved", "3": "road_dirt", "5": "track" },
        "segments": records.len(),
        "byClass": by_class,
        "points": records.iter().map(|r| r.points.len()).sum::<usize>(),
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
        ops["fullExport"]["roads"] = summary.clone();
        std::fs::write(&ops_path, pretty_nl(&ops))?;
    }
    if !quiet {
        println!("build-roads-from-topo: {terrain} — {}", compact(&summary));
    }
    Ok(summary)
}
