//! Chunk parse — the oracle. Verbatim port of `parseChunk` (`worldObjectsCore.ts:571`): the
//! gunzipped `{ instances: [[pid, x, y, z, rot], …] }` payload → a Structure-of-Arrays whose
//! column shapes match the JS `Float32Array`/`Uint16Array`/`Uint8Array` byte layout exactly
//! (Class R), plus the per-render-class row-index gather lists (`rowsByClass`, Class S).

use std::collections::HashMap;

use serde_json::Value;

use super::classify::{NO_CLASS, narrow_instance_row_v2};
use super::prefab::PrefabEntry;

/// SoA for one parsed chunk. Numeric columns are **truncated to `count`** — the JS master
/// arrays are allocated at `instances.length` but only `[0, count)` is ever read, so the
/// truncated form is the faithful byte-comparable one. `positions` is `[x0,y0,x1,y1,…]`.
#[derive(Default, Clone)]
pub struct WorldChunk {
    pub id: String,
    pub cx: f64,
    pub cy: f64,
    pub count: u32,
    pub positions: Vec<f32>,
    pub prefab_idx: Vec<u16>,
    pub rotations: Vec<f32>,
    pub z: Vec<f32>,
    /// T-090.12.1 — full-transform columns (`objects` schemaVersion 1.1.0): pitch / roll in
    /// Enfusion `GetAngles()` degrees and the uniform scale per row; identity (`0`, `0`, `1`)
    /// for every v1 5-wide row, so every pre-v2 reader of the columns above is unaffected.
    pub pitch: Vec<f32>,
    pub roll: Vec<f32>,
    pub scale: Vec<f32>,
    pub cls_codes: Vec<u8>,
    /// Render-class code → row indices (encounter order), the `rowsByClass` gather lists.
    pub rows_by_class: HashMap<u8, Vec<u32>>,
}

/// `Number(idStr)` for a chunk-id component (`"cx"`/`"cy"`). Real ids are `{int}_{int}`, so a
/// plain f64 parse matches `Number(...)`; unparseable → NaN (metadata only, not a parity column).
#[must_use]
fn number_of(s: Option<&str>) -> f64 {
    s.and_then(|v| v.parse::<f64>().ok()).unwrap_or(f64::NAN)
}

