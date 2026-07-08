//! Viewport → chunk-id set math — verbatim port of `chunkMath.ts`. **Class R**: every op is
//! deterministic IEEE-754 f64 (`0.05` is the same literal in both languages; `f64::floor`/`ceil`/
//! `max`/`min` agree bit-for-bit with `Math.*` on the finite inputs here), and the id strings are
//! exact. The rect integers are computed in f64 (to match `Math.floor`/`clampInt`) then cast to
//! `i64` for iteration/formatting — the values are whole and non-negative so `as i64` is lossless.
//!
//! Chunk ids are `format!("{cx}_{cy}")` with `i64` components, identical to `narrow_cells`
//! (`manifest.rs`) so the viewport set intersects the export cell set correctly. The default chunk
//! size lives in `manifest::DEFAULT_CHUNK_SIZE_M`; callers pass the manifest's `chunkSizeM`.

/// A world bbox `[minX, minY, maxX, maxY]` in meters (the `chunkMath.ts` `Bbox`).
pub type Bbox = [f64; 4];

/// Terrain extent in meters (`chunkMath.ts` `TerrainSizeM`). `Default` = `0×0` (an unconfigured
/// residency then resolves every viewport to the empty set until a manifest sets the bounds).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TerrainSizeM {
    pub width: f64,
    pub height: f64,
}

/// Inclusive chunk-index rect (`chunkMath.ts` `ChunkRect`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChunkRect {
    pub cx0: i64,
    pub cy0: i64,
    pub cx1: i64,
    pub cy1: i64,
}

/// `chunkId(cx, cy)` — `` `${cx}_${cy}` ``.
#[must_use]
pub fn chunk_id(cx: i64, cy: i64) -> String {
    format!("{cx}_{cy}")
}

/// `clampInt(v, lo, hi) = Math.max(lo, Math.min(hi, v))` — kept in f64 (v is a floored whole).
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

/// `chunkRectForBbox(bbox, terrain, chunkSizeM)`. `maxC = max(0, ceil(dim/size) - 1)`; each edge
/// = `clampInt(floor(coord/size), 0, maxC)` with `coord = min|max(bbox edges)`.
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

/// `expandChunkRect(rect, ring, terrain, chunkSizeM)` — grow the rect by `ring` chunks per edge,
/// each edge `clampInt(_, 0, maxC)`.
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

/// `chunkIdsForViewport(bbox, terrain, { chunkSizeM, extraRing })`. `extra_ring = 0` skips the
/// oversized expansion (the JS `opts.extraRing` default).
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
mod tests {
    use super::*;

    const EVERON: TerrainSizeM = TerrainSizeM {
        width: 12800.0,
        height: 12800.0,
    };
    const CHUNK: f64 = 512.0;

    /// Class R vs the pinned `chunkMath.test.ts` rect values.
    #[test]
    fn chunk_rect_pinned_cases() {
        assert_eq!(
            chunk_rect_for_bbox([0.0, 0.0, 511.9, 511.9], EVERON, CHUNK),
            ChunkRect {
                cx0: 0,
                cy0: 0,
                cx1: 0,
                cy1: 0
            }
        );
        assert_eq!(
            chunk_rect_for_bbox([512.0, 512.0, 1024.0, 1024.0], EVERON, CHUNK),
            ChunkRect {
                cx0: 1,
                cy0: 1,
                cx1: 2,
                cy1: 2
            }
        );
        // Past the east/south edge → clamped to the last cell (24 = ceil(12800/512)-1).
        assert_eq!(
            chunk_rect_for_bbox([12799.0, 12799.0, 99999.0, 99999.0], EVERON, CHUNK),
            ChunkRect {
                cx0: 24,
                cy0: 24,
                cx1: 24,
                cy1: 24
            }
        );
    }

    /// Class R vs the pinned `chunkMath.test.ts` margin values.
    #[test]
    fn preload_margin_pinned_cases() {
        assert_eq!(preload_margin_m([0.0, 0.0, 2000.0, 2000.0], CHUNK), 512.0);
        assert_eq!(preload_margin_m([0.0, 0.0, 12800.0, 12800.0], CHUNK), 640.0);
    }

    /// `chunkIdsForViewport([1024,1024,1536,1536])` covers chunks 1..4 on both axes = 16 ids,
    /// row-major.
    #[test]
    fn viewport_ids_length_and_order() {
        let ids = chunk_ids_for_viewport([1024.0, 1024.0, 1536.0, 1536.0], EVERON, CHUNK, 0);
        assert_eq!(ids.len(), 16);
        assert_eq!(ids[0], "1_1");
        assert_eq!(ids[1], "2_1"); // cx inner
        assert_eq!(ids[4], "1_2"); // cy outer wraps after 4 cx
        assert_eq!(ids[15], "4_4");
    }

    /// The oversized ring adds one chunk on every edge.
    #[test]
    fn oversized_ring_expands_rect() {
        let rect = chunk_rect_for_bbox([2048.0, 2048.0, 2048.0, 2048.0], EVERON, CHUNK);
        assert_eq!(
            rect,
            ChunkRect {
                cx0: 4,
                cy0: 4,
                cx1: 4,
                cy1: 4
            }
        );
        let ringed = expand_chunk_rect(rect, 1, EVERON, CHUNK);
        assert_eq!(
            ringed,
            ChunkRect {
                cx0: 3,
                cy0: 3,
                cx1: 5,
                cy1: 5
            }
        );
    }

    /// Row-major id emission (cy outer, cx inner).
    #[test]
    fn ids_for_rect_row_major() {
        let ids = chunk_ids_for_rect(ChunkRect {
            cx0: 0,
            cy0: 0,
            cx1: 1,
            cy1: 1,
        });
        assert_eq!(ids, vec!["0_0", "1_0", "0_1", "1_1"]);
    }
}
