// T-090.5.2 — Building render layers (Map Engine v2 slot 7, ids `world-buildings` +
// `world-building-badges`). Normative shipped geometry (contract N6): oriented bounding
// rectangle from prefab `spatial.halfExtentsM` + instance `rotationDeg` — real footprint rings
// only when a future export proves them. Export axes: halfExtents z is VERTICAL
// (heightM = 2·z, footprintM2 = 2x·2y), so the map-plane rect is ±x/±y. Rotation handedness
// (glyphs spec L2): 0° = map north (+y), clockwise-positive.
//
// Badges: military/tower/bunker get a center glyph at deckZoom ≥ BUILDING_BADGE_MIN_ZOOM.
// Derived from the prefab CLASS, not prefab render.iconKey — everon bunker prefabs ship
// without an iconKey, and the badge contract is class-based anyway.
//
// Pure decision exports (obbCorners, badgeIconKey, buildingPrefabLookup,
// buildingsFromChunkInstances) are node-testable; the two builders stay thin (spine rule).
// Visibility booleans come from lodGates.classVisible — sole authority (LOD5).

import { COORDINATE_SYSTEM } from '@deck.gl/core'
import { IconLayer, PolygonLayer } from '@deck.gl/layers'
import { REF_ZOOM } from './lodGates'
import type { WorldGlyphAtlas } from '../layers/worldGlyphAtlas'

/** One placed building, shaped for the Polygon/Icon layers (built once at load). */
export interface BuildingInstance {
  position: [number, number]
  /** OBB footprint ring (4 corners, unclosed — PolygonLayer closes it). */
  polygon: [number, number][]
  buildingClass: string
  /** `building-badge-{class}` for military/tower/bunker, else null. */
  badgeIconKey: string | null
}

/** The slice of a building prefab the render layer needs (from prefabs.json.gz). */
export interface BuildingPrefabInfo {
  buildingClass: string
  halfX: number
  halfY: number
}

const BADGE_CLASSES = new Set(['military', 'tower', 'bunker'])

/** Center badge glyph key for a building class (military/tower/bunker only). */
export function badgeIconKey(buildingClass: string): string | null {
  return BADGE_CLASSES.has(buildingClass) ? `building-badge-${buildingClass}` : null
}

/** OBB footprint corners around (x, y): half extents ±halfX/±halfY rotated by `rotationDeg`
 *  clockwise from north (L2). Returns the 4-corner ring in world meters. */
export function obbCorners(
  x: number,
  y: number,
  halfX: number,
  halfY: number,
  rotationDeg: number,
): [number, number][] {
  const rad = (rotationDeg * Math.PI) / 180
  const cos = Math.cos(rad)
  const sin = Math.sin(rad)
  const rot = (dx: number, dy: number): [number, number] => [
    x + dx * cos + dy * sin,
    y - dx * sin + dy * cos,
  ]
  return [rot(-halfX, -halfY), rot(halfX, -halfY), rot(halfX, halfY), rot(-halfX, halfY)]
}

/** prefabId → footprint info for the chunk filter. Included kinds: `building` plus `water`
 *  pier/dock (T-090.5.2.2 — walkable hard structures that must read on the map; their
 *  buildingClass carries the water class for styling). Everything else (trees/props in mixed
 *  P2 chunks) resolves to undefined and is discarded. */
// eslint-disable-next-line complexity -- defensive narrowing of untyped export rows: every branch is a field guard on one row shape; splitting them hides the wire contract (pre-existing at T-090.5.2.2, annotated in T-090.5.3 when lint surfaced it)
export function buildingPrefabLookup(raw: unknown): Map<number, BuildingPrefabInfo> {
  const rows = (raw as { prefabs?: unknown })?.prefabs
  const lookup = new Map<number, BuildingPrefabInfo>()
  if (!Array.isArray(rows)) return lookup
  for (const row of rows) {
    const p = row as {
      prefabId?: unknown
      kind?: unknown
      class?: unknown
      spatial?: { halfExtentsM?: { x?: unknown; y?: unknown } }
    }
    if (typeof p.prefabId !== 'number') continue
    const cls = typeof p.class === 'string' ? p.class : 'unknown'
    const included =
      p.kind === 'building' || (p.kind === 'water' && (cls === 'pier' || cls === 'dock'))
    if (!included) continue
    const hx = p.spatial?.halfExtentsM?.x
    const hy = p.spatial?.halfExtentsM?.y
    lookup.set(p.prefabId, {
      buildingClass: cls,
      halfX: typeof hx === 'number' && hx > 0 ? hx : 2,
      halfY: typeof hy === 'number' && hy > 0 ? hy : 2,
    })
  }
  return lookup
}

/** Chunk `instances` rows ([prefabId, x, y, z, rotationDeg]) → building instances with
 *  precomputed OBB rings; everything not in the building lookup is dropped unretained. */
