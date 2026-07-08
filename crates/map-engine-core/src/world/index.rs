//! Chunk-keyed world spatial index — port of `worldSpatialIndex.ts` (the rbush oracle). **Class
//! S**: `pick_rect`/`pick_nearest` return the same id set as the rbush, not a layout match. It
//! wraps [`crate::spatial::PointIndex`] (a uniform-grid CSR built over all resident rows) and adds
//! the two things the world domain needs that the bare `PointIndex` lacks: **per-chunk mutation**
//! (`insert_chunk`/`remove_chunk`, for streaming eviction) and a **render-class filter**.
//!
//! Query semantics mirror `worldSpatialIndex.ts:82-113` exactly:
//! - ids are `"{chunkId}:{row}"` where `row` is the compacted accepted-row index (matches
//!   `worldObjectsCore.indexChunk`'s `` `${chunk.id}:${i}` ``).
//! - radii are **world meters**; distance is `dx²+dy²` in f64 over the f32-stored coords.
//! - `pick_nearest` is **not** `PointIndex::pick_nearest` (which has no class filter and could
//!   return a rejected-class nearest): it box-searches `x±r`, filters by class, then takes the
//!   circular minimum with the same acceptance rule `d2 <= r2 && (best.is_none() || d2 < best)`.

use std::collections::HashMap;

use crate::spatial::point_index::PointIndex;

use super::classify::NO_CLASS;

/// Grid cell size for the internal `PointIndex` (meters). Correctness is cell-independent
/// (queries are exact for any positive cell); this only tunes bucket occupancy.
const INDEX_CELL_M: f64 = 256.0;

#[derive(Clone)]
struct StoredEntry {
    x: f32,
    y: f32,
    cls: u8,
    id: String,
}

/// Chunk-granular, class-filterable point index over world instances.
#[derive(Default)]
pub struct WorldSpatialIndex {
    by_chunk: HashMap<String, Vec<StoredEntry>>,
    dirty: bool,
    grid: Option<PointIndex>,
    // Flattened, index-aligned with `grid`'s handles (rebuilt from `by_chunk` in sorted id order).
    // `flat_x`/`flat_y` mirror the columns fed to `grid` so distances read the identical f32
    // values the grid indexed.
    flat_x: Vec<f32>,
    flat_y: Vec<f32>,
    flat_cls: Vec<u8>,
    flat_id: Vec<String>,
}

/// `mask = None` ⇒ all classes; `Some(bits)` ⇒ class code `c` allowed iff bit `c` is set.
#[inline]
fn class_allowed(cls: u8, mask: Option<u32>) -> bool {
    match mask {
        None => true,
        Some(bits) => cls < 32 && (bits >> cls) & 1 == 1,
    }
}

impl WorldSpatialIndex {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Bulk-insert one chunk's instances (idempotent — a chunk already present is replaced, like
    /// `insertChunk`'s leading `removeChunk`). `xs`/`ys`/`cls` are the chunk's compacted SoA
    /// columns (`positions` de-interleaved / `cls_codes`); rows with `cls == NO_CLASS` are skipped,
    /// and each kept row's id is `"{chunk_id}:{i}"` with `i` its position in the columns.
    ///
    /// # Panics
    /// Never — mismatched column lengths are clamped to the shortest.
    pub fn insert_chunk(&mut self, chunk_id: &str, xs: &[f32], ys: &[f32], cls: &[u8]) {
        self.remove_chunk(chunk_id);
        let n = xs.len().min(ys.len()).min(cls.len());
        let mut entries = Vec::new();
        for i in 0..n {
            if cls[i] == NO_CLASS {
                continue;
            }
            entries.push(StoredEntry {
                x: xs[i],
                y: ys[i],
                cls: cls[i],
                id: format!("{chunk_id}:{i}"),
            });
        }
        self.by_chunk.insert(chunk_id.to_string(), entries);
        self.dirty = true;
    }

    /// Remove a chunk's instances (LRU eviction / unload). Unknown chunk = no-op.
    pub fn remove_chunk(&mut self, chunk_id: &str) {
        if self.by_chunk.remove(chunk_id).is_some() {
            self.dirty = true;
        }
    }

    /// Drop everything.
    pub fn clear(&mut self) {
        self.by_chunk.clear();
        self.grid = None;
        self.flat_x.clear();
        self.flat_y.clear();
        self.flat_cls.clear();
        self.flat_id.clear();
        self.dirty = false;
    }

    /// Total indexed instances.
    #[must_use]
    pub fn size(&self) -> usize {
        self.by_chunk.values().map(Vec::len).sum()
    }

    /// Rebuild the flat columns + `PointIndex` when a mutation dirtied the index. Chunks are
    /// visited in **sorted id order** so handle assignment is deterministic (result sets are
    /// order-independent regardless).
    fn ensure_built(&mut self) {
        if !self.dirty && self.grid.is_some() {
            return;
        }
        let mut ids: Vec<&String> = self.by_chunk.keys().collect();
        ids.sort();
        self.flat_x.clear();
        self.flat_y.clear();
        self.flat_cls.clear();
        self.flat_id.clear();
        for id in ids {
            for e in &self.by_chunk[id] {
                self.flat_x.push(e.x);
                self.flat_y.push(e.y);
                self.flat_cls.push(e.cls);
                self.flat_id.push(e.id.clone());
            }
        }
        self.grid = Some(PointIndex::build(
            self.flat_x.clone(),
            self.flat_y.clone(),
            INDEX_CELL_M,
        ));
        self.dirty = false;
    }

