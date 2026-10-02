//! Chunk placement, anchor partitioning, density, and forest-region golden invariants.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use serde_json::{Value, json};

use super::{Gate, read_json};
use crate::world_export_pipeline::forest_contours::{
    DENSITY_THRESHOLD, DOMINANT_SHARE, MIN_COMPONENT_CELLS, REGION_CELL_M, Tree,
    derive_forest_regions,
};
use crate::world_export_pipeline::polygon_geometry::{cell_of, check_anchors, chunk_key};
use crate::world_export_pipeline::vegetation_density::{
    DENSITY_CELL_M, DENSITY_CHANNELS, DENSITY_COLS, DENSITY_ROWS, TBDD_FILE_BYTES, TBDD_VERSION,
    accumulate_corners, slice_chunk_corners,
};

/// Golden inputs whose shared spatial computations must agree with committed expectations.
pub(super) struct SpatialFixtures<'a> {
    pub sroot: &'a Path,
    pub chunk_sample: &'a Value,
    pub prefabs_sample: &'a Value,
    pub anchor_fixture: &'a Value,
    pub density_fixture: &'a Value,
    pub density_bin: &'a [u8],
    pub region_fixture: &'a Value,
}

pub(super) fn append_spatial_gates(
    fixtures: SpatialFixtures<'_>,
    gates: &mut Vec<Gate>,
) -> Result<()> {
    let SpatialFixtures {
        sroot,
        chunk_sample,
        prefabs_sample,
        anchor_fixture,
        density_fixture,
        density_bin,
        region_fixture,
    } = fixtures;
    let arr = |v: &Value| v.as_array().cloned().unwrap_or_default();

    // S11
    {
        let mut errs = Vec::new();
        let v_instance = jsonschema::validator_for(&read_json(
            &sroot.join("definitions/map-object-instance.schema.json"),
        )?)
        .map_err(|e| anyhow::anyhow!("compile: {e}"))?;
        let cx = chunk_sample["cx"].as_f64().unwrap_or(0.0);
        let cy = chunk_sample["cy"].as_f64().unwrap_or(0.0);
        let chunk_size = chunk_sample["chunkSizeM"].as_f64().unwrap_or(512.0);
        let rows = arr(&chunk_sample["chunk"]["instances"]);
        if rows.is_empty() {
            errs.push("chunk-sample: empty instances".into());
        }
        let mut prev: Option<Vec<f64>> = None;
        let mut wide = 0usize;
        for (i, row) in rows.iter().enumerate() {
            // Objects schemaVersion 1.1.0: rows are exactly 5 or 8 numbers; an 8-wide row
            // must carry a non-trivial pitch / roll / scale (the converter writes trivial trailers
            // as a 5-wide row, so a padded row would be a canonicality bug in the emitter).
            let nums: Option<Vec<f64>> = row
                .as_array()
                .filter(|a| a.iter().all(Value::is_number))
                .map(|a| a.iter().filter_map(Value::as_f64).collect());
            let Some(t) = nums.filter(|t| t.len() == 5 || t.len() == 8) else {
                errs.push(format!(
                    "chunk-sample[{i}]: not an all-number 5- or 8-tuple"
                ));
                continue;
            };
            if t.len() == 8 {
                wide += 1;
                if t[5] == 0.0 && t[6] == 0.0 && t[7] == 1.0 {
                    errs.push(format!(
                        "chunk-sample[{i}]: 8-wide row with trivial pitch/roll/scale (must be 5-wide)"
                    ));
                }
            }
            if v_instance.iter_errors(row).next().is_some() {
                errs.push(format!("chunk-sample[{i}]: schema invalid"));
            }
            let (x, y) = (t[1], t[2]);
            if x < cx * chunk_size || x >= (cx + 1.0) * chunk_size {
                errs.push(format!(
                    "chunk-sample[{i}]: x {x} outside [{}, {})",
                    cx * chunk_size,
                    (cx + 1.0) * chunk_size
                ));
            }
            if y < cy * chunk_size || y >= (cy + 1.0) * chunk_size {
                errs.push(format!(
                    "chunk-sample[{i}]: y {y} outside [{}, {})",
                    cy * chunk_size,
                    (cy + 1.0) * chunk_size
                ));
            }
            if let Some(p) = &prev {
                let sorted = p[1] < x || (p[1] == x && (p[2] < y || (p[2] == y && p[0] <= t[0])));
                if !sorted {
                    errs.push(format!(
                        "chunk-sample[{i}]: rows not sorted by (x, y, prefabId)"
                    ));
                }
            }
            prev = Some(t);
        }
        if wide == 0 {
            errs.push(
                "chunk-sample: no 8-wide row (the full-transform branch needs golden coverage)"
                    .into(),
            );
        }
        if !arr(prefabs_sample)
            .iter()
            .any(|p| p["render"]["importanceZoom"].is_number())
        {
            errs.push("prefabs-sample: no prefab carries render.importanceZoom (the zoom bump needs golden coverage)".into());
        }
        gates.push(Gate {
            id: "S11",
            label: "chunk golden — 5/8-tuple rows, canonical trailers, bounds, sort order, importanceZoom coverage",
            errs,
        });
    }

    // S12
    {
        let mut errs = Vec::new();
        let world = anchor_fixture["worldSizeM"].as_f64().unwrap_or(0.0);
        let chunk_size = anchor_fixture["chunkSizeM"].as_f64().unwrap_or(512.0);
        let raw = arr(&anchor_fixture["rawEntities"]);
        let expected = &anchor_fixture["expected"];
        let prefabs = arr(&expected["prefabs"]);
        let building_anchors: Vec<Value> = raw
            .iter()
            .filter(|r| {
                let rn = r["resourceName"].as_str().unwrap_or("");
                !rn.is_empty()
                    && prefabs
                        .iter()
                        .any(|p| p["resourceName"] == rn && p["kind"] == "building")
            })
            .cloned()
            .collect();
        if building_anchors.is_empty() {
            errs.push("anchor-fixture: no building anchors".into());
        }
        let chunks = expected["chunks"].clone();
        errs.extend(check_anchors(
            &building_anchors,
            &prefabs,
            |cx, cy| {
                chunks
                    .get(chunk_key(cx, cy))
                    .filter(|v| !v.is_null())
                    .cloned()
            },
            chunk_size,
            world,
            2.0,
        ));
        let mut expected_total = 0usize;
        for (key, chunk) in chunks.as_object().into_iter().flatten() {
            for row in chunk["instances"].as_array().into_iter().flatten() {
                expected_total += 1;
                let a = arr(row);
                let k = chunk_key(
                    cell_of(
                        a.get(1).and_then(Value::as_f64).unwrap_or(0.0),
                        chunk_size,
                        world,
                    ),
                    cell_of(
                        a.get(2).and_then(Value::as_f64).unwrap_or(0.0),
                        chunk_size,
                        world,
                    ),
                );
                if &k != key {
                    errs.push(format!(
                        "anchor-fixture chunk {key}: row {row} partitions to {k}"
                    ));
                }
            }
        }
        if expected_total != building_anchors.len() {
            errs.push(format!("anchor-fixture: expected chunks hold {expected_total} instances, raw has {} building rows (exclusion rule broken)", building_anchors.len()));
        }
        gates.push(Gate {
            id: "S12",
            label: "anchor fixture — shared checkAnchors PASS + partition consistency + exclusions",
            errs,
        });
    }

    // S13
    {
        let mut errs = Vec::new();
        let world = density_fixture["worldSizeM"].as_f64().unwrap_or(0.0);
        let ccx = density_fixture["chunk"]["cx"].as_u64().unwrap_or(0) as usize;
        let ccy = density_fixture["chunk"]["cy"].as_u64().unwrap_or(0) as usize;
        let pos = |key: &str| -> Vec<(f64, f64)> {
            arr(&density_fixture[key])
                .iter()
                .filter_map(|r| Some((r["x"].as_f64()?, r["y"].as_f64()?)))
                .collect()
        };
        let (t_grid, t_size) = accumulate_corners(pos("treePositions").into_iter(), world);
        let (r_grid, r_size) = accumulate_corners(pos("rockPositions").into_iter(), world);
        let t_slice = slice_chunk_corners(&t_grid, t_size, ccx, ccy);
        let r_slice = slice_chunk_corners(&r_grid, r_size, ccx, ccy);
        let rebuilt = website_map_engine::io::density::tbdd::encode_tbdd(
            DENSITY_CELL_M,
            DENSITY_COLS,
            DENSITY_ROWS,
            &[&t_slice, &r_slice],
        );
        let expected_bytes = density_fixture["expectedFileBytes"].as_u64().unwrap_or(0) as usize;
        if expected_bytes != TBDD_FILE_BYTES {
            errs.push(format!("fixture expectedFileBytes {expected_bytes} != lib TBDD_FILE_BYTES {TBDD_FILE_BYTES}"));
        }
        if density_bin.len() != TBDD_FILE_BYTES {
            errs.push(format!(
                "committed bin {} bytes, want {TBDD_FILE_BYTES}",
                density_bin.len()
            ));
        }
        if rebuilt != density_bin {
            errs.push("encode(fixture) != committed density-fixture.bin".into());
        }
        match website_map_engine::io::density::tbdd::decode_tbdd(density_bin) {
            Ok(dec) => {
                if dec.version != TBDD_VERSION
                    || dec.cell_m != DENSITY_CELL_M
                    || dec.cols != DENSITY_COLS
                    || dec.rows != DENSITY_ROWS
                    || dec.channels.len() != DENSITY_CHANNELS.len()
                {
                    errs.push(format!(
                        "decoded header mismatch: v={} cellM={} cols={} rows={} ch={}",
                        dec.version,
                        dec.cell_m,
                        dec.cols,
                        dec.rows,
                        dec.channels.len()
                    ));
                }
                let mut sparse: HashMap<(u64, u64), (u64, u64)> = HashMap::new();
                for e in arr(&density_fixture["expectedCorners"]) {
                    sparse.insert(
                        (e["i"].as_u64().unwrap_or(0), e["j"].as_u64().unwrap_or(0)),
                        (
                            e["tree"].as_u64().unwrap_or(0),
                            e["rock"].as_u64().unwrap_or(0),
                        ),
                    );
                }
                let cols = usize::from(DENSITY_COLS);
                for j in 0..usize::from(DENSITY_ROWS) {
                    for i in 0..cols {
                        let (etree, erock) =
                            sparse.get(&(i as u64, j as u64)).copied().unwrap_or((0, 0));
                        let tree = u64::from(dec.channels[0][j * cols + i]);
                        let rock = u64::from(dec.channels[1][j * cols + i]);
                        if tree != etree || rock != erock {
                            errs.push(format!("corner ({i},{j}): decoded tree={tree}/rock={rock}, expected tree={etree}/rock={erock}"));
                        }
                    }
                }
            }
            Err(e) => errs.push(format!("decode failed: {e}")),
        }
        errs.truncate(8);
        gates.push(Gate {
            id: "S13",
            label: "TBDD density fixture — encode byte-identity, header contract, expected corners",
            errs,
        });
    }

    // S14
    {
        let mut errs = Vec::new();
        let world = region_fixture["worldSizeM"].as_f64().unwrap_or(0.0);
        let terrain = region_fixture["terrainId"].as_str().unwrap_or("everon");
        let trees: Vec<Tree> = arr(&region_fixture["trees"])
            .iter()
            .filter_map(|t| {
                Some(Tree {
                    x: t["x"].as_f64()?,
                    y: t["y"].as_f64()?,
                    class: t["class"].as_str()?.to_string(),
                })
            })
            .collect();
        let res = derive_forest_regions(&trees, world, terrain);
        let exp = &region_fixture["expected"];
        let params = json!({
            "cellM": REGION_CELL_M as i64, "densityThreshold": DENSITY_THRESHOLD,
            "minComponentCells": MIN_COMPONENT_CELLS, "dominantShare": DOMINANT_SHARE,
        });
        if params != exp["params"] {
            errs.push(format!("params drift: {params} != {}", exp["params"]));
        }
        if res.unassigned_trees != exp["unassignedTrees"].as_u64().unwrap_or(u64::MAX) {
            errs.push(format!(
                "unassignedTrees {} != expected {}",
                res.unassigned_trees, exp["unassignedTrees"]
            ));
        }
        let derived = Value::Array(res.regions.clone());
        if derived != exp["regions"] {
            errs.push("derived regions differ from expected (rings or aggregates)".into());
        }
        let sum: u64 = res
            .regions
            .iter()
            .map(|r| r["treeCount"].as_u64().unwrap_or(0))
            .sum::<u64>()
            + res.unassigned_trees;
        if sum != trees.len() as u64 {
            errs.push(format!(
                "F2 identity on fixture: {sum} != {} trees",
                trees.len()
            ));
        }
        if !res
            .regions
            .iter()
            .any(|r| r["polygon"].as_array().map(Vec::len).unwrap_or(0) > 1)
        {
            errs.push("fixture no longer exercises a hole ring".into());
        }
        if !res
            .regions
            .iter()
            .any(|r| r["dominantSpeciesClass"] == "mixed")
        {
            errs.push("fixture no longer exercises the mixed dominant rule".into());
        }
        gates.push(Gate { id: "S14", label: "forest-region derivation fixture — deterministic rings + aggregates + F2 identity", errs });
    }

    Ok(())
}
