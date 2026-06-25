// Cluster / LOD index for the zoomed-out map (T-065) — a supercluster KD-forest that runs
// parallel to the dense `slotIconCache` and the picking `slotSpatialIndex`. When the camera is
// far enough out that individual icons are invisible dots (zoom < ZOOM_DETAIL_MIN) and the
// mission is large (slotCount > CLUSTER_SLOT_THRESHOLD), the map draws cluster bubbles + a count
// from this index instead of all ~367k IconLayer markers — Eden's "group icon stacked when
// zoomed out". Zoom back in and the detail IconLayer takes over (T-061/T-063 unchanged).
//
// Kept in sync on the SAME mutations as `slotIconCache` (called from inside its mutators), so
// it mirrors the icon cache through every path — single add, bulk paste, snapshot load, delete
// cascade, drag release. Two supercluster realities shape this module:
//
//   1. supercluster projects lng/lat through spherical mercator, which NaNs/wraps for raw Arma
//      meters (0–12800). So world x/y are linearly normalized into a safe lng/lat window using
//      the active terrain bounds before load()/query, and centroids are de-normalized back.
//   2. supercluster has NO incremental insert/remove — load() rebuilds the whole forest. To keep
//      the mutators O(k) we maintain an internal points array incrementally and only flip a
//      `dirty` flag; getClusters() lazily rebuilds the forest ONCE when dirty (never per pan
//      frame — pan/zoom only query, and edits at min-zoom are rare).
//
// Module-level singleton: safe under the single-mounted-doc invariant (same as slotIconCache /
// slotSpatialIndex / the LOCAL_ORIGIN singletons elsewhere in the engine).

import Supercluster from 'supercluster'
import type { TerrainDef } from '../coords/terrains'
import type { ID } from './schema'
import type { SlotIcon } from './selectors'

// Raw world meters (mx/my) ride along in properties so a terrain switch can re-project the whole
// set without a fresh snapshot; supercluster itself only reads geometry.coordinates.
type PointProps = { id: ID; mx: number; my: number }
type PointFeature = Supercluster.PointFeature<PointProps>

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

// Safe mercator window: lat is clamped well inside the ±85.05° mercator limit so the projection
// stays finite and monotonic across the whole terrain.
const LAT_SPAN = 170 // [-85, 85]
const LNG_SPAN = 360 // [-180, 180]

let terrainW = 12800
let terrainH = 12800

const points: PointFeature[] = []
const index = new Map<ID, number>() // id -> position in `points`
let sc: Supercluster<PointProps> | null = null
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

function makePoint(id: ID, x: number, y: number): PointFeature {
  return {
    type: 'Feature',
    properties: { id, mx: x, my: y },
    geometry: { type: 'Point', coordinates: [normLng(x), normLat(y)] },
  }
}

function normLng(x: number): number {
  return (x / terrainW) * LNG_SPAN - 180
}
function normLat(y: number): number {
  return (y / terrainH) * LAT_SPAN - 85
}
function worldX(lng: number): number {
  return ((lng + 180) / LNG_SPAN) * terrainW
}
function worldY(lat: number): number {
  return ((lat + 85) / LAT_SPAN) * terrainH
}

/** Map the continuous Deck zoom (band [-6, 6]) to an integer supercluster zoom level. Cluster mode
 *  is only active at/below ZOOM_CLUSTER_MAX (-4), so the live range is [-6, -4] → scz [2, 4]. */
export function deckZoomToSuperZoom(deckZoom: number): number {
  const z = Math.round(deckZoom + 8)
  return z < 0 ? 0 : z > 16 ? 16 : z
}

/** Capture the active terrain so normalization matches it (called on terrain change). Re-projects
 *  every cached point from its raw meters against the new bounds — cheap, only on a terrain switch. */
