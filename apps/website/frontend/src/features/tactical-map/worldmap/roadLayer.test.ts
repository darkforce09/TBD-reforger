// T-090.5.2 — Road styling + band visibility must mirror the canonical road class table in
// t090_render_lod_contract.md (colors/widths/dash verbatim; min-zooms delegated to lodGates).
// Contract N3 band rows for roads: −6…−4 highway/paved/runway · dirt+track from −2 · path from +4.
import { describe, it, expect } from 'vitest'
import {
  ROAD_STYLES,
  buildRoadLayer,
  dashArrayFor,
  parseRoadsPayload,
  visibleRoadClasses,
} from './roadLayer'

describe('ROAD_STYLES (contract road class table, verbatim)', () => {
  it('colors, widths (px @ z0 = meters) and dash flags match the contract', () => {
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

describe('parseRoadsPayload (roads.json.gz shape)', () => {
  const good = {
    schemaVersion: '1.0.0',
    terrainId: 'everon',
    roadSegments: [
      {
        id: 'road-everon-0000',
        roadClass: 'runway',
        points: [
          [4751.58, 12057.49],
          [4797.59, 12057.54],
        ],
      },
      {
        id: 'road-everon-0001',
        roadClass: 'road_dirt',
        points: [
          [0, 0],
          [10, 10],
          [20, 15],
        ],
      },
    ],
  }

  it('narrows well-formed segments', () => {
    const segs = parseRoadsPayload(good)
    expect(segs).toHaveLength(2)
    expect(segs[0].roadClass).toBe('runway')
    expect(segs[1].points).toHaveLength(3)
  })

  it('buildRoadLayer construction smoke: world-roads id, class-filtered data, never pickable', () => {
    const segs = parseRoadsPayload(good)
    const layer = buildRoadLayer({ segments: segs, visibleClasses: ['runway'] })
    expect(layer?.id).toBe('world-roads')
    expect(layer?.props.data).toHaveLength(1)
    expect(layer?.props.pickable).toBe(false)
    expect(layer?.props.widthMinPixels).toBe(1)
    // No visible class with segments → no layer at all.
    expect(buildRoadLayer({ segments: segs, visibleClasses: [] })).toBeNull()
  })

  it('drops malformed / unknown-class / degenerate segments and non-payloads', () => {
    const segs = parseRoadsPayload({
      roadSegments: [
        {
          id: 'x',
          roadClass: 'hyperloop',
          points: [
            [0, 0],
            [1, 1],
          ],
        }, // unknown class
        { id: 'y', roadClass: 'track', points: [[0, 0]] }, // < 2 points
        {
          id: 'z',
          roadClass: 'track',
          points: [
            [0, 0],
            [Number.NaN, 1],
          ],
        }, // non-finite
        {
          roadClass: 'track',
          points: [
            [0, 0],
            [1, 1],
          ],
        }, // no id
      ],
    })
    expect(segs).toEqual([])
    expect(parseRoadsPayload(null)).toEqual([])
    expect(parseRoadsPayload('<html>')).toEqual([])
  })
})
