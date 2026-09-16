//! Role: world.
//! Position: `spatial/indexing` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

use std::collections::HashMap;

use crate::spatial::indexing::point_index::PointIndex;

use crate::environment::classify::NO_CLASS;

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

    flat_x: Vec<f32>,
    flat_y: Vec<f32>,
    flat_cls: Vec<u8>,
    flat_id: Vec<String>,
}

#[inline]
fn class_allowed(cls: u8, mask: Option<u32>) -> bool {
    match mask {
        None => true,
        Some(bits) => cls < 32 && (bits >> cls) & 1 == 1,
    }
}

impl WorldSpatialIndex {
    /// New.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Bulk-insert one chunk's instances (idempotent — a chunk already present is replaced, like `insertChunk`'s leading `removeChunk`). `xs`/`ys`/`cls` are the chunk's compacted SoA columns (`positions` de-interleaved / `cls_codes`); rows with `cls == NO_CLASS` are skipped, and each kept row's id is `"{chunk_id}:{i}"` with `i` its position in the columns.
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

    /// All instance ids inside a world-meter bbox, optionally class-filtered. Corners are normalized first (`worldSpatialIndex.ts:100`).
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

    /// Nearest instance id within `radius_m` of `(x, y)`, optionally class-filtered, else `None`. Box-search then circular minimum — identical acceptance rule to `worldSpatialIndex.ts:82`.
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
#[path = "tests/world_tests.rs"]
mod tests;
