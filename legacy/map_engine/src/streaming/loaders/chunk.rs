//! Role: chunk.
//! Position: `streaming/loaders` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use std::collections::HashMap;

use serde_json::Value;

use crate::world::environment::buildings::prefab::PrefabEntry;
use crate::world::environment::classify::NO_CLASS;
use crate::world::environment::classify::narrow_instance_row_v2;

/// SoA for one parsed chunk. Numeric columns are **truncated to `count`** — the JS master arrays are allocated at `instances.length` but only `[0, count)` is ever read, so the truncated form is the faithful byte-comparable one. `positions` is `[x0,y0,x1,y1,…]`.
#[derive(Default, Clone)]
pub struct WorldChunk {
    /// Id.
    pub id: String,

    /// Cx.
    pub cx: f64,

    /// Cy.
    pub cy: f64,

    /// Count.
    pub count: u32,

    /// Positions.
    pub positions: Vec<f32>,

    /// Prefab idx.
    pub prefab_idx: Vec<u16>,

    /// Rotations.
    pub rotations: Vec<f32>,

    /// Z.
    pub z: Vec<f32>,

    /// Pitch.
    pub pitch: Vec<f32>,

    /// Roll.
    pub roll: Vec<f32>,

    /// Scale.
    pub scale: Vec<f32>,

    /// Cls codes.
    pub cls_codes: Vec<u8>,

    /// Render-class code → row indices (encounter order), the `rowsByClass` gather lists.
    pub rows_by_class: HashMap<u8, Vec<u32>>,
}

#[must_use]
fn number_of(s: Option<&str>) -> f64 {
    s.and_then(|v| v.parse::<f64>().ok()).unwrap_or(f64::NAN)
}

/// `parseChunk(id, raw)` (`:571`). `raw` is the gunzipped chunk JSON; `prefab_by_id` is the `buildPrefabMaps` table (keyed by `pid.to_bits()`). Returns `None` when `raw.instances` is not an array (the JS early return).
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
#[path = "tests/chunk_tests.rs"]
mod tests;
