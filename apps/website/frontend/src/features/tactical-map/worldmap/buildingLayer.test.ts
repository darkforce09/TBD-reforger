// T-090.5.2 — Building OBB geometry + badge derivation + LOD gate spot-checks. Covers the
// contract LOD3 building half ("at −2: buildings = OBB rects") and the R8 rotation-distinct
// requirement (90° ≠ 0° footprint). Rotation handedness is L2: 0° = north (+y), clockwise.
import { describe, it, expect } from 'vitest'
import { classVisible } from './lodGates'
import {
  FILL_BY_CLASS,
  FILL_DEFAULT,
  badgeIconKey,
  buildBuildingBadgeLayer,
  buildBuildingLayer,
  buildingPrefabLookup,
  buildingsFromChunkInstances,
  obbCorners,
} from './buildingLayer'

const closeTo = (ring: [number, number][], expected: [number, number][]) => {
  expect(ring).toHaveLength(expected.length)
  ring.forEach(([x, y], i) => {
    expect(x).toBeCloseTo(expected[i][0], 6)
    expect(y).toBeCloseTo(expected[i][1], 6)
  })
}

describe('obbCorners (N6 OBB geometry, L2 handedness)', () => {
  it('0°: axis-aligned rect at ±halfX/±halfY around the pivot', () => {
    closeTo(obbCorners(100, 200, 5, 3, 0), [
      [95, 197],
      [105, 197],
      [105, 203],
      [95, 203],
    ])
  })

  it('90° clockwise: extents swap — footprint is visually distinct from 0° (R8)', () => {
    // At 90° cw the local +y (north) edge points at world +x: halfY now spans x, halfX spans y.
    closeTo(obbCorners(100, 200, 5, 3, 90), [
      [97, 205],
      [97, 195],
      [103, 195],
      [103, 205],
    ])
  })

  it('360° = 0°; rectangle area is rotation-invariant', () => {
    closeTo(obbCorners(0, 0, 4, 2, 360), obbCorners(0, 0, 4, 2, 0))
    const ring = obbCorners(0, 0, 4, 2, 37)
    // Shoelace |area| = (2·4)·(2·2) for any rotation.
    let area = 0
    for (let i = 0; i < ring.length; i++) {
      const [x1, y1] = ring[i]
      const [x2, y2] = ring[(i + 1) % ring.length]
      area += x1 * y2 - x2 * y1
    }
    expect(Math.abs(area) / 2).toBeCloseTo(32, 6)
  })
})

describe('building LOD gates (contract LOD3 spot-checks)', () => {
  it('OBB rects from −2.5 — visible at default zoom −2, hidden at −3', () => {
    expect(classVisible('building', -3)).toBe(false)
    expect(classVisible('building', -2.5)).toBe(true)
    expect(classVisible('building', -2)).toBe(true)
  })

  it('badges from +1 only', () => {
    expect(classVisible('buildingBadge', 0)).toBe(false)
    expect(classVisible('buildingBadge', 1)).toBe(true)
  })
})

describe('badgeIconKey (class-derived, not prefab iconKey)', () => {
  it('military/tower/bunker get building-badge-*; the rest none', () => {
    expect(badgeIconKey('military')).toBe('building-badge-military')
    expect(badgeIconKey('tower')).toBe('building-badge-tower')
    expect(badgeIconKey('bunker')).toBe('building-badge-bunker')
    expect(badgeIconKey('residential')).toBeNull()
    expect(badgeIconKey('civic')).toBeNull()
    expect(badgeIconKey('unknown')).toBeNull()
  })
})