export function buildingsFromChunkInstances(
  instances: unknown,
  lookup: Map<number, BuildingPrefabInfo>,
): BuildingInstance[] {
  if (!Array.isArray(instances)) return []
  const out: BuildingInstance[] = []
  for (const row of instances) {
    if (!Array.isArray(row) || row.length < 2) continue
    const [prefabId, x, y, , rotationDeg] = row as number[]
    const info = lookup.get(prefabId)
    if (!info || !Number.isFinite(x) || !Number.isFinite(y)) continue
    const rot = Number.isFinite(rotationDeg) ? rotationDeg : 0
    out.push({
      position: [x, y],
      polygon: obbCorners(x, y, info.halfX, info.halfY, rot),
      buildingClass: info.buildingClass,
      badgeIconKey: badgeIconKey(info.buildingClass),
    })
  }
  return out
}

// Solid-dark A3-style footprints (operator style pass T-090.5.2.1 — supersedes the t090_5
// ghost values rgba(120,120,130,0.35)/#888; flagged for doc sync). T-090.5.2.2 adds per-class
// tints over the dark default so the taxonomy reads on the map: military sand-brown, bridges
// grey-blue, piers timber, ruins faded, castle stone-brown, lighthouse white landmark,
// containers steel-blue, tents olive.
type Rgba = [number, number, number, number]
export const FILL_DEFAULT: Rgba = [38, 38, 44, 184]
export const FILL_BY_CLASS: Record<string, Rgba> = {
  military: [0x7a, 0x5c, 0x3d, 184],
  bridge: [90, 90, 100, 200],
  pier: [110, 95, 75, 190],
  dock: [110, 95, 75, 190],
  ruin: [58, 56, 60, 110],
  castle: [70, 58, 48, 190],
  lighthouse: [235, 235, 235, 220],
  container: [60, 70, 90, 184],
  tent: [92, 82, 50, 184],
  shed: [50, 50, 56, 184],
  garage: [50, 50, 56, 184],
}
const STROKE: Rgba = [150, 150, 158, 204]
const STROKE_LIGHTHOUSE: Rgba = [180, 60, 50, 230]

/** Build the `world-buildings` OBB PolygonLayer. `visible` gates via Deck (data stays on GPU
 *  across band crossings). Mass layer — never pickable (T-090.9 owns pick). */
export function buildBuildingLayer(opts: {
  buildings: BuildingInstance[]
  visible: boolean
}): PolygonLayer<BuildingInstance> | null {
  if (opts.buildings.length === 0) return null
  return new PolygonLayer<BuildingInstance>({
    id: 'world-buildings',
    data: opts.buildings,
    visible: opts.visible,
    coordinateSystem: COORDINATE_SYSTEM.CARTESIAN,
    getPolygon: (d) => d.polygon.flat(),
    positionFormat: 'XY',
    filled: true,
    stroked: true,
    getFillColor: (d) => FILL_BY_CLASS[d.buildingClass] ?? FILL_DEFAULT,
    getLineColor: (d) => (d.buildingClass === 'lighthouse' ? STROKE_LIGHTHOUSE : STROKE),
    getLineWidth: 1,
    lineWidthUnits: 'pixels',
    pickable: false,
  })
}

/** Glyph display: baseSizePx is defined at REF_ZOOM (displayPx = base·2^(zoom−REF_ZOOM)), which
 *  is exactly `sizeUnits:'meters'` with size = base/2^REF_ZOOM — Deck then scales with zoom for
 *  free (no per-frame updateTriggers). Readability floor per plan §4.4 min-px clamps. */
const BADGE_SIZE_MIN_PX = 8
/** building-badge-* baseSizePx (glyphs-spec N4 table; all three badge glyphs are 10). */
const BADGE_BASE_SIZE_PX = 10

/** Build the `world-building-badges` IconLayer (military/tower/bunker center glyphs). Returns
 *  null until the glyph atlas is loaded — per-layer degrade, never a crash (plan risk R5). */
export function buildBuildingBadgeLayer(opts: {
  buildings: BuildingInstance[]
  atlas: WorldGlyphAtlas | null
  visible: boolean
}): IconLayer<BuildingInstance> | null {
  if (!opts.atlas) return null
  const data = opts.buildings.filter((b) => b.badgeIconKey !== null)
  if (data.length === 0) return null
  const { atlasUrl, iconMapping } = opts.atlas
  return new IconLayer<BuildingInstance>({
    id: 'world-building-badges',
    data,
    visible: opts.visible,
    coordinateSystem: COORDINATE_SYSTEM.CARTESIAN,
    iconAtlas: atlasUrl,
    iconMapping,
    getIcon: (d) => d.badgeIconKey as string,
    getPosition: (d) => d.position,
    getSize: BADGE_BASE_SIZE_PX / 2 ** REF_ZOOM,
    sizeUnits: 'meters',
    sizeMinPixels: BADGE_SIZE_MIN_PX,
    pickable: false,
  })
}
