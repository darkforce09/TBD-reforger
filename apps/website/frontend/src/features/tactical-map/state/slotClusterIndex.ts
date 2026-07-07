// Cluster / LOD index for the zoomed-out map (T-065) — a Rust cluster hierarchy (T-145 Phase 3.1)
// that runs parallel to the dense `slotIconCache` and the picking `slotSpatialIndex`. When the camera
// is far enough out that individual icons are invisible dots (zoom < ZOOM_DETAIL_MIN) and the mission
// is large (slotCount > CLUSTER_SLOT_THRESHOLD), the map draws cluster bubbles + a count from this
// index instead of all ~367k IconLayer markers — Eden's "group icon stacked when zoomed out". Zoom
// back in and the detail IconLayer takes over (T-061/T-063 unchanged).
//
// The clustering itself lives in `map-engine-core::spatial::cluster` (wasm `ClusterIndex`) — a
// supercluster-compatible hierarchy (same linear world→lng/lat normalization + fround(mercator)
// projection + greedy radius clustering), pinned set-equal to `supercluster` by
// `features/_wasm/cluster.parity.test.ts`. This module keeps the incremental point bookkeeping (so
// edits stay O(k)) and rebuilds the wasm index lazily when a query needs it:
//
//   * world x/y ride in parallel `xs`/`ys` columns + an `id` table (row = handle); the wasm
//     `ClusterIndex` normalizes/projects internally and returns WORLD-meter clusters directly.
//   * the index is built ONCE (all zoom levels) and queried at any zoom; only an EDIT (`dirty`)
//     rebuilds it — never a pan/zoom (getClusters at min-zoom only queries).
//
// Module-level singleton: safe under the single-mounted-doc invariant (same as slotIconCache /
// slotSpatialIndex / the LOCAL_ORIGIN singletons elsewhere in the engine).

import * as wasm from '@/wasm/pkg/map_engine_wasm'
import type { TerrainDef } from '../coords/terrains'
import type { ID } from './schema'
import type { SlotIcon } from './selectors'

/** Minimal viewport surface (mirror of slotSpatialIndex's) — keeps Deck out of this module. */
interface Viewport {
  unproject: (xy: number[]) => number[]
}

/** A cluster bubble (or a lone leaf) in WORLD meters, ready for the cluster layers. */
export interface ClusterMarker {
  x: number
  y: number
  /** Aggregated slot count (1 for a lone leaf). */
  count: number
  /** Leaf slot id when count === 1, else null. */
  id: ID | null
}

let terrainW = 12800
let terrainH = 12800

// Incremental point bookkeeping (row = handle): world columns + a row→id table + id→row map.
const xs: number[] = []
const ys: number[] = []
const rowIds: ID[] = []
const index = new Map<ID, number>()
let ci: wasm.ClusterIndex | null = null
let dirty = true

// Pan-stable cluster cache (T-065.2) — mirrors slotIconCache.getBaseIcons + iconCacheVersion. The
// cluster render path must NOT re-query per pan frame (the T-061 contract: layer data unchanged on
// pan, deck transforms the view only). So we cache the full-terrain cluster set keyed on the
// supercluster zoom bucket; it recomputes ONLY when an edit dirties it or the zoom bucket changes,
// and bumps `markersVersion` so the layer hook rebuilds exactly then.
let cachedMarkers: ClusterMarker[] = []
let cachedSuperZoom: number | null = null
let markersDirty = true
let markersVersion = 0

/** Map the continuous Deck zoom (band [-6, 6]) to an integer supercluster zoom level. Cluster mode
 *  is only active at/below ZOOM_CLUSTER_MAX (-4), so the live range is [-6, -4] → scz [2, 4]. */
export function deckZoomToSuperZoom(deckZoom: number): number {
  const z = Math.round(deckZoom + 8)
  return z < 0 ? 0 : z > 16 ? 16 : z
}

/** Capture the active terrain so the wasm normalization matches it (called on terrain change). Marks
 *  the index dirty — the next query rebuilds against the new bounds (cheap; only on a terrain switch). */
export function setTerrain(terrain: TerrainDef): void {
  if (terrain.width === terrainW && terrain.height === terrainH) return
  terrainW = terrain.width
  terrainH = terrain.height
  dirty = true
  markersDirty = true
}

/** O(n) full rebuild — on a full snapshot replace. Mirrors slotSpatialIndex.rebuild. */
export function rebuild(icons: SlotIcon[]): void {
  xs.length = 0
  ys.length = 0
  rowIds.length = 0
  index.clear()
  for (let i = 0; i < icons.length; i++) {
    index.set(icons[i].id, i)
    xs.push(icons[i].x)
    ys.push(icons[i].y)
    rowIds.push(icons[i].id)
  }
  dirty = true
  markersDirty = true
}

