//! Role: obb tests.
//! Position: `environment/buildings/tests` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use crate::environment::buildings::obb::*;
use serde_json::json;

fn close(ring: [[f64; 2]; 4], expected: [[f64; 2]; 4]) {
    for (g, e) in ring.iter().zip(expected.iter()) {
        assert!((g[0] - e[0]).abs() < 1e-9, "x: {g:?} vs {e:?}");
        assert!((g[1] - e[1]).abs() < 1e-9, "y: {g:?} vs {e:?}");
    }
}

#[test]
fn obb_zero_deg_is_axis_aligned_exact() {
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
            half_y: 5.0,
            importance_zoom: None
        })
    );
    assert_eq!(
        lu.get(&400.0_f64.to_bits()),
        Some(&BuildingPrefabInfo {
            building_class: "pier".into(),
            half_x: 10.0,
            half_y: 1.5,
            importance_zoom: None
        })
    );
    assert!(!lu.contains_key(&331.0_f64.to_bits()));
    assert!(!lu.contains_key(&401.0_f64.to_bits()));
    assert_eq!(building_prefab_lookup(&Value::Null).len(), 0);
}

#[test]
fn lookup_defaults_half_extents_to_two() {
    let raw = json!({ "prefabs": [
        { "prefabId": 7, "kind": "building", "class": "hut" }
    ]});
    let lu = building_prefab_lookup(&raw);
    assert_eq!(
        lu.get(&7.0_f64.to_bits()),
        Some(&BuildingPrefabInfo {
            building_class: "hut".into(),
            half_x: 2.0,
            half_y: 2.0,
            importance_zoom: None
        })
    );
}

#[test]
fn lookup_parses_importance_zoom() {
    let raw = json!({ "prefabs": [
        { "prefabId": 12, "kind": "building", "class": "lighthouse",
          "render": { "iconKey": "building-lighthouse", "importanceZoom": -4 } },
        { "prefabId": 13, "kind": "building", "class": "residential",
          "render": { "iconKey": "building-residential" } }
    ]});
    let lu = building_prefab_lookup(&raw);
    assert_eq!(
        lu.get(&12.0_f64.to_bits()).unwrap().importance_zoom,
        Some(-4.0)
    );
    assert_eq!(lu.get(&13.0_f64.to_bits()).unwrap().importance_zoom, None);
}
