// T-090.5.2/.5.2.1 — Road render layers (Map Engine v2 slot 6: `world-roads-casing` +
// `world-roads`). Two PathLayers for the whole terrain's road graph (766 Everon segments —
// plan rule: one Deck layer per class GROUP, never per chunk/class). Export polylines are
// road-surface quad soup, so parseRoadsPayload centerlines them (extractRoadCenterline) and
// measures the true per-segment width; strokes draw at that geometric width
// (`widthUnits:'meters'`, ≥1 px clamp) with class color/dash from the contract table and a
// near-black casing underneath (operator style pass, T-090.5.2.1 — width source deviates from
// the contract's px@z0 table; flagged for doc sync). Per-class min-zoom gates live in lodGates
// (single visibility authority — LOD5); this module only asks `classVisible`.
//
// Pure decision exports (ROAD_STYLES, extractRoadCenterline, visibleRoadClasses,
// parseRoadsPayload) are node-testable; the layer builders stay thin (spine testability rule).

import { COORDINATE_SYSTEM } from '@deck.gl/core'
import { PathLayer } from '@deck.gl/layers'
import { PathStyleExtension, type PathStyleExtensionProps } from '@deck.gl/extensions'
import { classVisible, type RoadClass } from './lodGates'

export type { RoadClass }

/** One road, centerlined (extractRoadCenterline) from `objects/roads.json.gz`. */
export interface RoadSegment {
  id: string
  roadClass: RoadClass
  /** Centerline vertices in world meters, y-up (CARTESIAN north-up). */
  points: [number, number][]
  /** Measured road width in meters (median cross-edge length from the export geometry). */
  widthM: number
}

export interface RoadStyle {
  color: [number, number, number]
  /** Contract-table width in meters — FALLBACK only; measured segment width wins (.5.2.1). */
  widthM: number
  dashed: boolean
}

/** Contract road table (color / width px @ z0 / dash). Min-zooms are lodGates', not ours. */
export const ROAD_STYLES: Record<RoadClass, RoadStyle> = {
  highway_paved: { color: [0xc8, 0xc8, 0xc8], widthM: 4, dashed: false },
  road_paved: { color: [0xa0, 0xa0, 0xa0], widthM: 2.5, dashed: false },
  road_dirt: { color: [0x8b, 0x69, 0x14], widthM: 2, dashed: true },
  track: { color: [0x6b, 0x50, 0x10], widthM: 1.5, dashed: true },
  path: { color: [0x5a, 0x4a, 0x3a], widthM: 1, dashed: true },
  runway: { color: [0xff, 0xff, 0xff], widthM: 6, dashed: false },
}

export const ROAD_CLASSES = Object.keys(ROAD_STYLES) as RoadClass[]

/** Dash pattern [dash, gap] in meters (PathStyleExtension uses getWidth units); [0,0] = solid. */
export function dashArrayFor(cls: RoadClass): [number, number] {
  return ROAD_STYLES[cls].dashed ? [8, 6] : [0, 0]
}

/** Road classes drawn at this deckZoom (stable order — join(',') makes a memo key). */
export function visibleRoadClasses(deckZoom: number): RoadClass[] {
  return ROAD_CLASSES.filter((cls) => classVisible(cls, deckZoom))
}

const isPoint = (p: unknown): p is [number, number] =>
  Array.isArray(p) && p.length >= 2 && Number.isFinite(p[0]) && Number.isFinite(p[1])

/** Consecutive centerline vertices closer than this are the collapsed duplicate cross-edges. */
const CENTERLINE_DEDUPE_M = 0.05

/** The export's road polylines are NOT centerlines — they are road-surface quad soup:
 *  alternating cross-edge point PAIRS (edge length = true road width; runway 20 m, paved 4 m,
 *  dirt 1.75 m on Everon), with every second cross-edge duplicated. Drawing them raw produces
 *  perpendicular "centipede" ticks (T-090.5.2.1 diagnosis: 41,758 of 169,346 steps are dups).
 *  Recover the drawable geometry: midpoint of each pair = centerline vertex, median pair
 *  length = measured width. Returns null when fewer than 2 distinct midpoints survive. */
