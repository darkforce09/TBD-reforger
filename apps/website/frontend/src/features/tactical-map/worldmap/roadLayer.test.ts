// T-090.5.2/.5.2.1 — Road styling + band visibility must mirror the canonical road class table
// in t090_render_lod_contract.md (colors/dash verbatim; min-zooms delegated to lodGates), and
// centerline extraction must recover drawable geometry from the export's quad-soup polylines
// (alternating cross-edge pairs; every second cross-edge duplicated).
import { describe, it, expect } from 'vitest'
import {
  ROAD_STYLES,
  buildRoadLayers,
  dashArrayFor,
  extractRoadCenterline,
  parseRoadsPayload,
  visibleRoadClasses,
} from './roadLayer'

// Road along +y at x=0, true width 4 m, in export quad-soup form:
// cross pair → mid (0,0) · reversed pair @ y=10 → mid (0,10) · duplicated pair (the export's
// every-second-edge dup) → same mid, deduped · pair @ y=20 → mid (0,20).
const QUAD_SOUP: [number, number][] = [
  [-2, 0],
  [2, 0],
  [2, 10],
  [-2, 10],
  [-2, 10],
  [2, 10],
  [2, 20],
  [-2, 20],
]

describe('ROAD_STYLES (contract road class table)', () => {
  it('colors, fallback widths and dash flags match the contract', () => {
    expect(ROAD_STYLES.highway_paved).toEqual({ color: [200, 200, 200], widthM: 4, dashed: false })
    expect(ROAD_STYLES.road_paved).toEqual({ color: [160, 160, 160], widthM: 2.5, dashed: false })
    expect(ROAD_STYLES.road_dirt).toEqual({ color: [139, 105, 20], widthM: 2, dashed: true })
    expect(ROAD_STYLES.track).toEqual({ color: [107, 80, 16], widthM: 1.5, dashed: true })
    expect(ROAD_STYLES.path).toEqual({ color: [90, 74, 58], widthM: 1, dashed: true })
    expect(ROAD_STYLES.runway).toEqual({ color: [255, 255, 255], widthM: 6, dashed: false })
  })

  it('solid classes get [0,0] dash arrays; dashed classes a real pattern', () => {
    expect(dashArrayFor('highway_paved')).toEqual([0, 0])
    expect(dashArrayFor('road_paved')).toEqual([0, 0])
    expect(dashArrayFor('runway')).toEqual([0, 0])
    for (const cls of ['road_dirt', 'track', 'path'] as const) {
      const [dash, gap] = dashArrayFor(cls)
      expect(dash).toBeGreaterThan(0)
      expect(gap).toBeGreaterThan(0)
    }
  })
})

describe('extractRoadCenterline (quad soup → centerline + measured width)', () => {
  it('midpoints each cross pair, dedupes the duplicated edges, measures the width', () => {
    const center = extractRoadCenterline(QUAD_SOUP)
    expect(center).not.toBeNull()
    expect(center?.path).toEqual([
      [0, 0],
      [0, 10],
      [0, 20],
    ])
    expect(center?.widthM).toBe(4)
  })

  it('drops an odd trailing point', () => {
    const center = extractRoadCenterline([...QUAD_SOUP, [999, 999]])
    expect(center?.path).toEqual([
      [0, 0],
      [0, 10],
      [0, 20],
    ])
  })

  it('null when fewer than 2 distinct midpoints survive', () => {
    expect(
      extractRoadCenterline([
        [-2, 0],
        [2, 0],
      ]),
    ).toBeNull()
    expect(
      extractRoadCenterline([
        [-2, 0],
        [2, 0],
        [2, 0],
        [-2, 0],
      ]),
    ).toBeNull()
    expect(extractRoadCenterline([])).toBeNull()
  })

  it('width is the median across cross-edges (robust to junction flares)', () => {
    const center = extractRoadCenterline([
      [-2, 0],
      [2, 0],
      [2, 10],
      [-2, 10],
      [-6, 20],
      [6, 20], // 12 m flare at a junction
    ])
    expect(center?.widthM).toBe(4)
  })
})

describe('visibleRoadClasses per N3 band', () => {
  it('whole-island −6…−4: only highway, paved, runway', () => {
    for (const zoom of [-6, -5, -4.5]) {
      expect(visibleRoadClasses(zoom).sort()).toEqual(['highway_paved', 'road_paved', 'runway'])
    }
  })

  it('dirt + track join at −2 (default zoom shows all but path)', () => {
    expect(visibleRoadClasses(-2.5).sort()).toEqual(['highway_paved', 'road_paved', 'runway'])
    expect(visibleRoadClasses(-2).sort()).toEqual([
      'highway_paved',
      'road_dirt',
      'road_paved',
      'runway',
      'track',
    ])
    expect(visibleRoadClasses(0)).not.toContain('path')
  })

  it('path joins at +4 only', () => {
    expect(visibleRoadClasses(3.9)).not.toContain('path')
    expect(visibleRoadClasses(4)).toContain('path')
    expect(visibleRoadClasses(6).sort()).toEqual(
      ['highway_paved', 'road_dirt', 'road_paved', 'runway', 'track', 'path'].sort(),
    )
  })
})

describe('parseRoadsPayload (roads.json.gz shape → centerlined segments)', () => {
  const good = {
    schemaVersion: '1.0.0',
    terrainId: 'everon',
    roadSegments: [
      { id: 'road-everon-0000', roadClass: 'runway', points: QUAD_SOUP },
      {
        id: 'road-everon-0001',
        roadClass: 'road_dirt',
        points: [
          [0, -1],
          [0, 1],
          [10, 1],
          [10, -1],
        ],
      },
    ],
  }

  it('narrows well-formed segments to centerlines with measured widths', () => {
    const segs = parseRoadsPayload(good)
    expect(segs).toHaveLength(2)
    expect(segs[0].points).toEqual([
      [0, 0],
      [0, 10],
      [0, 20],
    ])
    expect(segs[0].widthM).toBe(4)
    expect(segs[1].points).toEqual([
      [0, 0],
      [10, 0],
    ])
    expect(segs[1].widthM).toBe(2)
  })

  it('buildRoadLayers construction smoke: casing under road, class-filtered, never pickable', () => {
    const segs = parseRoadsPayload(good)
    const layers = buildRoadLayers({ segments: segs, visibleClasses: ['runway'] })
    expect(layers.map((l) => l.id)).toEqual(['world-roads-casing', 'world-roads'])
    for (const l of layers) {
      expect(l.props.data).toHaveLength(1)
      expect(l.props.pickable).toBe(false)
    }
    // No visible class with segments → no layers at all.
    expect(buildRoadLayers({ segments: segs, visibleClasses: [] })).toEqual([])
  })

  it('drops malformed / unknown-class / degenerate segments and non-payloads', () => {
    const segs = parseRoadsPayload({
      roadSegments: [
        { id: 'x', roadClass: 'hyperloop', points: QUAD_SOUP }, // unknown class
        { id: 'y', roadClass: 'track', points: [[0, 0]] }, // < 2 points
        {
          id: 'z',
          roadClass: 'track',
          points: [
            [0, 0],
            [Number.NaN, 1],
          ],
        }, // non-finite
        { roadClass: 'track', points: QUAD_SOUP }, // no id
        {
          id: 'w',
          roadClass: 'track',
          points: [
            [-2, 0],
            [2, 0],
          ],
        }, // single cross-edge → no centerline
      ],
    })
    expect(segs).toEqual([])
    expect(parseRoadsPayload(null)).toEqual([])
    expect(parseRoadsPayload('<html>')).toEqual([])
  })
})
