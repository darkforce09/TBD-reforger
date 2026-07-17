//! T-165.4 — shared map-pipeline geometry (port of `scripts/map-assets/lib/anchor-check.mjs`).
//! The remap + partition formulas are intentionally re-implemented here — they must AGREE with
//! the world builder without importing from it (non-circularity: a chunking bug in the builder
//! cannot self-certify).
//!
//! Conventions (T-090.3.1 plan decisions 2 + 4):
//!   raw (engine): x = east, y = altitude, z = north, headingDeg = GetAngles()[1]
//!   map:          x = engine.x, y = engine.z, z = engine.y, rotationDeg = headingDeg
//!   partition:    cell = clamp(floor(coord / chunk_size), 0, cells-1)
//!   chunk rows:   all-number 5-tuple [prefabId, x, y, z, rotationDeg]

use std::collections::{HashMap, HashSet};

use serde_json::Value;

/// Engine-space raw row → map-space point (x, y=engine z, rotationDeg).
#[must_use]
pub fn remap_raw_to_map(raw: &Value) -> (f64, f64, f64) {
    (
        raw["x"].as_f64().unwrap_or(0.0),
        raw["z"].as_f64().unwrap_or(0.0),
        raw["headingDeg"].as_f64().unwrap_or(0.0),
    )
}

/// Grid cell index for one map coordinate (clamped floor — half-open interior, closed last cell).
#[must_use]
pub fn cell_of(coord: f64, chunk_size_m: f64, world_size_m: f64) -> i64 {
    let cells = ((world_size_m / chunk_size_m).round() as i64).max(1);
    ((coord / chunk_size_m).floor() as i64).clamp(0, cells - 1)
}

#[must_use]
pub fn chunk_key(cx: i64, cy: i64) -> String {
    format!("{cx}_{cy}")
}

fn row_prefab_id(row: &Value) -> f64 {
    match row {
        Value::Array(a) => a.first().and_then(Value::as_f64).unwrap_or(-1.0),
        other => other["prefabId"].as_f64().unwrap_or(-1.0),
    }
}
fn row_x(row: &Value) -> f64 {
    match row {
        Value::Array(a) => a.get(1).and_then(Value::as_f64).unwrap_or(f64::NAN),
        other => other["x"].as_f64().unwrap_or(f64::NAN),
    }
}
fn row_y(row: &Value) -> f64 {
    match row {
        Value::Array(a) => a.get(2).and_then(Value::as_f64).unwrap_or(f64::NAN),
        other => other["y"].as_f64().unwrap_or(f64::NAN),
    }
}

/// P1-4 anchor check (see anchor-check.mjs header). `get_chunk(cx, cy)` returns the chunk doc
/// (`{ "instances": [...] }`) or None. Returns errors (empty = PASS).
pub fn check_anchors(
    anchors: &[Value],
    prefabs: &[Value],
    mut get_chunk: impl FnMut(i64, i64) -> Option<Value>,
    chunk_size_m: f64,
    world_size_m: f64,
    tolerance_m: f64,
) -> Vec<String> {
    let mut errors = Vec::new();
    let prefab_rn: HashMap<u64, String> = prefabs
        .iter()
        .filter_map(|p| {
            Some((
                p["prefabId"].as_f64()?.to_bits(),
                p["resourceName"].as_str()?.to_string(),
            ))
        })
        .collect();
    let building_ids: HashSet<u64> = prefabs
        .iter()
        .filter(|p| p["kind"] == "building")
        .filter_map(|p| p["prefabId"].as_f64().map(f64::to_bits))
        .collect();

    struct Hit {
        dist: f64,
        prefab_id: f64,
    }
    let nearest_building = |chunk: &Option<Value>, mx: f64, my: f64, match_rn: Option<&str>| {
        let mut best: Option<Hit> = None;
        let mut best_match: Option<Hit> = None;
        for row in chunk
            .as_ref()
            .and_then(|c| c["instances"].as_array())
            .into_iter()
            .flatten()
        {
            let pid = row_prefab_id(row);
            if !building_ids.contains(&pid.to_bits()) {
                continue;
            }
            let dist = (row_x(row) - mx).hypot(row_y(row) - my);
            if best.as_ref().is_none_or(|b| dist < b.dist) {
                best = Some(Hit {
                    dist,
                    prefab_id: pid,
                });
            }
            if let Some(rn) = match_rn
                && prefab_rn.get(&pid.to_bits()).map(String::as_str) == Some(rn)
                && best_match.as_ref().is_none_or(|b| dist < b.dist)
            {
                best_match = Some(Hit {
                    dist,
                    prefab_id: pid,
                });
            }
        }
        (best, best_match)
    };

    for anchor in anchors {
        let (mx, my, _) = remap_raw_to_map(anchor);
        let rn = anchor["resourceName"].as_str().unwrap_or("?");
        let cx = cell_of(mx, chunk_size_m, world_size_m);
        let cy = cell_of(my, chunk_size_m, world_size_m);
        let label = format!(
            "anchor {rn} @ map({mx},{my}) -> chunk {}",
            chunk_key(cx, cy)
        );

        let home_chunk = get_chunk(cx, cy);
        let (home, best_match) = nearest_building(&home_chunk, mx, my, Some(rn));
        let home_ok = best_match.as_ref().map(|m| m.dist <= tolerance_m) == Some(true);

        let mut neighbor_best: Option<Hit> = None;
        for dx in -1i64..=1 {
            for dy in -1i64..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let (ncx, ncy) = (cx + dx, cy + dy);
                if ncx < 0 || ncy < 0 {
                    continue;
                }
                let nchunk = get_chunk(ncx, ncy);
                let (hit, _) = nearest_building(&nchunk, mx, my, None);
                if let Some(h) = hit
                    && neighbor_best.as_ref().is_none_or(|b| h.dist < b.dist)
                {
                    neighbor_best = Some(h);
                }
            }
        }

        if !home_ok {
            let home_desc = match &home {
                Some(h) => format!(
                    "nearest home building {:.3} m (prefab {})",
                    h.dist, h.prefab_id
                ),
                None => "no building instance in home chunk".to_string(),
            };
            errors.push(format!(
                "{label}: FAIL — {home_desc}, tolerance {tolerance_m} m"
            ));
            continue;
        }
        if let (Some(nb), Some(h)) = (&neighbor_best, &home)
            && nb.dist < h.dist
        {
            errors.push(format!(
                "{label}: partition drift — neighbor chunk holds a nearer building ({:.3} m < home {:.3} m)",
                nb.dist, h.dist
            ));
        }
    }
    errors
}
