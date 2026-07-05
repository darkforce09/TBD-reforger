// T-090.5.5 — Tree/prop glyph gates. Pure helpers (rotation handedness R8/GL-G5, heightM size
// cap, hex tint), the two IconLayer builders (binary plumbing + null degrade + pickable), and
// the treeStore streaming brain: LOD3 (@ −2 trees hidden, no worker call), the glyph band opening
// at TREE_GLYPH_MIN_ZOOM, class partition (buildings dropped, rocks ride props), dedupe, and
// stale-reply supersede. Node env — Deck layers construct without a GL context (props only).
import { describe, it, expect } from 'vitest'
import {
  DEFAULT_GLYPH_RGBA,
  EMPTY_TREE_GLYPHS,
  GLYPH_SIZE_MIN_PX,
  buildPropGlyphLayer,
  buildTreeGlyphLayer,
  deckAngleForRotationDeg,
  glyphSizeMeters,
  hexToRgba,
  treeSizeMultiplier,
  type TreeGlyphComposite,
} from './treePropLayer'
import { createTreeStore, type TreeStreamClient } from './treeStore'
import { REF_ZOOM, classVisible } from './lodGates'
import { RENDER_CLASS_CODES, type VisibleSet, type WorldManifestLite } from '../workers/worldObjectsCore'
import { TERRAINS } from '../coords/terrains'
import type { WorldGlyphAtlas } from '../layers/worldGlyphAtlas'

// --- pure helpers ----------------------------------------------------------------------

describe('deckAngleForRotationDeg (R8 / GL-G5 — export yaw → Deck getAngle)', () => {
  it('flips handedness (clockwise export → CCW Deck) and keeps 0 vs 90 distinct', () => {
    expect(deckAngleForRotationDeg(0)).toBe(0)
    expect(deckAngleForRotationDeg(90)).toBe(-90)
    expect(deckAngleForRotationDeg(180)).toBe(-180)
    expect(deckAngleForRotationDeg(0)).not.toBe(deckAngleForRotationDeg(90)) // R8 distinctness
    expect(Object.is(deckAngleForRotationDeg(0), -0)).toBe(false) // never -0
    expect(deckAngleForRotationDeg(Number.NaN)).toBe(0)
  })
})

describe('treeSizeMultiplier (glyphs-spec 1.5× cap)', () => {
  it('clamps to [1.0, 1.5]; short/undefined/invalid → 1.0', () => {
    expect(treeSizeMultiplier(undefined)).toBe(1)
    expect(treeSizeMultiplier(10)).toBe(1) // reference height
    expect(treeSizeMultiplier(5)).toBe(1) // shorter clamps up to 1
    expect(treeSizeMultiplier(12.5)).toBe(1.25)
    expect(treeSizeMultiplier(20)).toBe(1.5) // 2× clamps to cap
    expect(treeSizeMultiplier(100)).toBe(1.5)
    expect(treeSizeMultiplier(0)).toBe(1)
    expect(treeSizeMultiplier(Number.NaN)).toBe(1)
  })
})

describe('glyphSizeMeters (sizeUnits:meters trick — displayPx = base·2^(zoom−REF))', () => {
  it('is baseSizePx·mult / 2^REF_ZOOM', () => {
    expect(glyphSizeMeters(24, 10)).toBeCloseTo(24 / 2 ** REF_ZOOM, 6) // mult 1
    expect(glyphSizeMeters(24, 20)).toBeCloseTo((24 * 1.5) / 2 ** REF_ZOOM, 6) // mult 1.5
    expect(glyphSizeMeters(16, undefined)).toBeCloseTo(16 / 2 ** REF_ZOOM, 6)
  })
})

describe('hexToRgba', () => {
  it('parses #rrggbb / #rgb / bare; invalid → DEFAULT_GLYPH_RGBA', () => {
    expect(hexToRgba('#2d5a27')).toEqual([45, 90, 39, 255])
    expect(hexToRgba('4a7a32')).toEqual([74, 122, 50, 255])
    expect(hexToRgba('#abc')).toEqual([170, 187, 204, 255])
    expect(hexToRgba(undefined)).toEqual(DEFAULT_GLYPH_RGBA)
    expect(hexToRgba('nothex')).toEqual(DEFAULT_GLYPH_RGBA)
    expect(hexToRgba('#12')).toEqual(DEFAULT_GLYPH_RGBA)
  })
})