describe('prefab lookup + chunk filter (buildings + piers out of mixed P2 chunks)', () => {
  const prefabsPayload = {
    prefabs: [
      {
        prefabId: 0,
        kind: 'building',
        class: 'residential',
        spatial: { halfExtentsM: { x: 5, y: 5, z: 4 } },
      },
      {
        prefabId: 84,
        kind: 'building',
        class: 'bunker',
        spatial: { halfExtentsM: { x: 3, y: 2, z: 1 } },
      },
      {
        prefabId: 331,
        kind: 'tree',
        class: 'conifer',
        spatial: { halfExtentsM: { x: 2, y: 2, z: 8 } },
      },
      {
        prefabId: 400,
        kind: 'water',
        class: 'pier',
        spatial: { halfExtentsM: { x: 10, y: 1.5, z: 1 } },
      },
      {
        prefabId: 401,
        kind: 'water',
        class: 'buoy',
        spatial: { halfExtentsM: { x: 0.5, y: 0.5, z: 0.5 } },
      },
    ],
  }

  it('lookup keeps buildings + water piers/docks (trees + buoys are filtered out)', () => {
    const lookup = buildingPrefabLookup(prefabsPayload)
    expect(lookup.size).toBe(3)
    expect(lookup.get(0)).toEqual({ buildingClass: 'residential', halfX: 5, halfY: 5 })
    expect(lookup.get(400)).toEqual({ buildingClass: 'pier', halfX: 10, halfY: 1.5 })
    expect(lookup.get(331)).toBeUndefined() // tree
    expect(lookup.get(401)).toBeUndefined() // buoy — not a walkable structure
    expect(buildingPrefabLookup(null).size).toBe(0)
  })

  it('per-class fills: taxonomy classes styled, unknown classes fall to the dark default', () => {
    expect(FILL_BY_CLASS.military).toEqual([0x7a, 0x5c, 0x3d, 184])
    for (const cls of ['bridge', 'pier', 'ruin', 'castle', 'lighthouse', 'container', 'tent']) {
      expect(FILL_BY_CLASS[cls], cls).toBeDefined()
      expect(FILL_BY_CLASS[cls]).not.toEqual(FILL_DEFAULT)
    }
    expect(FILL_BY_CLASS.residential).toBeUndefined() // default fill
  })

  it('construction smoke: layer ids, visibility pass-through, badge layer needs the atlas', () => {
    const lookup = buildingPrefabLookup(prefabsPayload)
    const buildings = buildingsFromChunkInstances(
      [
        [0, 100, 100, 0, 0],
        [84, 200, 200, 0, 45],
      ],
      lookup,
    )
    const poly = buildBuildingLayer({ buildings, visible: true })
    expect(poly?.id).toBe('world-buildings')
    expect(poly?.props.pickable).toBe(false)
    expect(poly?.props.visible).toBe(true)
    expect(buildBuildingLayer({ buildings: [], visible: true })).toBeNull()
    // Badge layer degrades to null without the atlas (risk R5), mounts with it.
    expect(buildBuildingBadgeLayer({ buildings, atlas: null, visible: true })).toBeNull()
    const badge = buildBuildingBadgeLayer({
      buildings,
      atlas: {
        atlasUrl: '/map-assets/glyphs/atlas/world-glyphs.webp',
        iconMapping: {
          'building-badge-bunker': {
            x: 0,
            y: 0,
            width: 128,
            height: 128,
            anchorX: 64,
            anchorY: 64,
            mask: false,
          },
        },
      },
      visible: false,
    })
    expect(badge?.id).toBe('world-building-badges')
    expect(badge?.props.visible).toBe(false)
    expect(badge?.props.data).toHaveLength(1) // bunker only — residential has no badge
    expect(badge?.props.pickable).toBe(false)
  })

  it('chunk instances resolve through the lookup; trees + malformed rows drop', () => {
    const lookup = buildingPrefabLookup(prefabsPayload)
    const out = buildingsFromChunkInstances(
      [
        [0, 5120.04, 5518.09, 51.85, 94.55], // residential
        [84, 100, 200, 0, 0], // bunker → badge
        [331, 50, 60, 12, 0], // tree → dropped
        [999, 1, 2, 0, 0], // unknown prefab → dropped
        [0, Number.NaN, 2, 0, 0], // non-finite → dropped
        'garbage',
      ],
      lookup,
    )
    expect(out).toHaveLength(2)
    expect(out[0].position).toEqual([5120.04, 5518.09])
    expect(out[0].buildingClass).toBe('residential')
    expect(out[0].badgeIconKey).toBeNull()
    expect(out[0].polygon).toHaveLength(4)
    expect(out[1].badgeIconKey).toBe('building-badge-bunker')
    expect(buildingsFromChunkInstances(null, lookup)).toEqual([])
  })
})