export function setTerrain(terrain: TerrainDef): void {
  if (terrain.width === terrainW && terrain.height === terrainH) return
  terrainW = terrain.width
  terrainH = terrain.height
  for (const p of points) {
    p.geometry.coordinates = [normLng(p.properties.mx), normLat(p.properties.my)]
  }
  dirty = true
  markersDirty = true
}

/** O(n) full rebuild — on a full snapshot replace. Mirrors slotSpatialIndex.rebuild. */
export function rebuild(icons: SlotIcon[]): void {
  points.length = 0
  index.clear()
  for (let i = 0; i < icons.length; i++) {
    index.set(icons[i].id, i)
    points.push(makePoint(icons[i].id, icons[i].x, icons[i].y))
  }
  dirty = true
  markersDirty = true
}

/** O(k) insert newly-placed icons (asset drop / paste). Ids already present are skipped. */
export function insert(icons: { id: ID; x: number; y: number }[]): void {
  for (const s of icons) {
    if (index.has(s.id)) continue
    index.set(s.id, points.length)
    points.push(makePoint(s.id, s.x, s.y))
  }
  dirty = true
}

/** O(k) remove ids via swap-and-pop (mirror of slotIconCache.remove). Ids not present skipped. */
export function remove(ids: ID[]): void {
  for (const id of ids) {
    const i = index.get(id)
    if (i === undefined) continue
    const last = points.length - 1
    if (i !== last) {
      const moved = points[last]
      points[i] = moved
      index.set(moved.properties.id, i)
    }
    points.pop()
    index.delete(id)
  }
  dirty = true
  markersDirty = true
}

/** O(k) reposition: rewrite the normalized coords in place. */
export function updatePositions(patches: Record<ID, { x: number; y: number }>): void {
  for (const id in patches) {
    const i = index.get(id)
    if (i === undefined) continue
    const p = points[i]
    p.properties.mx = patches[id].x
    p.properties.my = patches[id].y
    p.geometry.coordinates = [normLng(patches[id].x), normLat(patches[id].y)]
  }
  dirty = true
  markersDirty = true
}

/** Drop everything (store reset / doc unmount). */
export function clear(): void {
  points.length = 0
  index.clear()
  sc = null
  dirty = true
  cachedMarkers = []
  cachedSuperZoom = null
  markersDirty = true
}

function ensureForest(): Supercluster<PointProps> {
  if (dirty || !sc) {
    const next = new Supercluster<PointProps>({ radius: 60, maxZoom: 16 })
    next.load(points)
    sc = next
    dirty = false
  }
  return sc
}

/** Cluster markers in WORLD meters for a viewport bbox (world meters) at a Deck zoom. Lazily
 *  rebuilds the forest only when dirty — a pan/zoom in cluster mode only ever queries. */
export function getClusters(
  bbox: [number, number, number, number],
  deckZoom: number,
): ClusterMarker[] {
  if (!points.length) return []
  const forest = ensureForest()
  const [minX, minY, maxX, maxY] = bbox
  const features = forest.getClusters(
    [normLng(minX), normLat(minY), normLng(maxX), normLat(maxY)],
    deckZoomToSuperZoom(deckZoom),
  )
  const out: ClusterMarker[] = new Array(features.length)
  for (let i = 0; i < features.length; i++) {
    const f = features[i]
    const [lng, lat] = f.geometry.coordinates
    const isCluster = (f.properties as { cluster?: boolean }).cluster === true
    out[i] = {
      x: worldX(lng),
      y: worldY(lat),
      count: isCluster ? (f.properties as { point_count: number }).point_count : 1,
      id: isCluster ? null : (f.properties as { id: ID }).id,
    }
  }
  return out
}

/** Pan-stable cluster markers for the render layer (T-065.2). Queries the FULL terrain (not the
 *  viewport), so panning never changes the result — recomputes only when an edit dirtied the set or
 *  the supercluster zoom bucket changed, bumping `markersVersion`. Lazy: `ensureForest()` (the
 *  supercluster.load) runs on the first cluster-band read, never during detail-mode editing. */
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
  if (!points.length) return null
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
