//! Validate staged entities and build the classified, deterministically partitioned catalog.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use serde_json::{Map, Value, json};

use super::{
    CHUNK_SIZE_M, ChunkRow, KeptRow, PHASE_ORDER, compact, gz9, phase_kinds, pretty_nl, terrain_row,
};
use crate::browser_testing::server::repo_root;
use crate::repository_layout::terrain_dir;
use crate::world_export_pipeline::binary_emit;
use crate::world_export_pipeline::classify::{Classifier, Rules, load_rules, stream_raw_entities};
use crate::world_export_pipeline::json_number_formatting::{
    chunk_row_values, js_normalize, js_num, norm_heading, round2, round3, trailers_trivial,
};
use crate::world_export_pipeline::polygon_geometry::{cell_of, chunk_key};

/// Catalog state shared by density generation, inventory emission, and manifest updates.
pub(super) struct PreparedWorldObjects {
    pub(super) world_size_m: f64,
    pub(super) terrain_dir: PathBuf,
    pub(super) out_base: PathBuf,
    pub(super) objects_dir: PathBuf,
    pub(super) export_meta: Value,
    pub(super) staged_at: String,
    pub(super) rules: Rules,
    pub(super) raw_census: Vec<(String, u64, String, String, bool)>,
    pub(super) no_prefab_count: u64,
    pub(super) no_prefab_classes: Vec<(String, u64)>,
    pub(super) out_of_bounds: u64,
    pub(super) rows_with_scale: u64,
    pub(super) kept: Vec<KeptRow>,
    pub(super) density_phase: bool,
    pub(super) rock_rows: Vec<(f64, f64)>,
    pub(super) rock_out_of_bounds: u64,
    pub(super) line_count: u64,
    pub(super) prefabs: Vec<Value>,
    pub(super) chunks: HashMap<String, Vec<ChunkRow>>,
    pub(super) rows_wide: u64,
    pub(super) rows_wide_by_kind: BTreeMap<String, u64>,
    pub(super) sorted_chunk_keys: Vec<String>,
}

pub(super) fn prepare_world_objects(
    terrain: &str,
    phase: &str,
    out_base: Option<&Path>,
) -> Result<PreparedWorldObjects> {
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

    let terrain_dir = terrain_dir(&repo_root(), terrain);
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
    // T-090.12.1 — how many raw rows carried a `scale` key (0 for a pre-v2 export).
    let mut rows_with_scale = 0u64;
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
        // T-090.12.1 — the full transform. `-0` rounds to +0 so a flat entity stays 5-wide.
        let pitch = round2(row["pitchDeg"].as_f64().unwrap_or(0.0));
        let roll = round2(row["rollDeg"].as_f64().unwrap_or(0.0));
        let scale = match row["scale"].as_f64() {
            Some(s) if s.is_finite() && s > 0.0 => {
                rows_with_scale += 1;
                round3(s)
            }
            _ => 1.0,
        };
        kept.push(KeptRow {
            resource_name: rn.to_string(),
            kind: cls.kind.clone(),
            x,
            y,
            z: round2(row["y"].as_f64().unwrap_or(0.0)),
            rot: norm_heading(heading),
            pitch,
            roll,
            scale,
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
        chunks.entry(chunk_key(cx, cy)).or_default().push(ChunkRow {
            id: prefab_id_by_name[k.resource_name.as_str()],
            x: k.x,
            y: k.y,
            z: k.z,
            rot: k.rot,
            pitch: k.pitch,
            roll: k.roll,
            scale: k.scale,
        });
    }
    for list in chunks.values_mut() {
        list.sort_by(|a, b| {
            a.x.partial_cmp(&b.x)
                .unwrap()
                .then(a.y.partial_cmp(&b.y).unwrap())
                .then(a.id.cmp(&b.id))
        });
    }
    // T-090.12.1 — the row-width census (reported, and the manifest's `transforms` word).
    let mut rows_wide = 0u64;
    let mut rows_wide_by_kind: BTreeMap<String, u64> = BTreeMap::new();
    for k in &kept {
        if !trailers_trivial(k.pitch, k.roll, k.scale) {
            rows_wide += 1;
            *rows_wide_by_kind.entry(k.kind.clone()).or_default() += 1;
        }
    }
    let mut sorted_chunk_keys: Vec<String> = chunks.keys().cloned().collect();
    sorted_chunk_keys.sort_by_key(|k| {
        let mut it = k.split('_').map(|v| v.parse::<i64>().unwrap_or(0));
        (it.next().unwrap_or(0), it.next().unwrap_or(0))
    });

    // ---- write artifacts ----
    // T-537: refuse wiping objects/ with an empty catalog (would overwrite committed prefabs/chunks).
    crate::world_export_pipeline::refuse_empty_write(
        "build-world-objects catalog",
        kept.is_empty() || phase_prefab_names.is_empty(),
        "zero kept instances/prefabs — refusing empty objects/ overwrite",
    )?;
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

    // T-935.2 — the class byte the chunk JSON does not carry, from the catalogue just written.
    let class_by_pid = binary_emit::class_code_table(&prefabs_doc);
    let mut cells: Vec<Value> = Vec::new();
    for key in &sorted_chunk_keys {
        let list = &chunks[key];
        let rows: Vec<Value> = list
            .iter()
            .map(|r| {
                Value::Array(chunk_row_values(
                    r.id as f64,
                    r.x,
                    r.y,
                    r.z,
                    r.rot,
                    r.pitch,
                    r.roll,
                    r.scale,
                ))
            })
            .collect();
        // T-935.2 — narrowed from the SAME rows the gz write below serialises (see binary_emit).
        let pods = binary_emit::pods_from_rows(&rows, &class_by_pid);
        let doc = json!({ "instances": rows });
        std::fs::write(
            chunks_dir.join(format!("{key}.json.gz")),
            gz9(compact(&doc).as_bytes())?,
        )?;
        let mut it = key.split('_').map(|v| v.parse::<i64>().unwrap_or(0));
        let (cx, cy) = (it.next().unwrap_or(0), it.next().unwrap_or(0));
        // T-935.2 — dual emission: the binary twin beside the gz-JSON, which stays authoritative
        // until T-935.13 flips the manifest.
        binary_emit::write_chunk_bin(&chunks_dir.join(format!("{key}.bin")), cx, cy, &pods)?;
        cells.push(json!({
            "cx": cx, "cy": cy, "path": format!("objects/chunks/{key}.json.gz"),
            "instanceCount": list.len(),
        }));
    }
    std::fs::write(
        chunks_dir.join("manifest.json"),
        pretty_nl(&json!({ "chunkSizeM": js_num(CHUNK_SIZE_M), "cells": cells })),
    )?;

    Ok(PreparedWorldObjects {
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
    })
}