// --- layer builders --------------------------------------------------------------------

const ATLAS: WorldGlyphAtlas = {
  atlasUrl: '/map-assets/glyphs/atlas/world-glyphs.webp',
  iconMapping: {
    'tree-conifer': { x: 0, y: 0, width: 64, height: 64, anchorX: 32, anchorY: 64, mask: true },
    'prop-fence': { x: 64, y: 0, width: 64, height: 64, anchorX: 32, anchorY: 64, mask: false },
  },
}

const SAMPLE: TreeGlyphComposite = {
  count: 2,
  positions: Float32Array.from([10, 20, 30, 40]),
  anglesDeg: Float32Array.from([0, -90]),
  sizes: Float32Array.from([4.5, 3]),
  colors: Uint8Array.from([45, 90, 39, 255, 74, 122, 50, 255]),
  iconKeys: ['tree-conifer', 'prop-fence'],
}

describe('buildTreeGlyphLayer / buildPropGlyphLayer', () => {
  it('returns null with no atlas or an empty composite (R5 degrade)', () => {
    expect(buildTreeGlyphLayer({ composite: SAMPLE, atlas: null, visible: true })).toBeNull()
    expect(buildTreeGlyphLayer({ composite: EMPTY_TREE_GLYPHS, atlas: ATLAS, visible: true })).toBeNull()
    expect(buildPropGlyphLayer({ composite: EMPTY_TREE_GLYPHS, atlas: ATLAS, visible: true })).toBeNull()
  })

  it('builds world-trees / world-props IconLayers over binary attributes (never pickable)', () => {
    const trees = buildTreeGlyphLayer({ composite: SAMPLE, atlas: ATLAS, visible: true })
    const props = buildPropGlyphLayer({ composite: SAMPLE, atlas: ATLAS, visible: false })
    expect(trees?.id).toBe('world-trees')
    expect(props?.id).toBe('world-props')
    const p = trees?.props as Record<string, unknown>
    expect(p.pickable).toBe(false)
    expect(p.visible).toBe(true)
    expect(props?.props.visible).toBe(false)
    expect(p.sizeUnits).toBe('meters')
    expect(p.sizeMinPixels).toBe(GLYPH_SIZE_MIN_PX)
    // iconAtlas/iconMapping are async Deck props (resolve to null/{} without a GL context) —
    // the getIcon accessor test below covers the atlas mapping wiring instead.
    const data = p.data as { length: number; attributes: Record<string, { value: unknown; size: number }> }
    expect(data.length).toBe(2)
    expect(data.attributes.getPosition.value).toBe(SAMPLE.positions)
    expect(data.attributes.getPosition.size).toBe(2)
    expect(data.attributes.getAngle.value).toBe(SAMPLE.anglesDeg)
    expect(data.attributes.getSize.value).toBe(SAMPLE.sizes)
    expect(data.attributes.getColor.value).toBe(SAMPLE.colors)
    expect(data.attributes.getColor.size).toBe(4)
  })

  it('getIcon resolves the atlas key by row index (binary data → no per-row object)', () => {
    const trees = buildTreeGlyphLayer({ composite: SAMPLE, atlas: ATLAS, visible: true })
    const getIcon = (trees?.props as unknown as { getIcon: (o: unknown, i: { index: number }) => string }).getIcon
    expect(getIcon(undefined, { index: 0 })).toBe('tree-conifer')
    expect(getIcon(undefined, { index: 1 })).toBe('prop-fence')
  })
})

// --- treeStore -------------------------------------------------------------------------

const EVERON = TERRAINS.everon
const BUILDING = RENDER_CLASS_CODES.indexOf('building')
const TREE = RENDER_CLASS_CODES.indexOf('tree')
const PROP = RENDER_CLASS_CODES.indexOf('prop')
const ROCK = RENDER_CLASS_CODES.indexOf('rockLarge')