    /// All instance ids inside a world-meter bbox, optionally class-filtered. Corners are
    /// normalized first (`worldSpatialIndex.ts:100`).
    pub fn pick_rect(
        &mut self,
        min_x: f64,
        min_y: f64,
        max_x: f64,
        max_y: f64,
        mask: Option<u32>,
    ) -> Vec<String> {
        self.ensure_built();
        let (lo_x, hi_x) = (min_x.min(max_x), min_x.max(max_x));
        let (lo_y, hi_y) = (min_y.min(max_y), min_y.max(max_y));
        let grid = self.grid.as_ref().expect("built");
        let mut out = Vec::new();
        for h in grid.pick_rect(lo_x, lo_y, hi_x, hi_y) {
            let h = h as usize;
            if class_allowed(self.flat_cls[h], mask) {
                out.push(self.flat_id[h].clone());
            }
        }
        out
    }

    /// Nearest instance id within `radius_m` of `(x, y)`, optionally class-filtered, else `None`.
    /// Box-search then circular minimum — identical acceptance rule to `worldSpatialIndex.ts:82`.
    pub fn pick_nearest(
        &mut self,
        x: f64,
        y: f64,
        radius_m: f64,
        mask: Option<u32>,
    ) -> Option<String> {
        self.ensure_built();
        let grid = self.grid.as_ref().expect("built");
        let r = radius_m.max(0.0);
        let r2 = r * r;
        let mut best: Option<(f64, usize)> = None;
        for h in grid.pick_rect(x - r, y - r, x + r, y + r) {
            let h = h as usize;
            if !class_allowed(self.flat_cls[h], mask) {
                continue;
            }
            let dx = f64::from(self.flat_x[h]) - x;
            let dy = f64::from(self.flat_y[h]) - y;
            let d2 = dx * dx + dy * dy;
            if d2 <= r2 && best.is_none_or(|(bd, _)| d2 < bd) {
                best = Some((d2, h));
            }
        }
        best.map(|(_, h)| self.flat_id[h].clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // building=0, tree=1 (RENDER_CLASS_CODES order).
    const B: u8 = 0;
    const T: u8 = 1;

    fn idx() -> WorldSpatialIndex {
        let mut ix = WorldSpatialIndex::new();
        // chunk "0_0": a building at (10,10) and a tree at (12,10).
        ix.insert_chunk("0_0", &[10.0, 12.0], &[10.0, 10.0], &[B, T]);
        // chunk "1_0": a building at (600,10).
        ix.insert_chunk("1_0", &[600.0], &[10.0], &[B]);
        ix
    }

    #[test]
    fn pick_rect_class_filter() {
        let mut ix = idx();
        let mut all = ix.pick_rect(0.0, 0.0, 20.0, 20.0, None);
        all.sort();
        assert_eq!(all, vec!["0_0:0", "0_0:1"]);
        // Buildings only (mask bit 0).
        let only_b = ix.pick_rect(0.0, 0.0, 20.0, 20.0, Some(1 << B));
        assert_eq!(only_b, vec!["0_0:0"]);
    }

    #[test]
    fn pick_nearest_circular_and_class() {
        let mut ix = idx();
        // (11,10) is 1 m from the building (10,10) and 1 m from the tree (12,10) — tie broken by
        // the box-scan; use an off-center probe to avoid the tie.
        assert_eq!(ix.pick_nearest(10.4, 10.0, 5.0, None), Some("0_0:0".into()));
        // Class filter: force the tree even though a building is nearer.
        assert_eq!(
            ix.pick_nearest(10.4, 10.0, 5.0, Some(1 << T)),
            Some("0_0:1".into())
        );
        // Radius too small → None (nearest building is 0.4 m away, radius 0.1 excludes it).
        assert_eq!(ix.pick_nearest(10.4, 10.0, 0.1, None), None);
    }

    #[test]
    fn no_class_rows_are_skipped() {
        let mut ix = WorldSpatialIndex::new();
        ix.insert_chunk("0_0", &[1.0, 2.0], &[1.0, 2.0], &[B, NO_CLASS]);
        assert_eq!(ix.size(), 1);
        assert_eq!(ix.pick_rect(0.0, 0.0, 10.0, 10.0, None), vec!["0_0:0"]);
    }

    #[test]
    fn remove_and_reinsert_are_idempotent() {
        let mut ix = idx();
        assert_eq!(ix.size(), 3);
        ix.remove_chunk("0_0");
        assert_eq!(ix.size(), 1);
        assert!(ix.pick_rect(0.0, 0.0, 20.0, 20.0, None).is_empty());
        // Re-insert the same chunk twice: no duplication.
        ix.insert_chunk("1_0", &[600.0], &[10.0], &[B]);
        assert_eq!(ix.size(), 1);
    }
}