export function extractRoadCenterline(
  points: [number, number][],
): { path: [number, number][]; widthM: number } | null {
  const path: [number, number][] = []
  const widths: number[] = []
  const pairCount = Math.floor(points.length / 2) // odd trailing point is dropped
  for (let k = 0; k < pairCount; k++) {
    const a = points[2 * k]
    const b = points[2 * k + 1]
    const mx = (a[0] + b[0]) / 2
    const my = (a[1] + b[1]) / 2
    const prev = path[path.length - 1]
    if (prev && Math.hypot(mx - prev[0], my - prev[1]) < CENTERLINE_DEDUPE_M) continue
    path.push([mx, my])
    widths.push(Math.hypot(b[0] - a[0], b[1] - a[1]))
  }
  if (path.length < 2) return null
  const sorted = [...widths].sort((x, y) => x - y)
  return { path, widthM: sorted[Math.floor(sorted.length / 2)] }
}

/** Narrow the fetched roads payload and centerline every segment; malformed/unknown-class/
 *  degenerate segments are dropped (closed enum per map-object-roads schema). */
export function parseRoadsPayload(raw: unknown): RoadSegment[] {
  const segments = (raw as { roadSegments?: unknown })?.roadSegments
  if (!Array.isArray(segments)) return []
  const out: RoadSegment[] = []
  for (const s of segments) {
    const seg = s as { id?: unknown; roadClass?: unknown; points?: unknown }
    if (
      typeof seg.id === 'string' &&
      typeof seg.roadClass === 'string' &&
      seg.roadClass in ROAD_STYLES &&
      Array.isArray(seg.points) &&
      seg.points.length >= 2 &&
      seg.points.every(isPoint)
    ) {
      const center = extractRoadCenterline(seg.points as [number, number][])
      if (!center) continue
      out.push({
        id: seg.id,
        roadClass: seg.roadClass as RoadClass,
        points: center.path,
        // Measured width, sanity-clamped; style-table width is the fallback for nonsense.
        widthM:
          center.widthM > 0.3 && center.widthM < 40
            ? center.widthM
            : ROAD_STYLES[seg.roadClass as RoadClass].widthM,
      })
    }
  }
  return out
}

/** Near-black casing under every road (classic cartographic pop — operator pick 2026-07-05). */
const CASING_COLOR: [number, number, number] = [30, 30, 34]
const CASING_WIDTH_FACTOR = 1.4

/** Build the road layer pair for the classes visible at the current band: `world-roads-casing`
 *  (darker, +40% width) under `world-roads` (measured true width, class color/dash). Mass
 *  layers — never pickable (pick ships worker-side in T-090.9). */
export function buildRoadLayers(opts: {
  segments: RoadSegment[]
  visibleClasses: RoadClass[]
}): PathLayer<RoadSegment, PathStyleExtensionProps<RoadSegment>>[] {
  const visible = new Set(opts.visibleClasses)
  const data = opts.segments.filter((s) => visible.has(s.roadClass))
  if (data.length === 0) return []
  const common = {
    data,
    coordinateSystem: COORDINATE_SYSTEM.CARTESIAN,
    getPath: (d: RoadSegment) => d.points.flat(),
    positionFormat: 'XY' as const,
    widthUnits: 'meters' as const,
    capRounded: true,
    jointRounded: true,
    pickable: false,
  }
  return [
    new PathLayer<RoadSegment, PathStyleExtensionProps<RoadSegment>>({
      ...common,
      id: 'world-roads-casing',
      getColor: CASING_COLOR,
      getWidth: (d) => d.widthM * CASING_WIDTH_FACTOR,
      widthMinPixels: 2,
    }),
    new PathLayer<RoadSegment, PathStyleExtensionProps<RoadSegment>>({
      ...common,
      id: 'world-roads',
      getColor: (d) => ROAD_STYLES[d.roadClass].color,
      getWidth: (d) => d.widthM,
      widthMinPixels: 1,
      getDashArray: (d: RoadSegment) => dashArrayFor(d.roadClass),
      extensions: [new PathStyleExtension({ dash: true })],
    }),
  ]
}