/** Prefab rows with render blocks (the shape narrowPrefabRows now ships). */
const PREFABS: WorldManifestLite['prefabRows'] = [
  { prefabId: 100, kind: 'tree', class: 'conifer', spatial: { heightM: 20 },
    render: { iconKey: 'tree-conifer', baseSizePx: 24, defaultColor: '#2d5a27' } },
  { prefabId: 101, kind: 'tree', class: 'deciduous', spatial: { heightM: 10 },
    render: { iconKey: 'tree-deciduous', baseSizePx: 24, defaultColor: '#4a7a32' } },
  { prefabId: 102, kind: 'building', class: 'residential',
    render: { iconKey: 'building-residential', baseSizePx: 12 } }, // dropped (buildings ≠ glyph layer)
  { prefabId: 103, kind: 'prop', class: 'fence',
    render: { iconKey: 'prop-fence', baseSizePx: 12 } }, // no defaultColor → DEFAULT tint
  { prefabId: 104, kind: 'rock', class: 'boulder',
    render: { iconKey: 'rock-boulder', baseSizePx: 14 } }, // rockLarge → prop group
]

const MANIFEST: WorldManifestLite = {
  terrainId: 'everon',
  chunkSizeM: 512,
  cells: null,
  prefabRows: PREFABS,
  roadsPath: null,
  densityPath: null,
  instanceCount: null,
  hasOversized: false,
}

/** All fixture instances, tagged with their render-class code. */
const ROWS: { pid: number; x: number; y: number; rot: number; code: number }[] = [
  { pid: 100, x: 10, y: 20, rot: 0, code: TREE },
  { pid: 101, x: 30, y: 40, rot: 90, code: TREE },
  { pid: 102, x: 50, y: 60, rot: 0, code: BUILDING },
  { pid: 103, x: 70, y: 80, rot: 45, code: PROP },
  { pid: 104, x: 90, y: 99, rot: 0, code: ROCK },
]

/** Simulate the worker: only classes whose gate is open at this zoom are returned (matches the
 *  real visibleInstances gate — the store then partitions + drops buildings). */
function makeSet(deckZoom: number): VisibleSet {
  const rows = ROWS.filter((r) => classVisible(RENDER_CLASS_CODES[r.code], deckZoom))
  return {
    count: rows.length,
    positions: Float32Array.from(rows.flatMap((r) => [r.x, r.y])),
    prefabIdx: Uint16Array.from(rows.map((r) => r.pid)),
    rotations: Float32Array.from(rows.map((r) => r.rot)),
    classes: Uint8Array.from(rows.map((r) => r.code)),
  }
}

const tick = () => new Promise<void>((r) => setTimeout(r, 0))

interface Harness {
  store: ReturnType<typeof createTreeStore>
  calls: { manifest: number; visible: number; lastZoom: number | null }
}

function makeHarness(manifest: WorldManifestLite | null = MANIFEST): Harness {
  const calls = { manifest: 0, visible: 0, lastZoom: null as number | null }
  const client: TreeStreamClient = {
    loadManifest: async () => {
      calls.manifest++
      return manifest
    },
    visibleInstances: async (_bbox, z) => {
      calls.visible++
      calls.lastZoom = z
      return makeSet(z)
    },
  }
  return { store: createTreeStore({ client }), calls }
}

/** Kick the stream + settle the manifest promise so setTreeViewport runs synchronously after. */
async function ready(h: Harness): Promise<void> {
  h.store.ensureTreeStream(EVERON)
  await tick()
}

const BBOX: [number, number, number, number] = [1024, 512, 1536, 1024]
const ALL = { trees: true, props: true }

describe('treeStore — LOD3 gate (contract LOD3: trees hidden below their band)', () => {
  it('@ −2: no worker call, composites empty', async () => {
    const h = makeHarness()
    await ready(h)
    h.store.setTreeViewport(BBOX, -2, ALL)
    await tick()
    expect(h.calls.visible).toBe(0) // gated before the worker (skip-when-invisible)
    expect(h.store.getTreeGlyphs().count).toBe(0)
    expect(h.store.getPropGlyphs().count).toBe(0)
  })

  it('both toggles off @ 0: no worker call, empty', async () => {
    const h = makeHarness()
    await ready(h)
    h.store.setTreeViewport(BBOX, 0, { trees: false, props: false })
    await tick()
    expect(h.calls.visible).toBe(0)
    expect(h.store.getTreeGlyphs().count).toBe(0)
  })
})

