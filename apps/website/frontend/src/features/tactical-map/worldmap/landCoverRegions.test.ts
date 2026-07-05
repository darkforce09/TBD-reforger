// T-090.8.1 — Land-cover region gates: payload narrowing against the map-object-region
// golden shape (multi-ring hulls, kind enum), malformed-row rejection, and the
// `world-landcover` builder (per-kind tint, never pickable, null on empty).
import { describe, it, expect } from 'vitest'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import {
  LANDCOVER_FILL,
  buildLandCoverLayer,
  parseRegionsPayload,
  type LandCoverRegion,
} from './landCoverRegions'

const GOLDEN_DIR = resolve(process.cwd(), '../../../packages/tbd-schema/golden/map-objects')

const RING: [number, number][] = [
  [9408, 192],
  [9504, 192],
  [9504, 224],
  [9408, 224],
]
const HOLE: [number, number][] = [
  [9440, 200],
  [9472, 200],
  [9472, 216],
  [9440, 216],
]

function forestRow(id = 'forest-everon-001'): Record<string, unknown> {
  return {
    id,
    kind: 'forest',
    polygon: [RING, HOLE],
    treeCount: 479000,
    dominantSpeciesClass: 'mixed',
    densityPerHa: 103.2,
    areaHa: 4641.5,
    coverType: 'hard',
    source: 'derived-hull',
  }
}

describe('parseRegionsPayload', () => {
  it('parses multi-ring regions with their T-090.9 metadata', () => {
    const rows = parseRegionsPayload({
      regions: [forestRow(), { id: 'f1', kind: 'field', polygon: [RING], areaHa: 12 }],
    })
    expect(rows).toHaveLength(2)
    expect(rows[0].kind).toBe('forest')
    expect(rows[0].polygon).toHaveLength(2) // outer + hole preserved for Deck complex polygons
    expect(rows[0].treeCount).toBe(479000)
    expect(rows[0].dominantSpeciesClass).toBe('mixed')
    expect(rows[0].coverType).toBe('hard')
    expect(rows[1].kind).toBe('field')
    expect(rows[1].treeCount).toBeUndefined()
  })

  it('drops malformed rows: bad kind, missing/short/non-numeric rings, missing id', () => {
    const rows = parseRegionsPayload({
      regions: [
        forestRow('ok'),
        { id: 'bad-kind', kind: 'swamp', polygon: [RING] },
        { id: 'no-polygon', kind: 'forest' },
        { id: 'empty-polygon', kind: 'forest', polygon: [] },
        { id: 'short-ring', kind: 'forest', polygon: [[[0, 0], [1, 1]]] },
        { id: 'nan-point', kind: 'forest', polygon: [[[0, 0], [1, Number.NaN], [2, 2]]] },
        { kind: 'forest', polygon: [RING] },
        'not-an-object',
      ],
    })
    expect(rows.map((r) => r.id)).toEqual(['ok'])
  })

  it('returns [] on non-payload shapes', () => {
    expect(parseRegionsPayload(null)).toEqual([])
    expect(parseRegionsPayload({})).toEqual([])
    expect(parseRegionsPayload({ regions: 'nope' })).toEqual([])
  })

  it('accepts the committed map-object-regions golden (F1 shape parity)', () => {
    const golden = JSON.parse(
      readFileSync(`${GOLDEN_DIR}/map-object-regions-everon-sample.json`, 'utf8'),
    ) as unknown
    const rows = parseRegionsPayload(golden)
    expect(rows.length).toBeGreaterThan(0)
    for (const r of rows) {
      expect(['forest', 'field', 'waterBody']).toContain(r.kind)
      expect(r.polygon[0].length).toBeGreaterThanOrEqual(3)
    }
  })
})

describe('buildLandCoverLayer', () => {
  const regions = parseRegionsPayload({
    regions: [
      forestRow(),
      { id: 'f1', kind: 'field', polygon: [RING] },
      { id: 'w1', kind: 'waterBody', polygon: [RING] },
    ],
  })

  it('builds world-landcover with per-kind tints, never pickable', () => {
    const layer = buildLandCoverLayer({ regions, visible: true })
    expect(layer?.id).toBe('world-landcover')
    expect(layer?.props.pickable).toBe(false)
    expect(layer?.props.visible).toBe(true)
    const fill = layer?.props.getFillColor as unknown as (d: LandCoverRegion) => number[]
    expect(fill(regions[0])).toEqual(LANDCOVER_FILL.forest)
    expect(fill(regions[1])).toEqual(LANDCOVER_FILL.field)
    expect(fill(regions[2])).toEqual(LANDCOVER_FILL.waterBody)
    const poly = layer?.props.getPolygon as (d: LandCoverRegion) => unknown
    expect(poly(regions[0])).toBe(regions[0].polygon)
  })

  it('forest tint stays a light underlay (never darker than the mass fill α 0.35)', () => {
    expect(LANDCOVER_FILL.forest[3]).toBeLessThan(0.2 * 255)
  })

  it('returns null on empty data; visible:false passes through', () => {
    expect(buildLandCoverLayer({ regions: [], visible: true })).toBeNull()
    expect(buildLandCoverLayer({ regions, visible: false })?.props.visible).toBe(false)
  })
})
