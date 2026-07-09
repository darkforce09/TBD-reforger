//! T-151.8 — exact-count density ladder (Class R integers only).
//!
//! Switch predicate and per-chunk count grid for the tree heatmap rung.
//! TBDD corner sums are **not** used here (they are not instance counts).

use super::chunk::WorldChunk;
use super::classify::class_code;
use super::lod_gates::INSTANCE_BUDGET;
use std::collections::HashMap;

/// Parse `"cx_cy"` chunk id → `(cx, cy)`. Returns `None` on malformed ids.
#[must_use]
pub fn parse_chunk_xy(id: &str) -> Option<(i64, i64)> {
    let (a, b) = id.split_once('_')?;
    Some((a.parse().ok()?, b.parse().ok()?))
}

/// Exact tree+vegetation instance count over `draw_ids` (Class R — sum of row lens).
#[must_use]
pub fn exact_tree_count(chunks: &HashMap<String, WorldChunk>, draw_ids: &[String]) -> usize {
    let tree_code = class_code("tree");
    let veg_code = class_code("vegetation");
    let mut n = 0usize;
    for id in draw_ids {
        let Some(chunk) = chunks.get(id) else {
            continue;
        };
        if let Some(rows) = chunk.rows_by_class.get(&tree_code) {
            n += rows.len();
        }
        if let Some(rows) = chunk.rows_by_class.get(&veg_code) {
            n += rows.len();
        }
    }
    n
}

/// Exact tree+veg count for one chunk id (0 if missing).
#[must_use]
pub fn exact_tree_count_chunk(chunks: &HashMap<String, WorldChunk>, id: &str) -> u32 {
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

/// Pack an `n_cx × n_cy` R32Uint grid (row-major, cy outer, cx inner — matches chunk_ids_for_rect).
/// Texel `(cx, cy)` = exact tree+veg count for `{cx}_{cy}` among `resident` chunk map; else 0.
#[must_use]
pub fn pack_density_grid_r32(
    chunks: &HashMap<String, WorldChunk>,
    n_cx: u32,
    n_cy: u32,
) -> Vec<u32> {
    let w = n_cx as usize;
    let h = n_cy as usize;
    let mut out = vec![0u32; w * h];
    for cy in 0..h {
        for cx in 0..w {
            let id = format!("{cx}_{cy}");
            out[cy * w + cx] = exact_tree_count_chunk(chunks, &id);
        }
    }
    out
}

/// Sum texels for the given draw_ids (Class R gate vs [`exact_tree_count`]).
#[must_use]
pub fn density_texel_sum_for_draw_ids(grid: &[u32], n_cx: u32, draw_ids: &[String]) -> u64 {
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
mod tests {
    use super::*;
    use crate::world::chunk::WorldChunk;
    use crate::world::classify::class_code;

    fn chunk_with_tree_rows(id: &str, n: usize) -> WorldChunk {
        let mut c = WorldChunk {
            id: id.to_string(),
            count: n as u32,
            positions: vec![0.0; n * 2],
            prefab_idx: vec![0; n],
            rotations: vec![0.0; n],
            z: vec![0.0; n],
            cls_codes: vec![class_code("tree"); n],
            rows_by_class: HashMap::new(),
            ..Default::default()
        };
        c.rows_by_class
            .insert(class_code("tree"), (0..n as u32).collect::<Vec<_>>());
        c
    }

    #[test]
    fn r1_exact_tree_count_hand_sum() {
        let mut chunks = HashMap::new();
        chunks.insert("1_1".into(), chunk_with_tree_rows("1_1", 10));
        chunks.insert("1_2".into(), chunk_with_tree_rows("1_2", 7));
        let draw = vec!["1_1".into(), "1_2".into()];
        assert_eq!(exact_tree_count(&chunks, &draw), 17);
    }

    #[test]
    fn r2_heatmap_boundary() {
        assert!(!heatmap_trees(INSTANCE_BUDGET));
        assert!(heatmap_trees(INSTANCE_BUDGET + 1));
        assert!(!heatmap_trees(0));
    }

    #[test]
    fn r3_texel_sum_equals_exact() {
        let mut chunks = HashMap::new();
        chunks.insert("0_0".into(), chunk_with_tree_rows("0_0", 3));
        chunks.insert("1_0".into(), chunk_with_tree_rows("1_0", 5));
        chunks.insert("0_1".into(), chunk_with_tree_rows("0_1", 2));
        let grid = pack_density_grid_r32(&chunks, 2, 2);
        let draw = vec!["0_0".into(), "1_0".into()];
        let exact = exact_tree_count(&chunks, &draw) as u64;
        assert_eq!(density_texel_sum_for_draw_ids(&grid, 2, &draw), exact);
        assert_eq!(exact, 8);
    }

    #[test]
    fn everon_grid_dims() {
        assert_eq!(density_grid_dims(12800.0, 12800.0, 512.0), (25, 25));
    }
}
