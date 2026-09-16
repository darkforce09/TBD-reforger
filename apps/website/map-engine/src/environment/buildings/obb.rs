//! Role: obb.
//! Position: `environment/buildings` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use std::collections::HashMap;

use serde_json::Value;

/// `obbCorners(x, y, halfX, halfY, rotationDeg)` (`buildingLayer.ts:47`). Returns the 4-corner ring (unclosed) in world meters, order `(−hX,−hY) (hX,−hY) (hX,hY) (−hX,hY)`.
#[must_use]
pub fn obb_corners(x: f64, y: f64, half_x: f64, half_y: f64, rotation_deg: f64) -> [[f64; 2]; 4] {
    let rad = (rotation_deg * std::f64::consts::PI) / 180.0;
    let cos = rad.cos();
    let sin = rad.sin();
    let rot = |dx: f64, dy: f64| [x + dx * cos + dy * sin, y - dx * sin + dy * cos];
    [
        rot(-half_x, -half_y),
        rot(half_x, -half_y),
        rot(half_x, half_y),
        rot(-half_x, half_y),
    ]
}

/// Footprint info for a building/pier prefab (mirror of `BuildingPrefabInfo`).
#[derive(Clone, Debug, PartialEq)]
pub struct BuildingPrefabInfo {
    /// Building class.
    pub building_class: String,

    /// Half x.
    pub half_x: f64,

    /// Half y.
    pub half_y: f64,

    /// Importance zoom.
    pub importance_zoom: Option<f64>,
}

/// Footprint info for a fence prop (`kind=prop`, `class=fence`).
#[derive(Clone, Debug, PartialEq)]
pub struct FencePrefabInfo {
    /// Half x.
    pub half_x: f64,

    /// Half y.
    pub half_y: f64,
}

/// `fencePrefabLookup` — keeps prefabs with `kind=prop` and `class=fence`.
#[must_use]
pub fn fence_prefab_lookup(raw: &Value) -> HashMap<u16, FencePrefabInfo> {
    let mut lookup = HashMap::new();
    let Some(rows) = raw.get("prefabs").and_then(Value::as_array) else {
        return lookup;
    };
    for row in rows {
        let Some(prefab_id) = row.get("prefabId").and_then(Value::as_f64) else {
            continue;
        };
        let kind = row.get("kind").and_then(Value::as_str).unwrap_or("");
        let cls = row
            .get("class")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        if kind != "prop" || cls != "fence" {
            continue;
        }
        let he = row.get("spatial").and_then(|s| s.get("halfExtentsM"));
        let hx = he.and_then(|h| h.get("x")).and_then(Value::as_f64);
        let hy = he.and_then(|h| h.get("y")).and_then(Value::as_f64);
        if !(0.0..65536.0).contains(&prefab_id) || prefab_id.fract() != 0.0 {
            continue;
        }
        lookup.insert(
            prefab_id as u16,
            FencePrefabInfo {
                half_x: hx.filter(|&v| v > 0.0).unwrap_or(1.0),
                half_y: hy.filter(|&v| v > 0.0).unwrap_or(0.25),
            },
        );
    }
    lookup
}

/// `buildingPrefabLookup(raw)` (`:69`). Keeps a prefab iff it has a numeric `prefabId` **and** is a `building`, or a `water` pier/dock. `halfX`/`halfY` fall back to `2.0` when absent or `≤ 0`. Keyed by `prefabId.to_bits()` (matches the JS `Map<number,…>`).
#[must_use]
pub fn building_prefab_lookup(raw: &Value) -> HashMap<u64, BuildingPrefabInfo> {
    let mut lookup = HashMap::new();
    let Some(rows) = raw.get("prefabs").and_then(Value::as_array) else {
        return lookup;
    };
    for row in rows {
        let Some(prefab_id) = row.get("prefabId").and_then(Value::as_f64) else {
            continue;
        };
        let cls = row
            .get("class")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let kind = row.get("kind").and_then(Value::as_str).unwrap_or("");
        let included = kind == "building" || (kind == "water" && (cls == "pier" || cls == "dock"));
        if !included {
            continue;
        }
        let he = row.get("spatial").and_then(|s| s.get("halfExtentsM"));
        let hx = he.and_then(|h| h.get("x")).and_then(Value::as_f64);
        let hy = he.and_then(|h| h.get("y")).and_then(Value::as_f64);

        let importance_zoom = row
            .get("render")
            .and_then(|r| r.get("importanceZoom"))
            .and_then(Value::as_f64);
        lookup.insert(
            prefab_id.to_bits(),
            BuildingPrefabInfo {
                building_class: cls.to_string(),
                half_x: hx.filter(|&v| v > 0.0).unwrap_or(2.0),
                half_y: hy.filter(|&v| v > 0.0).unwrap_or(2.0),
                importance_zoom,
            },
        );
    }
    lookup
}

#[cfg(test)]
#[path = "tests/obb_tests.rs"]
mod tests;
