// T-090.5.2 — Road render layer (Map Engine v2 slot 6, layer id `world-roads`). One PathLayer
// for the whole terrain's road graph (766 Everon segments — plan rule: one Deck layer per class
// GROUP, never per chunk/class). Class styling follows the road table in
// t090_render_lod_contract.md verbatim: widths are px @ deckZoom 0, where 1 px = 1 m (mpp = 1),
// so `widthUnits: 'meters'` reproduces the table exactly and scales geometrically at every other
// zoom; `widthMinPixels: 1` is the contract's ≥1 px clamp. Per-class min-zoom gates live in
// lodGates (single visibility authority — LOD5); this module only asks `classVisible`.
//
// Pure decision exports (ROAD_STYLES, visibleRoadClasses, parseRoadsPayload) are node-testable;
// buildRoadLayer stays a thin Deck builder (spine testability rule).

import { COORDINATE_SYSTEM } from '@deck.gl/core'
import { PathLayer } from '@deck.gl/layers'
import { PathStyleExtension, type PathStyleExtensionProps } from '@deck.gl/extensions'
import { classVisible, type RoadClass } from './lodGates'

export type { RoadClass }

/** One road polyline from `objects/roads.json.gz` (map-object-roads schema). */
export interface RoadSegment {
  id: string
  roadClass: RoadClass
  /** World-meter vertices, y-up (CARTESIAN north-up — direct PathLayer path). */
  points: [number, number][]
}

export interface RoadStyle {
  color: [number, number, number]
  /** Stroke width in meters (= px @ deckZoom 0 per the contract table). */
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

/** Narrow the fetched roads payload; malformed/unknown-class segments are dropped (closed enum
 *  per map-object-roads schema — an unknown class means a schema bump we don't render yet). */
export function parseRoadsPayload(raw: unknown): RoadSegment[] {
  const segments = (raw as { roadSegments?: unknown })?.roadSegments
  if (!Array.isArray(segments)) return []
  const out: RoadSegment[] = []
  for (const s of segments) {
    const seg = s as Partial<RoadSegment>
    if (
      typeof seg.id === 'string' &&
      typeof seg.roadClass === 'string' &&
      seg.roadClass in ROAD_STYLES &&
      Array.isArray(seg.points) &&
      seg.points.length >= 2 &&
      seg.points.every(isPoint)
    ) {
      out.push({ id: seg.id, roadClass: seg.roadClass, points: seg.points })
    }
  }
  return out
}

/** Build the `world-roads` PathLayer: segments filtered to the classes visible at the current
 *  band. Mass layer — never pickable (pick ships worker-side in T-090.9). */
export function buildRoadLayer(opts: {
  segments: RoadSegment[]
  visibleClasses: RoadClass[]
}): PathLayer<RoadSegment, PathStyleExtensionProps<RoadSegment>> | null {
  const visible = new Set(opts.visibleClasses)
  const data = opts.segments.filter((s) => visible.has(s.roadClass))
  if (data.length === 0) return null
  return new PathLayer<RoadSegment, PathStyleExtensionProps<RoadSegment>>({
    id: 'world-roads',
    data,
    coordinateSystem: COORDINATE_SYSTEM.CARTESIAN,
    getPath: (d) => d.points.flat(),
    positionFormat: 'XY',
    getColor: (d) => ROAD_STYLES[d.roadClass].color,
    getWidth: (d) => ROAD_STYLES[d.roadClass].widthM,
    widthUnits: 'meters',
    widthMinPixels: 1,
    capRounded: true,
    jointRounded: true,
    getDashArray: (d: RoadSegment) => dashArrayFor(d.roadClass),
    extensions: [new PathStyleExtension({ dash: true })],
    pickable: false,
  })
}
