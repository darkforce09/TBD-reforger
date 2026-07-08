//! Prefab-row narrowing + prefab map build — ports of `narrowPrefabRows`/`narrowSpatial`/
//! `narrowRender` (`worldObjectsCore.ts:291`) and `buildPrefabMaps` (`:381`). The map value
//! (`code` + row) is what a chunk's per-instance class lookup reads; `has_oversized` is the
//! §6 oversized-ring flag.

use std::collections::HashMap;

use serde_json::Value;

use super::classify::{NO_CLASS, OVERSIZED_HALF_EXTENT_M, class_code, render_class_for_prefab};

/// Clone-safe prefab row subset (mirror of `WorldPrefabRow`). `prefab_id` keeps full f64
/// precision (the join key). Spatial half-extents / render glyph fields are carried for W3.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PrefabRow {
    pub prefab_id: f64,
    pub kind: String,
    pub class: String,
    pub label: Option<String>,
    pub resource_name: Option<String>,
    pub half_x: Option<f64>,
    pub half_y: Option<f64>,
    pub half_z: Option<f64>,
    pub height_m: Option<f64>,
    pub icon_key: Option<String>,
    pub base_size_px: Option<f64>,
    pub default_color: Option<String>,
    pub importance_zoom: Option<f64>,
}

/// `buildPrefabMaps` byId value: prefabId → render-class code + the narrowed row.
#[derive(Clone, Debug, PartialEq)]
pub struct PrefabEntry {
    pub code: u8,
    pub row: PrefabRow,
}

/// A JSON value that is a number → `Some(f64)`, mirroring `typeof v === 'number'`.
#[must_use]
fn num(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::as_f64)
}

/// A JSON value that is a string → owned `String`, mirroring `typeof v === 'string'`.
#[must_use]
fn text(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_str).map(str::to_string)
}

/// `narrowPrefabRows(raw)` (`:291`) — keep rows with a numeric `prefabId` and string `kind`;
/// `class` falls back to `"unknown"`. Spatial + render blocks narrowed field-by-field.
#[must_use]
pub fn narrow_prefab_rows(raw: &Value) -> Vec<PrefabRow> {
    let Some(rows) = raw.get("prefabs").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let Some(prefab_id) = r.get("prefabId").and_then(Value::as_f64) else {
            continue;
        };
        let Some(kind) = r.get("kind").and_then(Value::as_str) else {
            continue;
        };
        let class = r
            .get("class")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();

        let spatial = r.get("spatial");
        let he = spatial.and_then(|s| s.get("halfExtentsM"));
        let render = r.get("render");

        out.push(PrefabRow {
            prefab_id,
            kind: kind.to_string(),
            class,
            label: text(r.get("label")),
            resource_name: text(r.get("resourceName")),
            half_x: num(he.and_then(|h| h.get("x"))),
            half_y: num(he.and_then(|h| h.get("y"))),
            half_z: num(he.and_then(|h| h.get("z"))),
            height_m: num(spatial.and_then(|s| s.get("heightM"))),
            icon_key: text(render.and_then(|r| r.get("iconKey"))),
            base_size_px: num(render.and_then(|r| r.get("baseSizePx"))),
            default_color: text(render.and_then(|r| r.get("defaultColor"))),
            importance_zoom: num(render.and_then(|r| r.get("importanceZoom"))),
        });
    }
    out
}

/// `buildPrefabMaps(prefabRows)` (`:381`) → (prefabId→{code,row}, has_oversized). The map is
/// keyed by `prefab_id.to_bits()` so the chunk lookup matches JS `Map<number,…>` bit-for-bit
/// (both sides parse the same integer to the same f64). `has_oversized` mirrors
/// `cls && Math.max(hx, hy) >= 64` with `hx/hy` defaulting to 0.
#[must_use]
pub fn build_prefab_maps(rows: Vec<PrefabRow>) -> (HashMap<u64, PrefabEntry>, bool) {
    let mut by_id = HashMap::with_capacity(rows.len());
    let mut has_oversized = false;
    for row in rows {
        let cls = render_class_for_prefab(&row.kind, &row.class);
        let code = cls.map_or(NO_CLASS, class_code);
        let hx = row.half_x.unwrap_or(0.0);
        let hy = row.half_y.unwrap_or(0.0);
        if cls.is_some() && hx.max(hy) >= OVERSIZED_HALF_EXTENT_M {
            has_oversized = true;
        }
        by_id.insert(row.prefab_id.to_bits(), PrefabEntry { code, row });
    }
    (by_id, has_oversized)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn narrow_keeps_wellformed_rows() {
        let raw = json!({
            "prefabs": [
                { "prefabId": 0, "kind": "tree", "class": "conifer",
                  "spatial": { "halfExtentsM": { "x": 1.2, "y": 1.2, "z": 6 }, "heightM": 12 },
                  "render": { "iconKey": "tree-conifer", "baseSizePx": 18, "defaultColor": "#2d5a27" } },
                { "prefabId": "bad", "kind": "tree" },          // non-numeric id → dropped
                { "prefabId": 5 },                                // missing kind → dropped
                { "prefabId": 9, "kind": "building" }             // class defaults to "unknown"
            ]
        });
        let rows = narrow_prefab_rows(&raw);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].prefab_id, 0.0);
        assert_eq!(rows[0].half_x, Some(1.2));
        assert_eq!(rows[0].height_m, Some(12.0));
        assert_eq!(rows[0].icon_key.as_deref(), Some("tree-conifer"));
        assert_eq!(rows[1].class, "unknown");
    }

    #[test]
    fn build_maps_codes_and_oversized() {
        let rows = vec![
            PrefabRow {
                prefab_id: 0.0,
                kind: "tree".into(),
                class: "conifer".into(),
                ..Default::default()
            },
            PrefabRow {
                prefab_id: 9.0,
                kind: "building".into(),
                class: "residential".into(),
                half_x: Some(70.0),
                half_y: Some(3.0),
                ..Default::default()
            }, // oversized
            PrefabRow {
                prefab_id: 3.0,
                kind: "misc".into(),
                class: "x".into(),
                ..Default::default()
            }, // NO_CLASS
        ];
        let (by_id, oversized) = build_prefab_maps(rows);
        assert!(oversized);
        assert_eq!(by_id.get(&0.0_f64.to_bits()).unwrap().code, 1); // tree
        assert_eq!(by_id.get(&9.0_f64.to_bits()).unwrap().code, 0); // building
        assert_eq!(by_id.get(&3.0_f64.to_bits()).unwrap().code, NO_CLASS);
    }

    #[test]
    fn oversized_only_when_classified() {
        // A 70 m half-extent on an UNclassified prefab must NOT set oversized (JS: `cls && …`).
        let rows = vec![PrefabRow {
            prefab_id: 1.0,
            kind: "misc".into(),
            class: "x".into(),
            half_x: Some(70.0),
            ..Default::default()
        }];
        let (_by_id, oversized) = build_prefab_maps(rows);
        assert!(!oversized);
    }
}
