//! Land-cover region narrowing — port of `parseRegionsPayload`/`narrowRings`
//! (`landCoverRegions.ts:66`/`:51`). Accepts both the shipped `{ regions: [...] }` wrapper and
//! the golden bare-array shape. Structural (**Class S**) vs the TS loader.

use serde_json::Value;

/// One narrowed land-cover region (mirror of `LandCoverRegion`). `polygon` rings: first outer,
/// rest holes.
#[derive(Clone, Debug, PartialEq)]
pub struct LandCoverRegion {
    pub id: String,
    pub kind: String,
    pub polygon: Vec<Vec<[f64; 2]>>,
    pub tree_count: Option<f64>,
    pub dominant_species_class: Option<String>,
    pub density_per_ha: Option<f64>,
    pub area_ha: Option<f64>,
    pub cover_type: Option<String>,
}

/// `kind ∈ {forest, field, waterBody}` (the N5 taxonomy).
#[must_use]
fn is_kind(v: &str) -> bool {
    v == "forest" || v == "field" || v == "waterBody"
}

/// `narrowRings(polygon)` (`:51`) — a non-empty array of rings; each ring `len ≥ 3` of finite
/// `[x, y]` points. Returns `None` (drops the region) on any violation.
#[must_use]
fn narrow_rings(polygon: &Value) -> Option<Vec<Vec<[f64; 2]>>> {
    let arr = polygon.as_array()?;
    if arr.is_empty() {
        return None;
    }
    let mut rings = Vec::with_capacity(arr.len());
    for ring in arr {
        let ra = ring.as_array()?;
        if ra.len() < 3 {
            return None;
        }
        let mut pts = Vec::with_capacity(ra.len());
        for p in ra {
            let pa = p.as_array()?;
            if pa.len() < 2 {
                return None;
            }
            let x = pa[0].as_f64().filter(|n| n.is_finite())?;
            let y = pa[1].as_f64().filter(|n| n.is_finite())?;
            pts.push([x, y]);
        }
        rings.push(pts);
    }
    Some(rings)
}

/// `parseRegionsPayload(raw)` (`:66`). Keeps a row iff `id` is a string, `kind` is a valid kind,
/// and `narrow_rings` succeeds. Accepts a bare array or `{ regions: [...] }`.
#[must_use]
pub fn parse_regions_payload(raw: &Value) -> Vec<LandCoverRegion> {
    let rows = if raw.is_array() {
        raw.as_array()
    } else {
        raw.get("regions").and_then(Value::as_array)
    };
    let Some(rows) = rows else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for row in rows {
        let Some(id) = row.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(kind) = row.get("kind").and_then(Value::as_str) else {
            continue;
        };
        if !is_kind(kind) {
            continue;
        }
        let Some(polygon) = row.get("polygon").and_then(narrow_rings) else {
            continue;
        };
        out.push(LandCoverRegion {
            id: id.to_string(),
            kind: kind.to_string(),
            polygon,
            tree_count: row.get("treeCount").and_then(Value::as_f64),
            dominant_species_class: row
                .get("dominantSpeciesClass")
                .and_then(Value::as_str)
                .map(str::to_string),
            density_per_ha: row.get("densityPerHa").and_then(Value::as_f64),
            area_ha: row.get("areaHa").and_then(Value::as_f64),
            cover_type: row
                .get("coverType")
                .and_then(Value::as_str)
                .map(str::to_string),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use std::fs;

    fn golden(name: &str) -> Value {
        let path = format!(
            "{}/../../packages/tbd-schema/golden/map-objects/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        serde_json::from_slice(&fs::read(&path).expect("read golden")).expect("parse golden")
    }

    #[test]
    fn parses_bare_array_golden() {
        let regions = parse_regions_payload(&golden("map-object-regions-everon-sample.json"));
        assert_eq!(regions.len(), 4);
        assert_eq!(regions[0].id, "forest-everon-001");
        assert_eq!(regions[0].kind, "forest");
        assert_eq!(regions[0].tree_count, Some(12400.0));
        assert_eq!(regions[0].polygon.len(), 1); // one outer ring
        assert_eq!(regions[0].polygon[0].len(), 5); // closed square (5 verts)
        assert_eq!(regions[3].kind, "waterBody");
    }

    #[test]
    fn accepts_wrapped_and_drops_malformed() {
        let raw = json!({ "regions": [
            { "id": "f1", "kind": "forest", "polygon": [[[0, 0], [1, 0], [1, 1]]] },
            { "id": "bad-kind", "kind": "swamp", "polygon": [[[0, 0], [1, 0], [1, 1]]] },
            { "id": "short-ring", "kind": "field", "polygon": [[[0, 0], [1, 0]]] },
            { "kind": "forest", "polygon": [[[0, 0], [1, 0], [1, 1]]] }
        ]});
        let regions = parse_regions_payload(&raw);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].id, "f1");
    }

    #[test]
    fn non_payload_is_empty() {
        assert_eq!(parse_regions_payload(&Value::Null).len(), 0);
        assert_eq!(parse_regions_payload(&json!("<html>")).len(), 0);
    }
}