describe('treeStore — glyph band + partition', () => {
  it('@ 0: trees visible, buildings dropped, props still gated out', async () => {
    const h = makeHarness()
    await ready(h)
    h.store.setTreeViewport(BBOX, 0, ALL)
    await tick()
    expect(h.calls.visible).toBe(1)
    const trees = h.store.getTreeGlyphs()
    expect(trees.count).toBe(2) // conifer + deciduous; building (code 0) dropped
    expect(trees.iconKeys).toEqual(['tree-conifer', 'tree-deciduous'])
    // conifer heightM 20 → 1.5×; deciduous heightM 10 → 1.0×
    expect(trees.sizes[0]).toBeCloseTo((24 * 1.5) / 2 ** REF_ZOOM, 6)
    expect(trees.sizes[1]).toBeCloseTo(24 / 2 ** REF_ZOOM, 6)
    // rot 0 → 0, rot 90 → −90 (handedness)
    expect([...trees.anglesDeg]).toEqual([0, -90])
    expect([...trees.colors.slice(0, 4)]).toEqual([45, 90, 39, 255])
    expect(h.store.getPropGlyphs().count).toBe(0) // prop gate closed at 0
  })

  it('@ +3: props/rocks ride the prop group; fence has the DEFAULT tint', async () => {
    const h = makeHarness()
    await ready(h)
    h.store.setTreeViewport(BBOX, 3, { trees: false, props: true })
    await tick()
    const props = h.store.getPropGlyphs()
    expect(props.count).toBe(2) // fence (prop) + boulder (rockLarge)
    expect(props.iconKeys).toEqual(['prop-fence', 'rock-boulder'])
    expect(props.sizes[0]).toBeCloseTo(12 / 2 ** REF_ZOOM, 6) // no heightM → mult 1
    expect([...props.colors.slice(0, 4)]).toEqual(DEFAULT_GLYPH_RGBA) // fence has no defaultColor
  })
})

describe('treeStore — dedupe + empty-terrain', () => {
  it('repeat viewport in the same chunk set + band does not refetch (T-057)', async () => {
    const h = makeHarness()
    await ready(h)
    h.store.setTreeViewport(BBOX, 0, ALL)
    await tick()
    h.store.setTreeViewport(BBOX, 0, ALL)
    await tick()
    expect(h.calls.visible).toBe(1)
    // A band change refetches.
    h.store.setTreeViewport(BBOX, 3, ALL)
    await tick()
    expect(h.calls.visible).toBe(2)
  })

  it('terrain without an export → glyphs cleanly absent (plan R11)', async () => {
    const h = makeHarness(null)
    await ready(h)
    h.store.setTreeViewport(BBOX, 0, ALL)
    await tick()
    expect(h.calls.visible).toBe(0)
    expect(h.store.getTreeGlyphs().count).toBe(0)
  })
})

describe('treeStore — stale reply supersede (replace-not-accumulate race)', () => {
  it('a superseded in-flight reply is discarded (only the newest commits)', async () => {
    const resolvers: (() => void)[] = []
    const calls = { visible: 0 }
    const client: TreeStreamClient = {
      loadManifest: async () => MANIFEST,
      visibleInstances: (_bbox, z) =>
        new Promise<VisibleSet>((res) => {
          calls.visible++
          resolvers.push(() => res(makeSet(z)))
        }),
    }
    const store = createTreeStore({ client })
    store.ensureTreeStream(EVERON)
    await tick()
    // Two different chunk sets at the same band → two in-flight calls.
    store.setTreeViewport([0, 0, 512, 512], 0, ALL)
    store.setTreeViewport([2048, 2048, 2560, 2560], 0, ALL)
    expect(calls.visible).toBe(2)
    const rev0 = store.getTreeRevision()
    resolvers[1]() // newest resolves first
    await tick()
    resolvers[0]() // stale resolves second — must be discarded
    await tick()
    expect(store.getTreeRevision()).toBe(rev0 + 1) // exactly one commit, not two
  })
})
