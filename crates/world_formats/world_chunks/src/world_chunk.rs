//! One world object chunk in column form, and its decoder from chunk JSON.
//!
//! **Role:** [`WorldChunk`] holds a chunk's placed objects as columns (positions, prefab index,
//! rotation, height, pitch, roll, scale, render class) plus the rows of each render class;
//! [`parse_chunk`] fills it from a gunzipped chunk JSON and the prefab map.
//! **Position:** the base of `world_chunks`, over `prefab_catalog`'s prefab map and render classes;
//! [`crate::chunk_container`] fills the same columns from a `TBDC` container; read by the map
//! engine's world store, scheduler, vegetation and line of sight and by the developer tools'
//! verification.
//! **Signals & state:** none; a chunk is plain data.
//! **Invariants:** every column holds exactly `count` rows; a row the narrowing rejects is
//! dropped, not defaulted; a prefab missing from the map gets `NO_CLASS`.

use std::collections::HashMap;

use serde_json::Value;

use crate::chunk_id::ChunkId;

use prefab_catalog::prefab_rows::PrefabEntry;
use prefab_catalog::render_classes::NO_CLASS;
use prefab_catalog::render_classes::narrow_instance_row_v2;

/// SoA for one parsed chunk. Numeric columns are **truncated to `count`** — the JS master arrays are allocated at `instances.length` but only `[0, count)` is ever read, so the truncated form is the faithful byte-comparable one. `positions` is `[x0,y0,x1,y1,…]`.
#[derive(Default, Clone)]
pub struct WorldChunk {
    /// The chunk's `cx_cy` identifier: the requested one for chunk JSON, the header's for `TBDC`.
    pub id: ChunkId,

    /// Chunk grid column along world X (west edge divided by the chunk edge), read from the
    /// identifier; `NaN` when the identifier's first part is not a number.
    pub cx: f64,

    /// Chunk grid row along map Y, counted south to north, read from the identifier; `NaN` when
    /// the identifier's second part is not a number.
    pub cy: f64,

    /// Number of object rows kept; every column holds exactly this many rows (an instance row
    /// the narrowing rejects is dropped, not counted).
    pub count: u32,

    /// Object positions on the map plane in metres, interleaved `[x0, y0, x1, y1, …]`: two
    /// entries per row.
    pub positions: Vec<f32>,

    /// Per row, the prefab id narrowed to 16 bits: the index into the prefab catalogue and the
    /// key of the map engine's per-prefab glyph and building tables.
    pub prefab_idx: Vec<u16>,

    /// Per row, the object's yaw (heading) in degrees; 0 when the instance row omits it.
    pub rotations: Vec<f32>,

    /// Per row, the object's height in metres (instance row element 3); 0 when the row omits it.
    pub z: Vec<f32>,

    /// Per row, pitch in degrees (instance row element 5); 0 when absent or not finite.
    pub pitch: Vec<f32>,

    /// Per row, roll in degrees (instance row element 6); 0 when absent or not finite.
    pub roll: Vec<f32>,

    /// Per row, uniform scale factor (instance row element 7); 1 when absent, not finite or not
    /// positive, so a row never collapses an object to nothing.
    pub scale: Vec<f32>,

    /// Per row, the render-class wire code (the index into `RENDER_CLASS_CODES`), or [`NO_CLASS`]
    /// for a prefab without a render class or missing from the prefab map.
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
    id: &ChunkId,
    raw: &Value,
    prefab_by_id: &HashMap<u64, PrefabEntry>,
) -> Option<WorldChunk> {
    let instances = raw.get("instances")?.as_array()?;
    let mut parts = id.as_str().split('_');
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
        id: id.clone(),
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
#[path = "tests/world_chunk_tests.rs"]
mod tests;
