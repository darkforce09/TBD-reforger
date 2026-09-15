//! Role: chunk math.
//! Position: `streaming/scheduler` in the graphics engine.
//! Signals & state: camera, spatial, asset, or GPU data owned by this module.
//! Invariants: preserve coordinates, resource lifetimes, ordering, and binary layouts.

/// A world bbox `[minX, minY, maxX, maxY]` in meters (the `chunkMath.ts` `Bbox`).
pub type Bbox = [f64; 4];

/// Terrain extent in meters (`chunkMath.ts` `TerrainSizeM`). `Default` = `0×0` (an unconfigured residency then resolves every viewport to the empty set until a manifest sets the bounds).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TerrainSizeM {
    /// Width.
    pub width: f64,

    /// Height.
    pub height: f64,
}

/// Inclusive chunk-index rect (`chunkMath.ts` `ChunkRect`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChunkRect {
    /// Cx0.
    pub cx0: i64,

    /// Cy0.
    pub cy0: i64,

    /// Cx1.
    pub cx1: i64,

    /// Cy1.
    pub cy1: i64,
}

/// `chunkId(cx, cy)` — `` `${cx}_${cy}` ``.
#[must_use]
pub fn chunk_id(cx: i64, cy: i64) -> String {
    format!("{cx}_{cy}")
}

#[inline]
fn clamp_f(v: f64, lo: f64, hi: f64) -> f64 {
    lo.max(hi.min(v))
}

/// `preloadMarginM(bbox, chunkSizeM)` — `max(0.05 * span, chunkSizeM)`, `span = max(w, h)`.
#[must_use]
pub fn preload_margin_m(bbox: Bbox, chunk_size_m: f64) -> f64 {
    let span = (bbox[2] - bbox[0]).max(bbox[3] - bbox[1]);
    (0.05 * span).max(chunk_size_m)
}

/// `expandBbox(bbox, marginM)` — grow every edge outward by `margin_m` (no clamp).
#[must_use]
pub fn expand_bbox(bbox: Bbox, margin_m: f64) -> Bbox {
    [
        bbox[0] - margin_m,
        bbox[1] - margin_m,
        bbox[2] + margin_m,
        bbox[3] + margin_m,
    ]
}

/// `chunkRectForBbox(bbox, terrain, chunkSizeM)`. `maxC = max(0, ceil(dim/size) - 1)`; each edge = `clampInt(floor(coord/size), 0, maxC)` with `coord = min|max(bbox edges)`.
#[must_use]
pub fn chunk_rect_for_bbox(bbox: Bbox, terrain: TerrainSizeM, chunk_size_m: f64) -> ChunkRect {
    let max_cx = 0.0_f64.max((terrain.width / chunk_size_m).ceil() - 1.0);
    let max_cy = 0.0_f64.max((terrain.height / chunk_size_m).ceil() - 1.0);
    let cx0 = clamp_f((bbox[0].min(bbox[2]) / chunk_size_m).floor(), 0.0, max_cx);
    let cy0 = clamp_f((bbox[1].min(bbox[3]) / chunk_size_m).floor(), 0.0, max_cy);
    let cx1 = clamp_f((bbox[0].max(bbox[2]) / chunk_size_m).floor(), 0.0, max_cx);
    let cy1 = clamp_f((bbox[1].max(bbox[3]) / chunk_size_m).floor(), 0.0, max_cy);
    ChunkRect {
        cx0: cx0 as i64,
        cy0: cy0 as i64,
        cx1: cx1 as i64,
        cy1: cy1 as i64,
    }
}

/// `expandChunkRect(rect, ring, terrain, chunkSizeM)` — grow the rect by `ring` chunks per edge, each edge `clampInt(_, 0, maxC)`.
#[must_use]
pub fn expand_chunk_rect(
    rect: ChunkRect,
    ring: i64,
    terrain: TerrainSizeM,
    chunk_size_m: f64,
) -> ChunkRect {
    let max_cx = 0.0_f64.max((terrain.width / chunk_size_m).ceil() - 1.0) as i64;
    let max_cy = 0.0_f64.max((terrain.height / chunk_size_m).ceil() - 1.0) as i64;
    ChunkRect {
        cx0: (rect.cx0 - ring).clamp(0, max_cx),
        cy0: (rect.cy0 - ring).clamp(0, max_cy),
        cx1: (rect.cx1 + ring).clamp(0, max_cx),
        cy1: (rect.cy1 + ring).clamp(0, max_cy),
    }
}

/// `chunkIdsForRect(rect)` — **row-major, cy outer, cx inner** (the fetch/dedupe/pin order).
#[must_use]
pub fn chunk_ids_for_rect(rect: ChunkRect) -> Vec<String> {
    let mut out = Vec::new();
    for cy in rect.cy0..=rect.cy1 {
        for cx in rect.cx0..=rect.cx1 {
            out.push(chunk_id(cx, cy));
        }
    }
    out
}

/// `chunkIdsForViewport(bbox, terrain, { chunkSizeM, extraRing })`. `extra_ring = 0` skips the oversized expansion (the JS `opts.extraRing` default).
#[must_use]
pub fn chunk_ids_for_viewport(
    bbox: Bbox,
    terrain: TerrainSizeM,
    chunk_size_m: f64,
    extra_ring: i64,
) -> Vec<String> {
    let preloaded = expand_bbox(bbox, preload_margin_m(bbox, chunk_size_m));
    let mut rect = chunk_rect_for_bbox(preloaded, terrain, chunk_size_m);
    if extra_ring > 0 {
        rect = expand_chunk_rect(rect, extra_ring, terrain, chunk_size_m);
    }
    chunk_ids_for_rect(rect)
}

#[cfg(test)]
#[path = "tests/chunk_math_tests.rs"]
mod tests;
