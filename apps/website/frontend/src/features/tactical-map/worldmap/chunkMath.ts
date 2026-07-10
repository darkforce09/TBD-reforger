// ORACLE-ONLY (T-151.11.3, audit B-03): the live paths call wasm `chunk_ids_for_viewport`
// / `WorldResidency.set_viewport`; this file feeds parity/unit tests only.
// T-090.5.1 — Export-chunk math for Map Engine v2 streaming (implementation plan §6). Pure:
// no React/Deck/fetch, node-testable; the worker (T-090.5.3) and chunkStore consume it.
//
// Chunk key contract (T-090.3.1 export): (cx, cy) = floor(x/512), floor(y/512) in world meters,
// file names `objects/chunks/{cx}_{cy}.json.gz` + `objects/density/{cx}_{cy}.bin`. The runtime
// authority for the cell size is manifest `objects.chunkSizeM`; every function takes an optional
// override and defaults to 512. NOTE: state/spatialChunks.ts also bins at 512 m — that grid culls
// mission SLOT icons (T-067) and is a separate domain; do not merge the two.

/** Default export chunk edge in world meters (= manifest `objects.chunkSizeM` today). */
export const DEFAULT_CHUNK_SIZE_M = 512

/** World bbox [minX, minY, maxX, maxY] in meters. */
export type Bbox = [number, number, number, number]

/** Inclusive chunk-index rectangle. */
export interface ChunkRect {
  cx0: number
  cy0: number
  cx1: number
  cy1: number
}

/** Terrain extent in world meters (Everon 12800×12800). */
export interface TerrainSizeM {
  width: number
  height: number
}

const clampInt = (v: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, v))

/** Export-artifact id for a chunk — matches the on-disk `{cx}_{cy}` file stem. */
export function chunkId(cx: number, cy: number): string {
  return `${cx}_${cy}`
}

/** Border-preload margin in meters (plan §6, A3 PreloadMapObjects analogue):
 *  max(5% of the larger viewport span, one chunk ring). */
export function preloadMarginM(bbox: Bbox, chunkSizeM = DEFAULT_CHUNK_SIZE_M): number {
  const span = Math.max(bbox[2] - bbox[0], bbox[3] - bbox[1])
  return Math.max(0.05 * span, chunkSizeM)
}

/** Expand a bbox by a symmetric margin (no clamping — chunk conversion clamps). */
export function expandBbox(bbox: Bbox, marginM: number): Bbox {
  return [bbox[0] - marginM, bbox[1] - marginM, bbox[2] + marginM, bbox[3] + marginM]
}

/** Bbox → inclusive chunk rect, clamped to the terrain grid so we never request ids that
 *  cannot exist on disk. Degenerate/edge coordinates clamp to the last row/column. */
export function chunkRectForBbox(
  bbox: Bbox,
  terrain: TerrainSizeM,
  chunkSizeM = DEFAULT_CHUNK_SIZE_M,
): ChunkRect {
  const maxCx = Math.max(0, Math.ceil(terrain.width / chunkSizeM) - 1)
  const maxCy = Math.max(0, Math.ceil(terrain.height / chunkSizeM) - 1)
  return {
    cx0: clampInt(Math.floor(Math.min(bbox[0], bbox[2]) / chunkSizeM), 0, maxCx),
    cy0: clampInt(Math.floor(Math.min(bbox[1], bbox[3]) / chunkSizeM), 0, maxCy),
    cx1: clampInt(Math.floor(Math.max(bbox[0], bbox[2]) / chunkSizeM), 0, maxCx),
    cy1: clampInt(Math.floor(Math.max(bbox[1], bbox[3]) / chunkSizeM), 0, maxCy),
  }
}

/** Grow a chunk rect by `ring` chunks on every side, clamped to the terrain grid. Used for the
 *  oversized-object +1 ring (plan §6: runway/pier/powerline spans exceed their home chunk). */
export function expandChunkRect(
  rect: ChunkRect,
  ring: number,
  terrain: TerrainSizeM,
  chunkSizeM = DEFAULT_CHUNK_SIZE_M,
): ChunkRect {
  const maxCx = Math.max(0, Math.ceil(terrain.width / chunkSizeM) - 1)
  const maxCy = Math.max(0, Math.ceil(terrain.height / chunkSizeM) - 1)
  return {
    cx0: clampInt(rect.cx0 - ring, 0, maxCx),
    cy0: clampInt(rect.cy0 - ring, 0, maxCy),
    cx1: clampInt(rect.cx1 + ring, 0, maxCx),
    cy1: clampInt(rect.cy1 + ring, 0, maxCy),
  }
}

/** Enumerate a rect's chunk ids row-major (stable order → stable fetch/dedupe keys). */
export function chunkIdsForRect(rect: ChunkRect): string[] {
  const ids: string[] = []
  for (let cy = rect.cy0; cy <= rect.cy1; cy++) {
    for (let cx = rect.cx0; cx <= rect.cx1; cx++) ids.push(chunkId(cx, cy))
  }
  return ids
}

/** Viewport bbox → chunk ids to hydrate: bbox grown by the preload margin (§6 border rule),
 *  optionally + `extraRing` chunks (oversized-object classes), clamped to the terrain. */
export function chunkIdsForViewport(
  bbox: Bbox,
  terrain: TerrainSizeM,
  opts?: { chunkSizeM?: number; extraRing?: number },
): string[] {
  const chunkSizeM = opts?.chunkSizeM ?? DEFAULT_CHUNK_SIZE_M
  const preloaded = expandBbox(bbox, preloadMarginM(bbox, chunkSizeM))
  let rect = chunkRectForBbox(preloaded, terrain, chunkSizeM)
  if (opts?.extraRing) rect = expandChunkRect(rect, opts.extraRing, terrain, chunkSizeM)
  return chunkIdsForRect(rect)
}