/// `parseChunk(id, raw)` (`:571`). `raw` is the gunzipped chunk JSON; `prefab_by_id` is the
/// `buildPrefabMaps` table (keyed by `pid.to_bits()`). Returns `None` when `raw.instances` is
/// not an array (the JS early return).
///
/// Bit-exact hazards preserved: `x/y/rot/z` stored as `as f32` (JS `Float32Array`), `pid as u16`
/// (JS `Uint16Array`; Everon pids < 65536 so identical to ToUint16), and the class lookup uses
/// the **untruncated** `pid`. Rows failing `narrow_instance_row` are skipped with no gap.
#[must_use]
pub fn parse_chunk(
    id: &str,
    raw: &Value,
    prefab_by_id: &HashMap<u64, PrefabEntry>,
) -> Option<WorldChunk> {
    let instances = raw.get("instances")?.as_array()?;
    let mut parts = id.split('_');
    let cx = number_of(parts.next());
    let cy = number_of(parts.next());

    let n = instances.len();
    let mut positions: Vec<f32> = Vec::with_capacity(2 * n);
    let mut prefab_idx: Vec<u16> = Vec::with_capacity(n);
    let mut rotations: Vec<f32> = Vec::with_capacity(n);
    let mut z: Vec<f32> = Vec::with_capacity(n);
    let mut pitch: Vec<f32> = Vec::with_capacity(n);
    let mut roll: Vec<f32> = Vec::with_capacity(n);
    let mut scale: Vec<f32> = Vec::with_capacity(n);
    let mut cls_codes: Vec<u8> = Vec::with_capacity(n);
    let mut rows_by_class: HashMap<u8, Vec<u32>> = HashMap::new();
    let mut count: u32 = 0;

    for inst in instances {
        let Some(row) = narrow_instance_row_v2(inst) else {
            continue;
        };
        let (pid, x, y, zv, rot) = (row.pid, row.x, row.y, row.z, row.rot);
        let i = count;
        count += 1;
        positions.push(x as f32);
        positions.push(y as f32);
        prefab_idx.push(pid as u16);
        rotations.push(rot as f32);
        z.push(zv as f32);
        pitch.push(row.pitch as f32);
        roll.push(row.roll as f32);
        scale.push(row.scale as f32);
        let code = prefab_by_id
            .get(&pid.to_bits())
            .map_or(NO_CLASS, |e| e.code);
        cls_codes.push(code);
        if code != NO_CLASS {
            rows_by_class.entry(code).or_default().push(i);
        }
    }

    Some(WorldChunk {
        id: id.to_string(),
        cx,
        cy,
        count,
        positions,
        prefab_idx,
        rotations,
        z,
        pitch,
        roll,
        scale,
        cls_codes,
        rows_by_class,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::prefab::{PrefabRow, build_prefab_maps, narrow_prefab_rows};
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
    fn parse_chunk_synthetic_exact() {
        // Controlled prefab map: 9→building(0), 14→tree(1), 18→unclassified(255).
        let rows = vec![
            PrefabRow {
                prefab_id: 9.0,
                kind: "building".into(),
                class: "residential".into(),
                ..Default::default()
            },
            PrefabRow {
                prefab_id: 14.0,
                kind: "tree".into(),
                class: "conifer".into(),
                ..Default::default()
            },
            PrefabRow {
                prefab_id: 18.0,
                kind: "misc".into(),
                class: "x".into(),
                ..Default::default()
            },
        ];
        let (by_id, _) = build_prefab_maps(rows);

        let raw = json!({ "instances": [
            [9, 512.0, 700.25, 41.3, 90],
            [14, 800.5, 900.0, 55.0, 47.25],
            [18, 1023.999, 640.0, 60.1, 359.5],
            "garbage-row"                       // dropped by narrow_instance_row
        ]});
        let c = parse_chunk("1_1", &raw, &by_id).unwrap();

        assert_eq!(c.count, 3);
        assert_eq!(c.cx, 1.0);
        assert_eq!(c.cy, 1.0);
        assert_eq!(c.prefab_idx, vec![9u16, 14, 18]);
        assert_eq!(c.cls_codes, vec![0u8, 1, NO_CLASS]);
        assert_eq!(c.rows_by_class.get(&0), Some(&vec![0u32]));
        assert_eq!(c.rows_by_class.get(&1), Some(&vec![1u32]));
        assert_eq!(c.rows_by_class.get(&NO_CLASS), None); // 255 never gathered
        // f32 store boundary (compute the expected via the same `as f32` cast).
        assert_eq!(c.positions.len(), 6);
        assert_eq!(c.positions[0], 512.0_f64 as f32);
        assert_eq!(c.positions[1], 700.25_f64 as f32);
        assert_eq!(c.positions[4], 1023.999_f64 as f32); // not f32-exact → proves the narrowing
        assert_eq!(c.z[0], 41.3_f64 as f32);
        assert_eq!(c.rotations[2], 359.5_f64 as f32);
    }

    #[test]
    fn parse_chunk_golden_consistent() {
        let (by_id, _) = build_prefab_maps(narrow_prefab_rows(&json!({
            "prefabs": golden("map-object-prefabs-sample.json")
        })));
        let chunk_raw = golden("map-object-chunk-sample.json");
        let c = parse_chunk("1_1", chunk_raw.get("chunk").unwrap(), &by_id).unwrap();

        // T-090.12.1: the golden gained one 8-wide row (3 → 4 rows; the v2 branch pin).
        assert_eq!(c.count, 4);
        assert_eq!(c.prefab_idx, vec![9u16, 14, 14, 18]);
        assert_eq!(c.positions.len(), 8);
        assert_eq!(c.pitch, vec![0.0_f32, 0.0, -3.5, 0.0]);
        assert_eq!(c.roll, vec![0.0_f32, 0.0, 1.25, 0.0]);
        assert_eq!(c.scale, vec![1.0_f32, 1.0, 1.15, 1.0]);
        // rows_by_class is internally consistent: every gathered index carries that code.
        for (&code, rows) in &c.rows_by_class {
            assert_ne!(code, NO_CLASS);
            for &i in rows {
                assert_eq!(c.cls_codes[i as usize], code);
            }
        }
    }

    /// T-090.12.1 — a mixed 5-/8-wide payload: the v1 columns are byte-identical to the v1
    /// parse (same casts, same order), the new columns default to identity on 5-wide rows.
    #[test]
    fn parse_chunk_v2_columns_default_to_identity_on_v1_rows() {
        let (by_id, _) = build_prefab_maps(vec![PrefabRow {
            prefab_id: 14.0,
            kind: "tree".into(),
            class: "conifer".into(),
            ..Default::default()
        }]);
        let raw = json!({ "instances": [
            [14, 800.5, 900.0, 55.0, 47.25],
            [14, 900.0, 950.0, 52.0, 47.25, -3.5, 1.25, 1.15],
            [14, 901.0, 951.0, 52.0, 47.25, 0, 0, 0.5],
        ]});
        let c = parse_chunk("1_1", &raw, &by_id).unwrap();
        assert_eq!(c.count, 3);
        assert_eq!(
            c.positions,
            vec![800.5_f32, 900.0, 900.0, 950.0, 901.0, 951.0]
        );
        assert_eq!(c.rotations, vec![47.25_f32; 3]);
        assert_eq!(c.pitch, vec![0.0_f32, -3.5, 0.0]);
        assert_eq!(c.roll, vec![0.0_f32, 1.25, 0.0]);
        assert_eq!(c.scale, vec![1.0_f32, 1.15, 0.5]);
        assert_eq!(c.rows_by_class.get(&1), Some(&vec![0u32, 1, 2]));
    }
}