/** O(k) insert newly-placed icons (asset drop / paste). Ids already present are skipped. */
export function insert(icons: { id: ID; x: number; y: number }[]): void {
  for (const s of icons) {
    if (index.has(s.id)) continue
    index.set(s.id, xs.length)
    xs.push(s.x)
    ys.push(s.y)
    rowIds.push(s.id)
  }
  dirty = true
}

/** O(k) remove ids via swap-and-pop (mirror of slotIconCache.remove). Ids not present skipped. */
export function remove(ids: ID[]): void {
  for (const id of ids) {
    const i = index.get(id)
    if (i === undefined) continue
    const last = xs.length - 1
    if (i !== last) {
      xs[i] = xs[last]
      ys[i] = ys[last]
      const movedId = rowIds[last]
      rowIds[i] = movedId
      index.set(movedId, i)
    }
    xs.pop()
    ys.pop()
    rowIds.pop()
    index.delete(id)
  }
  dirty = true
  markersDirty = true
}

/** O(k) reposition: rewrite the world columns in place. */
export function updatePositions(patches: Record<ID, { x: number; y: number }>): void {
  for (const id in patches) {
    const i = index.get(id)
    if (i === undefined) continue
    xs[i] = patches[id].x
    ys[i] = patches[id].y
  }
  dirty = true
  markersDirty = true
}

/** Drop everything (store reset / doc unmount). */
export function clear(): void {
  xs.length = 0
  ys.length = 0
  rowIds.length = 0
  index.clear()
  ci?.free()
  ci = null
  dirty = true
  cachedMarkers = []
  cachedSuperZoom = null
  markersDirty = true
}

/** Build the wasm cluster hierarchy once; reused across zooms. Rebuilds only when an edit dirtied
 *  the point set (never on pan/zoom — those only query). */
function ensureIndex(): wasm.ClusterIndex {
  if (dirty || !ci) {
    ci?.free()
    ci = new wasm.ClusterIndex(new Float32Array(xs), new Float32Array(ys), terrainW, terrainH)
    dirty = false
  }
  return ci
}

/** Cluster markers in WORLD meters for a viewport bbox (world meters) at a Deck zoom. Lazily
 *  (re)builds the wasm index only when dirty — a pan/zoom in cluster mode only ever queries. */
export function getClusters(
  bbox: [number, number, number, number],
  deckZoom: number,
): ClusterMarker[] {
  if (!xs.length) return []
  const idx = ensureIndex()
  const [minX, minY, maxX, maxY] = bbox
  const res = idx.get_clusters(minX, minY, maxX, maxY, deckZoom)
  // Copy the columns out, then free the wasm result handle (kickoff gotcha: free after clone-out).
  const cxs = res.xs
  const cys = res.ys
  const counts = res.counts
  const leaves = res.leaves
  res.free()
  const out: ClusterMarker[] = new Array(counts.length)
  for (let i = 0; i < counts.length; i++) {
    const leaf = leaves[i]
    out[i] = { x: cxs[i], y: cys[i], count: counts[i], id: leaf < 0 ? null : rowIds[leaf] }
  }
  return out
}

/** Pan-stable cluster markers for the render layer (T-065.2). Queries the FULL terrain (not the
 *  viewport), so panning never changes the result — recomputes only when an edit dirtied the set or
 *  the supercluster zoom bucket changed, bumping `markersVersion`. Lazy: `ensureIndex()` runs on the
 *  first cluster-band read, never during detail-mode editing. */
export function getClusterMarkers(deckZoom: number): ClusterMarker[] {
  const z = deckZoomToSuperZoom(deckZoom)
  if (markersDirty || z !== cachedSuperZoom) {
    cachedMarkers = getClusters([0, 0, terrainW, terrainH], deckZoom)
    cachedSuperZoom = z
    markersDirty = false
    markersVersion++
  }
  return cachedMarkers
}

/** Bumps whenever `getClusterMarkers` recomputes (zoom-bucket change or edit). The layer hook
 *  subscribes this to rebuild exactly then — mirrors slotIconCache.getVersion / iconCacheVersion. */
export function getClusterMarkersVersion(): number {
  return markersVersion
}

/** Nearest cluster/leaf to a screen-pixel click within `radiusPx` (world-projected), for the
 *  cluster drill-in. Returns the marker's WORLD centroid, else null. */
export function pickClusterAt(
  px: [number, number],
  viewport: Viewport,
  deckZoom: number,
  radiusPx = 48,
): ClusterMarker | null {
  if (!xs.length) return null
  const center = viewport.unproject(px)
  const cx = center[0]
  const cy = center[1]
  const edge = viewport.unproject([px[0] + radiusPx, px[1]])
  const r = Math.abs(edge[0] - cx)
  const markers = getClusters([cx - r, cy - r, cx + r, cy + r], deckZoom)
  if (!markers.length) return null
  let best: ClusterMarker | null = null
  let bestD = Infinity
  for (const m of markers) {
    const dx = m.x - cx
    const dy = m.y - cy
    const d = dx * dx + dy * dy
    if (d < bestD) {
      bestD = d
      best = m
    }
  }
  return best
}
