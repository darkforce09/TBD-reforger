// Fixed world-grid chunking (T-067.0). Partition a terrain's flat-meter bounds into a
// CHUNK_SIZE_M grid so the detail IconLayer can feed Deck only the slots in the visible
// chunks (plus a halo ring + the current selection) once a mission crosses
// CHUNK_CULL_THRESHOLD (slotIconCache.getBaseIconsForBbox). Pure math — no Y.Doc / store
// dependency; the icon cache owns chunk membership. NOTE: these bins are GEOGRAPHIC (viewport
// culling), NOT the T-062.1 IDB iteration-order PERSIST_CHUNK_SIZE batches.

import type { TerrainDef } from '../coords/terrains'

/** Chunk edge in world meters. Everon 12800m → 25×25 grid; Arland 10240m → 20×20. */
export const CHUNK_SIZE_M = 512

/** Adjacent-chunk ring drawn around the viewport bbox so panning never reveals an
 *  un-culled hole at a chunk seam before the next frame's bbox recompute. */
export const CHUNK_HALO = 1

function clampInt(v: number, min: number, max: number): number {
  return v < min ? min : v > max ? max : v
}

/** Columns across the terrain (ceil so a partial trailing chunk still has a column). */
export function terrainChunkCols(terrain: TerrainDef): number {
  return Math.max(1, Math.ceil(terrain.width / CHUNK_SIZE_M))
}

export function terrainChunkRows(terrain: TerrainDef): number {
  return Math.max(1, Math.ceil(terrain.height / CHUNK_SIZE_M))
}

/** World meters → clamped [col, row] chunk indices. */
export function chunkColRow(x: number, y: number, terrain: TerrainDef): [number, number] {
  const cols = terrainChunkCols(terrain)
  const rows = terrainChunkRows(terrain)
  return [
    clampInt(Math.floor(x / CHUNK_SIZE_M), 0, cols - 1),
    clampInt(Math.floor(y / CHUNK_SIZE_M), 0, rows - 1),
  ]
}

/** Flat chunk key. Single integer so membership lives in a plain Map (no string alloc). */
export function chunkKey(cx: number, cy: number, cols: number): number {
  return cy * cols + cx
}

/** Chunk-index rectangle covering a world bbox, expanded by `halo` chunks and clamped to the
 *  grid. The visible region is always a rectangle, so callers can compare these four ints to
 *  detect "same chunks as last frame" (stable IconLayer identity on intra-chunk pans). */
export function chunkRectForBbox(
  bbox: [number, number, number, number],
  terrain: TerrainDef,
  halo: number,
): [number, number, number, number] {
  const cols = terrainChunkCols(terrain)
  const rows = terrainChunkRows(terrain)
  const [cx0, cy0] = chunkColRow(bbox[0], bbox[1], terrain)
  const [cx1, cy1] = chunkColRow(bbox[2], bbox[3], terrain)
  return [
    clampInt(Math.min(cx0, cx1) - halo, 0, cols - 1),
    clampInt(Math.min(cy0, cy1) - halo, 0, rows - 1),
    clampInt(Math.max(cx0, cx1) + halo, 0, cols - 1),
    clampInt(Math.max(cy0, cy1) + halo, 0, rows - 1),
  ]
}

/** Set of chunk keys covering a world bbox + halo. */
export function chunkKeysForBbox(
  bbox: [number, number, number, number],
  terrain: TerrainDef,
  halo: number,
): Set<number> {
  const cols = terrainChunkCols(terrain)
  const [rcx0, rcy0, rcx1, rcy1] = chunkRectForBbox(bbox, terrain, halo)
  const out = new Set<number>()
  for (let cy = rcy0; cy <= rcy1; cy++) {
    for (let cx = rcx0; cx <= rcx1; cx++) out.add(chunkKey(cx, cy, cols))
  }
  return out
}
