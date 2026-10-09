//! The tree counts that decide between drawing each tree and the forest density fill.
//!
//! **Role:** counts the tree and vegetation rows of the chunks a view draws, exactly
//! ([`exact_tree_count`]) or weighted by the view's cover of each chunk ([`visible_tree_count`]);
//! switches to the heatmap past the instance budget ([`heatmap_trees`]); and packs the per-chunk
//! density grid ([`pack_density_grid_r32`]).
//! **Position:** reads `world_chunks`' chunk rows keyed by their [`ChunkId`], `prefab_catalog`'s
//! class codes and `map_draw_lanes`' budget and gates; the map engine's draw-buffer packer calls it.
//! **Signals & state:** none; pure functions.
//! **Invariants:** vegetation rows count only at the zooms where vegetation draws; the density
//! grid's texels sum to the exact count of the chunks drawn.

use map_coordinates::chunk_math::Bbox;
use map_draw_lanes::zoom_gates::INSTANCE_BUDGET;
use prefab_catalog::render_classes::class_code;
use std::collections::HashMap;
use world_chunks::ChunkId;
use world_chunks::world_chunk::WorldChunk;

/// Parse a `cx_cy` chunk id → `(cx, cy)`. Returns `None` on malformed ids.
#[must_use]
fn parse_chunk_xy(id: &ChunkId) -> Option<(i64, i64)> {
    let (a, b) = id.as_str().split_once('_')?;
    Some((a.parse().ok()?, b.parse().ok()?))
}

/// Exact tree (+ vegetation, when vegetation is drawn at `z`) instance count over `draw_ids` (Class R — sum of row lens).
pub fn exact_tree_count(
    chunks: &HashMap<ChunkId, WorldChunk>,
    draw_ids: &[ChunkId],
    z: f64,
) -> usize {
    let tree_code = class_code("tree");
    let veg_code = class_code("vegetation");
    let count_veg = map_draw_lanes::zoom_gates::class_visible("vegetation", z);
    let mut n = 0usize;
    for id in draw_ids {
        let Some(chunk) = chunks.get(id) else {
            continue;
        };
        if let Some(rows) = chunk.rows_by_class.get(&tree_code) {
            n += rows.len();
        }
        if count_veg && let Some(rows) = chunk.rows_by_class.get(&veg_code) {
            n += rows.len();
        }
    }
    n
}

/// Exact tree+veg count for one chunk id (0 if missing).
#[must_use]
fn exact_tree_count_chunk(chunks: &HashMap<ChunkId, WorldChunk>, id: &ChunkId) -> u32 {
    let tree_code = class_code("tree");
    let veg_code = class_code("vegetation");
    let Some(chunk) = chunks.get(id) else {
        return 0;
    };
    let mut n = 0u32;
    if let Some(rows) = chunk.rows_by_class.get(&tree_code) {
        n += rows.len() as u32;
    }
    if let Some(rows) = chunk.rows_by_class.get(&veg_code) {
        n += rows.len() as u32;
    }
    n
}

/// Ladder switch: heatmap when exact count exceeds [`INSTANCE_BUDGET`].
#[must_use]
pub fn heatmap_trees(exact_count: usize) -> bool {
    exact_count > INSTANCE_BUDGET
}

fn chunk_bbox(cx: i64, cy: i64, size: f64) -> Bbox {
    let x0 = cx as f64 * size;
    let y0 = cy as f64 * size;
    [x0, y0, x0 + size, y0 + size]
}

fn rect_overlap_area(a: Bbox, b: Bbox) -> f64 {
    let w = (a[2].min(b[2]) - a[0].max(b[0])).max(0.0);
    let h = (a[3].min(b[3]) - a[1].max(b[1])).max(0.0);
    w * h
}

/// Tree and vegetation rows of the drawn chunks, each chunk's count weighted by the share of its
/// area the `viewport` box (metres) covers, summed and floored. A chunk missing from `chunks`,
/// with a malformed id or outside the viewport adds nothing; 0 when `chunk_size_m` is not positive.
#[must_use]
pub fn visible_tree_count(
    chunks: &HashMap<ChunkId, WorldChunk>,
    draw_ids: &[ChunkId],
    viewport: Bbox,
    chunk_size_m: f64,
) -> usize {
    let tree_code = class_code("tree");
    let veg_code = class_code("vegetation");
    let chunk_area = chunk_size_m * chunk_size_m;
    if chunk_area <= 0.0 {
        return 0;
    }
    let mut acc = 0.0f64;
    for id in draw_ids {
        let Some(chunk) = chunks.get(id) else {
            continue;
        };
        let Some((cx, cy)) = parse_chunk_xy(id) else {
            continue;
        };
        let frac = (rect_overlap_area(chunk_bbox(cx, cy, chunk_size_m), viewport) / chunk_area)
            .clamp(0.0, 1.0);
        if frac <= 0.0 {
            continue;
        }
        let mut rows = 0usize;
        if let Some(r) = chunk.rows_by_class.get(&tree_code) {
            rows += r.len();
        }
        if let Some(r) = chunk.rows_by_class.get(&veg_code) {
            rows += r.len();
        }
        acc += rows as f64 * frac;
    }
    acc.floor() as usize
}

/// Pack an `n_cx × n_cy` R32Uint grid (row-major, cy outer, cx inner — matches chunk_ids_for_rect). Texel `(cx, cy)` = exact tree+veg count for [`ChunkId::of_cell`]`(cx, cy)` among `resident` chunk map; else 0.
#[must_use]
pub fn pack_density_grid_r32(
    chunks: &HashMap<ChunkId, WorldChunk>,
    n_cx: u32,
    n_cy: u32,
) -> Vec<u32> {
    let w = n_cx as usize;
    let h = n_cy as usize;
    let mut out = vec![0u32; w * h];
    for cy in 0..n_cy {
        for cx in 0..n_cx {
            let id = ChunkId::of_cell(i64::from(cx), i64::from(cy));
            out[cy as usize * w + cx as usize] = exact_tree_count_chunk(chunks, &id);
        }
    }
    out
}

/// Sum texels for the given draw_ids (Class R gate vs [`exact_tree_count`]).
#[must_use]
pub fn density_texel_sum_for_draw_ids(grid: &[u32], n_cx: u32, draw_ids: &[ChunkId]) -> u64 {
    let w = n_cx as usize;
    let mut sum = 0u64;
    for id in draw_ids {
        let Some((cx, cy)) = parse_chunk_xy(id) else {
            continue;
        };
        if cx < 0 || cy < 0 {
            continue;
        }
        let ux = cx as usize;
        let uy = cy as usize;
        if let Some(&v) = grid.get(uy * w + ux) {
            sum += u64::from(v);
        }
    }
    sum
}

/// Chunk grid dimensions for a terrain extent + chunk size (ceil division).
#[must_use]
pub fn density_grid_dims(terrain_w: f64, terrain_h: f64, chunk_size_m: f64) -> (u32, u32) {
    let n_cx = (terrain_w / chunk_size_m).ceil().max(1.0) as u32;
    let n_cy = (terrain_h / chunk_size_m).ceil().max(1.0) as u32;
    (n_cx, n_cy)
}

#[cfg(test)]
#[path = "tests/canopy_tests.rs"]
mod tests;
