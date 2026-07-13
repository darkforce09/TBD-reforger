//! Oriented-bounding-box footprint corners + the building/pier prefab filter — ports of
//! `obbCorners` (`buildingLayer.ts:47`) and `buildingPrefabLookup` (`:69`). `obb_corners` is a
//! **Class T** kernel (≤ 1 ULP vs the TS): rotation `0° = north (+y)`, clockwise-positive.

use std::collections::HashMap;

use serde_json::Value;

/// `obbCorners(x, y, halfX, halfY, rotationDeg)` (`buildingLayer.ts:47`). Returns the 4-corner
/// ring (unclosed) in world meters, order `(−hX,−hY) (hX,−hY) (hX,hY) (−hX,hY)`.
///
/// Bit-exact rules: `rad = (deg * PI) / 180` in that operation order; each coordinate is summed
/// **left-to-right with no fused multiply-add** (`x + dx*cos + dy*sin` ⇒ `(x + dx*cos) + dy*sin`)
/// — JS performs no FMA here, so fusing would diverge. All-f64.
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
    pub building_class: String,
    pub half_x: f64,
    pub half_y: f64,
}

/// Footprint info for a fence prop (`kind=prop`, `class=fence`).
#[derive(Clone, Debug, PartialEq)]
pub struct FencePrefabInfo {
    pub half_x: f64,
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

/// `buildingPrefabLookup(raw)` (`:69`). Keeps a prefab iff it has a numeric `prefabId` **and**
/// is a `building`, or a `water` pier/dock. `halfX`/`halfY` fall back to `2.0` when absent or
/// `≤ 0`. Keyed by `prefabId.to_bits()` (matches the JS `Map<number,…>`).
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
        lookup.insert(
            prefab_id.to_bits(),
            BuildingPrefabInfo {
                building_class: cls.to_string(),
                half_x: hx.filter(|&v| v > 0.0).unwrap_or(2.0),
                half_y: hy.filter(|&v| v > 0.0).unwrap_or(2.0),
            },
        );
    }
    lookup
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// `toBeCloseTo(_, 6)` in the TS test ≈ 5e-7; assert to 1e-9 here (the ≤1-ULP-vs-TS bar is
    /// the runtime job of `world.parity.test.ts`).
    fn close(ring: [[f64; 2]; 4], expected: [[f64; 2]; 4]) {
        for (g, e) in ring.iter().zip(expected.iter()) {
            assert!((g[0] - e[0]).abs() < 1e-9, "x: {g:?} vs {e:?}");
            assert!((g[1] - e[1]).abs() < 1e-9, "y: {g:?} vs {e:?}");
        }
    }

    #[test]
    fn obb_zero_deg_is_axis_aligned_exact() {
        // rad=0 → cos=1, sin=0 exactly → integer corners.
        let ring = obb_corners(100.0, 200.0, 5.0, 3.0, 0.0);
        assert_eq!(
            ring,
            [[95.0, 197.0], [105.0, 197.0], [105.0, 203.0], [95.0, 203.0]]
        );
    }

    #[test]
    fn obb_ninety_deg_swaps_extents() {
        close(
            obb_corners(100.0, 200.0, 5.0, 3.0, 90.0),
            [[97.0, 205.0], [97.0, 195.0], [103.0, 195.0], [103.0, 205.0]],
        );
    }

    #[test]
    fn obb_360_equals_0_and_area_invariant() {
        close(
            obb_corners(0.0, 0.0, 4.0, 2.0, 360.0),
            obb_corners(0.0, 0.0, 4.0, 2.0, 0.0),
        );
        // Shoelace |area| = (2·4)·(2·2) = 32 for any rotation.
        let ring = obb_corners(0.0, 0.0, 4.0, 2.0, 37.0);
        let mut area = 0.0;
        for i in 0..4 {
            let a = ring[i];
            let b = ring[(i + 1) % 4];
            area += a[0] * b[1] - b[0] * a[1];
        }
        assert!((area.abs() / 2.0 - 32.0).abs() < 1e-9);
    }

    #[test]
    fn lookup_keeps_buildings_and_piers_only() {
        let raw = json!({ "prefabs": [
            { "prefabId": 0, "kind": "building", "class": "residential", "spatial": { "halfExtentsM": { "x": 5, "y": 5, "z": 4 } } },
            { "prefabId": 331, "kind": "tree", "class": "conifer", "spatial": { "halfExtentsM": { "x": 2, "y": 2 } } },
            { "prefabId": 400, "kind": "water", "class": "pier", "spatial": { "halfExtentsM": { "x": 10, "y": 1.5 } } },
            { "prefabId": 401, "kind": "water", "class": "buoy", "spatial": { "halfExtentsM": { "x": 0.5, "y": 0.5 } } }
        ]});
        let lu = building_prefab_lookup(&raw);
        assert_eq!(lu.len(), 2);
        assert_eq!(
            lu.get(&0.0_f64.to_bits()),
            Some(&BuildingPrefabInfo {
                building_class: "residential".into(),
                half_x: 5.0,
                half_y: 5.0
            })
        );
        assert_eq!(
            lu.get(&400.0_f64.to_bits()),
            Some(&BuildingPrefabInfo {
                building_class: "pier".into(),
                half_x: 10.0,
                half_y: 1.5
            })
        );
        assert!(!lu.contains_key(&331.0_f64.to_bits())); // tree
        assert!(!lu.contains_key(&401.0_f64.to_bits())); // buoy
        assert_eq!(building_prefab_lookup(&Value::Null).len(), 0);
    }

    #[test]
    fn lookup_defaults_half_extents_to_two() {
        let raw = json!({ "prefabs": [
            { "prefabId": 7, "kind": "building", "class": "hut" }  // no spatial → defaults 2/2
        ]});
        let lu = building_prefab_lookup(&raw);
        assert_eq!(
            lu.get(&7.0_f64.to_bits()),
            Some(&BuildingPrefabInfo {
                building_class: "hut".into(),
                half_x: 2.0,
                half_y: 2.0
            })
        );
    }
}
