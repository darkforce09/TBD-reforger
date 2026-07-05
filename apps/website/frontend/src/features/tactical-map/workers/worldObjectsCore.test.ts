// T-090.5.3 — Worker core gates: W1 (golden chunk parses off the main thread — the core IS
// the worker body, driven here with an fs/fixture fetch), W2 (pickNearest === brute force),
// W4 (visibleInstances respects lodGates — trees hidden below their band, no cluster path),
// and the INSTANCE_BUDGET cap, census-driven from the committed Everon type inventory.
import { describe, it, expect } from 'vitest'
import { gzipSync } from 'node:zlib'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import {
  OVERSIZED_HALF_EXTENT_M,
  RENDER_CLASS_CODES,
  createWorldObjectsCore,
  renderClassForPrefab,
  type WorldObjectsCoreApi,
  type WorldPrefabRow,
} from './worldObjectsCore'
import {
  BUILDING_BADGE_MIN_ZOOM,
  BUILDING_FOOTPRINT_MIN_ZOOM,
  INSTANCE_BUDGET,
  PROP_MIN_ZOOM,
  TREE_GLYPH_MIN_ZOOM,
  VEGETATION_MIN_ZOOM,
  classVisible,
  type WorldRenderClass,
} from '../worldmap/lodGates'
import { HYDRATE_RENDER_CLASSES } from '../worldmap/chunkStore'

// ---------------------------------------------------------------------------------------
// Fixtures: the committed T-090.2 goldens (packages/tbd-schema) + the real Everon census
// (packages/map-assets). Paths mirror sampleElevation.test.ts (cwd = apps/website/frontend).
// ---------------------------------------------------------------------------------------

const GOLDEN_DIR = resolve(process.cwd(), '../../../packages/tbd-schema/golden/map-objects')
const MAP_ASSETS = resolve(process.cwd(), '../../../packages/map-assets')

interface GoldenChunk {
  cx: number
  cy: number
  chunkSizeM: number
  chunk: { instances: [number, number, number, number, number][] }
}

const goldenChunk = JSON.parse(
  readFileSync(`${GOLDEN_DIR}/map-object-chunk-sample.json`, 'utf8'),
) as GoldenChunk
const goldenPrefabs = JSON.parse(
  readFileSync(`${GOLDEN_DIR}/map-object-prefabs-sample.json`, 'utf8'),
) as WorldPrefabRow[]

/** Synthetic second chunk (cx=2, cy=1): one of every kind so the gate/pick tests can probe
 *  class behavior. Coordinates are f32-exact (quarters) so parity asserts stay exact. */
const MIXED_ROWS: [number, number, number, number, number][] = [
  [9, 1200.25, 650.5, 30, 45], // building residential
  [7, 1250.5, 620.25, 0.5, 90], // water pier → 'building' render class
  [0, 1300.75, 700.5, 55, 10], // tree conifer
  [1, 1310.25, 710.75, 54, 20], // tree deciduous
  [3, 1320.5, 720.25, 52, 0], // vegetation bush
  [4, 1330.75, 730.5, 51, 0], // rock boulder → rockLarge
  [5, 1340.25, 740.75, 50, 0], // prop fence
  [8, 1350.5, 750.25, 49, 0], // road kind → unclassified (never drawn/picked)
]
/** Tree-only chunk (cx=3, cy=1) — exercises "hydrated, nothing drawable" delivery. */
const TREE_ROWS: [number, number, number, number, number][] = [[2, 1800.25, 600.5, 40, 0]]

const gz = (obj: unknown) => new Uint8Array(gzipSync(Buffer.from(JSON.stringify(obj))))
const plain = (obj: unknown) => new Uint8Array(Buffer.from(JSON.stringify(obj)))

/** Minimal TBDD encoder (T-090.3.2 wire layout) for density fixtures — 16 B header + LE
 *  u16 tree/rock channels (mirrors scripts/map-assets/lib/density-grid.mjs). */
function tbdd(tree: Uint16Array, cols = 17, rows = 17): Uint8Array {
  const bytes = new Uint8Array(16 + 2 * cols * rows * 2)
  bytes.set([0x54, 0x42, 0x44, 0x44])
  const dv = new DataView(bytes.buffer)
  dv.setUint16(4, 1, true)
  dv.setUint16(6, 32, true)
  dv.setUint16(8, cols, true)
  dv.setUint16(10, rows, true)
  bytes[12] = 2
  for (let k = 0; k < tree.length; k++) dv.setUint16(16 + 2 * k, tree[k], true)
  return bytes // rock channel stays zero
}

interface FixtureOpts {
  withOversizedPrefab?: boolean
  /** Omit `objects.densityPath` from the manifest (pre-T-090.3.2 export shape). */
  withoutDensity?: boolean
}

function makeCore(opts: FixtureOpts = {}, instanceBudget?: number): WorldObjectsCoreApi {
  const prefabs: WorldPrefabRow[] = [...goldenPrefabs]
  if (opts.withOversizedPrefab) {
    prefabs.push({
      prefabId: 99,
      kind: 'building',
      class: 'bridge',
      label: 'Long bridge',
      spatial: { halfExtentsM: { x: OVERSIZED_HALF_EXTENT_M + 16, y: 6, z: 5 } },
    })
  }
  const files = new Map<string, Uint8Array>([
    [
      '/map-assets/everon/manifest.json',
      plain({
        objects: {
          prefabsPath: 'objects/prefabs.json.gz',
          chunksPath: 'objects/chunks',
          chunkSizeM: 512,
          roadsPath: 'objects/roads.json.gz',
          ...(opts.withoutDensity ? {} : { densityPath: 'objects/density' }),
          instanceCount:
            goldenChunk.chunk.instances.length + MIXED_ROWS.length + TREE_ROWS.length,
        },
      }),
    ],
    // Density fixtures (T-090.8.1): 1_1 fully dense, 2_1 all-zero, 3_1 missing (no file).
    ['/map-assets/everon/objects/density/1_1.bin', tbdd(Uint16Array.from({ length: 17 * 17 }, () => 4))],
    ['/map-assets/everon/objects/density/2_1.bin', tbdd(new Uint16Array(17 * 17))],
    ['/map-assets/everon/objects/prefabs.json.gz', gz({ prefabs })],
    [
      '/map-assets/everon/objects/chunks/manifest.json',
      plain({
        chunkSizeM: 512,
        cells: [
          { cx: 1, cy: 1, path: 'objects/chunks/1_1.json.gz', instanceCount: 3 },
          { cx: 2, cy: 1, path: 'objects/chunks/2_1.json.gz', instanceCount: MIXED_ROWS.length },
          { cx: 3, cy: 1, path: 'objects/chunks/3_1.json.gz', instanceCount: TREE_ROWS.length },
        ],
      }),
    ],
    // W1: the golden chunk payload, gzipped exactly like the production export files.
    ['/map-assets/everon/objects/chunks/1_1.json.gz', gz(goldenChunk.chunk)],
    ['/map-assets/everon/objects/chunks/2_1.json.gz', gz({ instances: MIXED_ROWS })],
    ['/map-assets/everon/objects/chunks/3_1.json.gz', gz({ instances: TREE_ROWS })],
  ])
  return createWorldObjectsCore({
    fetchBytes: async (url) => files.get(url) ?? null,
    instanceBudget,
  })
}

/** Bbox covering all three fixture chunks (cx 1–3, cy 1). */
const ALL_BBOX: [number, number, number, number] = [512, 512, 2048, 1024]

async function loadAll(core: WorldObjectsCoreApi) {
  const manifest = await core.loadManifest('everon')
  expect(manifest).not.toBeNull()
  const result = await core.loadChunksInBbox(ALL_BBOX, 0, {
    deckZoom: -2,
    classes: [...HYDRATE_RENDER_CLASSES],
  })
  return { manifest: manifest as NonNullable<typeof manifest>, result }
}

// ---------------------------------------------------------------------------------------

describe('renderClassForPrefab (taxonomy → render class)', () => {
  it('maps kinds per the T-090.5.2.2 taxonomy', () => {
    expect(renderClassForPrefab('building', 'residential')).toBe('building')
    expect(renderClassForPrefab('water', 'pier')).toBe('building')
    expect(renderClassForPrefab('water', 'dock')).toBe('building')
    expect(renderClassForPrefab('water', 'buoy')).toBeNull()
    expect(renderClassForPrefab('tree', 'conifer')).toBe('tree')
    expect(renderClassForPrefab('vegetation', 'bush')).toBe('vegetation')
    expect(renderClassForPrefab('rock', 'boulder')).toBe('rockLarge')
    expect(renderClassForPrefab('prop', 'fence')).toBe('prop')
    expect(renderClassForPrefab('utility', 'powerline')).toBe('prop')
    expect(renderClassForPrefab('road', 'road_paved')).toBeNull()
  })
})

describe('loadManifest', () => {
  it('returns null for terrains without an export (empty state, plan R11)', async () => {
    const core = createWorldObjectsCore({ fetchBytes: async () => null })
    expect(await core.loadManifest('arland')).toBeNull()
    expect(await core.loadManifest('custom')).toBeNull()
    expect(core.getStatus().ready).toBe(false)
  })

  it('flips getStatus().ready and reports the chunk grid', async () => {
    const core = makeCore()
    const manifest = await core.loadManifest('everon')
    expect(manifest?.chunkSizeM).toBe(512)
    expect(manifest?.cells?.map((c) => c.id)).toEqual(['1_1', '2_1', '3_1'])
    expect(manifest?.prefabRows.length).toBe(goldenPrefabs.length)
    expect(core.getStatus().ready).toBe(true)
  })

  it('hasOversized is data-driven from prefab half extents (plan §6 oversized ring)', async () => {
    expect((await makeCore().loadManifest('everon'))?.hasOversized).toBe(false)
    expect(
      (await makeCore({ withOversizedPrefab: true }).loadManifest('everon'))?.hasOversized,
    ).toBe(true)
  })
})

describe('W1 — golden chunk parse (worker core, gunzip + typed arrays)', () => {
  it('parses the T-090.2 golden chunk byte-shape into a building group', async () => {
    const core = makeCore()
    const { result } = await loadAll(core)
    const chunk = result.chunks.find((c) => c.id === '1_1')
    expect(chunk).toBeDefined()
    expect(chunk?.cx).toBe(goldenChunk.cx)
    expect(chunk?.cy).toBe(goldenChunk.cy)
    expect(chunk?.totalInstances).toBe(goldenChunk.chunk.instances.length)

    // The golden's three rows are all building-kind prefabs (9/14/18) → one 'building'
    // group, nothing else (requested classes = HYDRATE set, gate open at −2).
    expect(Object.keys(chunk?.groups ?? {})).toEqual(['building'])
    const group = chunk?.groups.building
    expect(group?.count).toBe(3)
    goldenChunk.chunk.instances.forEach(([pid, x, y, z, rot], i) => {
      expect(group?.prefabIdx[i]).toBe(pid)
      expect(group?.positions[2 * i]).toBeCloseTo(x, 3)
      expect(group?.positions[2 * i + 1]).toBeCloseTo(y, 3)
      expect(group?.z[i]).toBeCloseTo(z, 3)
      expect(group?.rotations[i]).toBeCloseTo(rot, 3)
    })
  })

  it('delivers non-drawable chunks with empty groups (hydrated-empty caching)', async () => {
    const core = makeCore()
    const { result } = await loadAll(core)
    const treeOnly = result.chunks.find((c) => c.id === '3_1')
    expect(treeOnly).toBeDefined()
    expect(treeOnly?.totalInstances).toBe(TREE_ROWS.length)
    expect(Object.keys(treeOnly?.groups ?? {})).toEqual([])
  })

  it('mixed chunks deliver ONLY building/pier rows this slice (trees never cross)', async () => {
    const core = makeCore()
    const { result } = await loadAll(core)
    const mixed = result.chunks.find((c) => c.id === '2_1')
    const group = mixed?.groups.building
    expect(group?.count).toBe(2) // prefab 9 (building) + 7 (pier)
    expect([...(group?.prefabIdx ?? [])]).toEqual([9, 7])
  })

  it('excludeIds suppresses re-delivery of chunks the main thread already holds', async () => {
    const core = makeCore()
    await loadAll(core)
    const result = await core.loadChunksInBbox(ALL_BBOX, 0, {
      deckZoom: -2,
      classes: ['building'],
      excludeIds: ['1_1', '3_1'],
    })
    expect(result.chunks.map((c) => c.id)).toEqual(['2_1'])
  })
})

describe('W2 — pickNearest parity with brute force', () => {
  /** Every classified fixture instance (unclassified kinds are not indexed → not pickable). */
  function classifiedInstances(): { id: string; x: number; y: number; cls: string }[] {
    const byPrefab = new Map(goldenPrefabs.map((p) => [p.prefabId, p]))
    const rows: { id: string; x: number; y: number; cls: string }[] = []
    const add = (chunkId: string, list: [number, number, number, number, number][]) => {
      list.forEach(([pid, x, y], i) => {
        const p = byPrefab.get(pid)
        const cls = p ? renderClassForPrefab(p.kind, p.class) : null
        if (cls) rows.push({ id: `${chunkId}:${i}`, x: Math.fround(x), y: Math.fround(y), cls })
      })
    }
    add('1_1', goldenChunk.chunk.instances)
    add('2_1', MIXED_ROWS)
    add('3_1', TREE_ROWS)
    return rows
  }

  function bruteNearest(
    probe: [number, number],
    radiusM: number,
    filter?: (cls: string) => boolean,
  ): string | null {
    let best: string | null = null
    let bestD = radiusM * radiusM
    for (const r of classifiedInstances()) {
      if (filter && !filter(r.cls)) continue
      const d = (r.x - probe[0]) ** 2 + (r.y - probe[1]) ** 2
      if (best === null ? d <= bestD : d < bestD) {
        bestD = d
        best = r.id
      }
    }
    return best
  }

  it('matches a brute-force scan across probes, radii and zoom filters', async () => {
    const core = makeCore()
    await loadAll(core)
    const probes: [number, number][] = [
      [512, 700], // on the golden boundary row
      [801, 900],
      [1240, 640], // between building and pier
      [1305, 705], // in the tree cluster
      [1351, 750],
      [0, 0],
      [2000, 2000],
    ]
    for (const probe of probes) {
      for (const radius of [5, 60, 5000]) {
        expect(await core.pickNearest(probe, radius)).toBe(bruteNearest(probe, radius))
      }
      // N4: with a deckZoom, only classes visible at that zoom are pickable.
      const gate = (cls: string) => classVisible(cls as WorldRenderClass, -2)
      expect(await core.pickNearest(probe, 5000, -2)).toBe(bruteNearest(probe, 5000, gate))
    }
  })

  it('radius miss returns null; unclassified rows are never picked', async () => {
    const core = makeCore()
    await loadAll(core)
    // Probe directly on the road-kind row (prefab 8 @ 1350.5, 750.25): its only neighbor
    // within 2 m is itself — but it was never indexed, so the pick misses entirely.
    expect(await core.pickNearest([1350.5, 750.25], 2)).toBeNull()
    expect(await core.pickNearest([50, 50], 10)).toBeNull()
  })

  it('pickRect mirrors the index content (read-only marquee)', async () => {
    const core = makeCore()
    await loadAll(core)
    const all = await core.pickRect([0, 0, 12800, 12800])
    expect(all.sort()).toEqual(classifiedInstances().map((r) => r.id).sort())
    // Gate applied: at −2 only building-class instances are pickable.
    const atDefault = await core.pickRect([0, 0, 12800, 12800], -2)
    expect(atDefault.sort()).toEqual(
      classifiedInstances()
        .filter((r) => r.cls === 'building')
        .map((r) => r.id)
        .sort(),
    )
  })
})

describe('W4 — visibleInstances respects lodGates (LOD5: no cluster path)', () => {
  it('at default −2: buildings visible, trees hidden', async () => {
    const core = makeCore()
    await loadAll(core)
    const set = await core.visibleInstances([0, 0, 12800, 12800], -2)
    // 3 golden buildings + synthetic building + pier = 5; zero of the 3 trees.
    expect(set.count).toBe(5)
    const buildingCode = RENDER_CLASS_CODES.indexOf('building')
    expect([...set.classes]).toEqual(Array.from({ length: 5 }, () => buildingCode))
  })

  it('below the building band (−3): nothing draws', async () => {
    const core = makeCore()
    await loadAll(core)
    expect((await core.visibleInstances([0, 0, 12800, 12800], -3)).count).toBe(0)
  })

  it('tree band opens at TREE_GLYPH_MIN_ZOOM; props/vegetation/rocks at their gates', async () => {
    const core = makeCore()
    await loadAll(core)
    const at0 = await core.visibleInstances([0, 0, 12800, 12800], TREE_GLYPH_MIN_ZOOM)
    expect(at0.count).toBe(5 + 3) // + conifer, deciduous, palm
    const at3 = await core.visibleInstances([0, 0, 12800, 12800], PROP_MIN_ZOOM)
    expect(at3.count).toBe(5 + 3 + 3) // + bush (≥1.5), boulder (≥1), fence (≥3)
  })

  it('hard-caps at the instance budget', async () => {
    const core = makeCore({}, 4)
    await loadAll(core)
    const set = await core.visibleInstances([0, 0, 12800, 12800], -2)
    expect(set.count).toBe(4)
    expect(set.positions.length).toBe(8)
    expect(set.prefabIdx.length).toBe(4)
  })
})

describe('T-090.5.5 — render passthrough + self-hydrating visibleInstances', () => {
  it('narrowPrefabRows carries the render block + spatial.heightM (glyph inputs)', async () => {
    const manifest = await makeCore().loadManifest('everon')
    const tree = manifest?.prefabRows.find((r) => r.prefabId === 0)
    expect(tree?.render?.iconKey).toBe('tree-conifer')
    expect(tree?.render?.baseSizePx).toBe(18)
    expect(tree?.render?.defaultColor).toBe('#2d5a27')
    expect(tree?.spatial?.heightM).toBe(12)
  })

  it('visibleInstances self-hydrates the covering chunks (no prior loadChunksInBbox)', async () => {
    const core = makeCore()
    await core.loadManifest('everon')
    // Chunk 2_1 holds the mixed rows (incl. two trees). Query it at the tree band with NO chunk
    // load first — visibleInstances must hydrate 2_1 itself before the rbush query (it is the
    // sole tree/prop driver; no dependency on the building chunkStore).
    const set = await core.visibleInstances([1024, 512, 1536, 1024], TREE_GLYPH_MIN_ZOOM)
    expect(set.count).toBeGreaterThan(0)
    expect([...set.classes]).toContain(RENDER_CLASS_CODES.indexOf('tree'))
  })

  it('skip-when-invisible: a below-band zoom returns empty without a prior chunk load', async () => {
    const core = makeCore()
    await core.loadManifest('everon')
    expect((await core.visibleInstances([0, 0, 12800, 12800], -3)).count).toBe(0)
  })
})

describe('INSTANCE_BUDGET vs the committed Everon census (data-driven)', () => {
  interface Census {
    byKind: Record<string, { instances: number }>
  }
  const census = JSON.parse(
    readFileSync(`${MAP_ASSETS}/everon/objects/type-inventory.json`, 'utf8'),
  ) as Census

  it('every band boundary where buildings draw fits the hydrated classes in budget', () => {
    // This slice hydrates only the building group (piers ride it). Sum the census kinds
    // that feed that group and assert the budget at every N3 band boundary where the
    // building gate is open — the streamed working set can never exceed the island total.
    const buildingGroupTotal =
      census.byKind.building.instances + census.byKind.water.instances
    const boundaries = [
      -6,
      -4,
      BUILDING_FOOTPRINT_MIN_ZOOM,
      -2,
      TREE_GLYPH_MIN_ZOOM,
      BUILDING_BADGE_MIN_ZOOM,
      VEGETATION_MIN_ZOOM,
      PROP_MIN_ZOOM,
      6,
    ]
    for (const z of boundaries) {
      const drawn = HYDRATE_RENDER_CLASSES.filter((c) => classVisible(c, z)).reduce(
        (sum) => sum + buildingGroupTotal,
        0,
      )
      expect(drawn).toBeLessThanOrEqual(INSTANCE_BUDGET)
    }
    expect(buildingGroupTotal).toBeGreaterThan(0)
  })

  it('trees stay outside the hydrate set below their band (501k never streams at −2)', () => {
    expect(census.byKind.tree.instances).toBeGreaterThan(INSTANCE_BUDGET)
    expect(HYDRATE_RENDER_CLASSES.includes('tree')).toBe(false)
    expect(classVisible('tree', -2)).toBe(false)
  })
})

describe('loadForestMass (T-090.8.1 — TBDD → marching squares in the worker)', () => {
  it('returns geometry at the chunk world origin; empties and misses land in emptyIds', async () => {
    const core = makeCore()
    await core.loadManifest('everon')
    const result = await core.loadForestMass(['1_1', '2_1', '3_1'])
    expect(result.chunks.map((c) => c.id)).toEqual(['1_1'])
    expect(result.emptyIds).toEqual(['2_1', '3_1']) // all-zero grid + missing file
    const chunk = result.chunks[0]
    expect(chunk.cx).toBe(1)
    expect(chunk.cy).toBe(1)
    expect(chunk.treeMax).toBe(4)
    // Fully dense 17×17 grid → one closed quad per 16×16 cell, zero contour segments.
    expect(chunk.fillStartIndices).toHaveLength(256)
    expect(chunk.outlineSegments).toHaveLength(0)
    // First ring starts at the chunk origin (cx·512, cy·512).
    expect(chunk.fillPositions[0]).toBe(512)
    expect(chunk.fillPositions[1]).toBe(512)
    let maxX = 0
    for (let k = 0; k < chunk.fillPositions.length; k += 2) maxX = Math.max(maxX, chunk.fillPositions[k])
    expect(maxX).toBe(1024) // 512 + 16 cells · 32 m
  })

  it('recomputes fresh arrays per call (worker-shell transfer can never detach the cache)', async () => {
    const core = makeCore()
    await core.loadManifest('everon')
    const a = await core.loadForestMass(['1_1'])
    const b = await core.loadForestMass(['1_1'])
    expect(b.chunks[0].fillPositions).not.toBe(a.chunks[0].fillPositions)
    expect([...b.chunks[0].fillPositions]).toEqual([...a.chunks[0].fillPositions])
  })

  it('iso above the grid density empties the chunk (tuning knob)', async () => {
    const core = makeCore()
    await core.loadManifest('everon')
    const result = await core.loadForestMass(['1_1'], 5)
    expect(result.chunks).toHaveLength(0)
    expect(result.emptyIds).toEqual(['1_1'])
  })

  it('manifest without densityPath → everything empty (pre-density exports)', async () => {
    const core = makeCore({ withoutDensity: true })
    await core.loadManifest('everon')
    const result = await core.loadForestMass(['1_1'])
    expect(result.chunks).toHaveLength(0)
    expect(result.emptyIds).toEqual(['1_1'])
    expect((await core.loadManifest('everon'))?.densityPath).toBeNull()
  })

  it('exposes densityPath on the manifest lite', async () => {
    expect((await makeCore().loadManifest('everon'))?.densityPath).toBe('objects/density')
  })
})

describe('resolve + unload', () => {
  it('joins an instance id back to its prefab identity', async () => {
    const core = makeCore()
    await loadAll(core)
    const id = await core.pickNearest([1250.5, 620.25], 1)
    expect(id).toBe('2_1:1')
    const resolved = await core.resolve(id as string)
    expect(resolved?.kind).toBe('water')
    expect(resolved?.class).toBe('pier')
    expect(resolved?.renderClass).toBe('building')
    expect(resolved?.position[0]).toBeCloseTo(1250.5, 3)
    expect(resolved?.rotationDeg).toBeCloseTo(90, 3)
    expect(await core.resolve('9_9:0')).toBeNull()
    expect(await core.resolve('garbage')).toBeNull()
  })

  it('unload drops the index and manifest (terrain switch)', async () => {
    const core = makeCore()
    await loadAll(core)
    await core.unload()
    expect(core.getStatus().ready).toBe(false)
    expect((await core.visibleInstances([0, 0, 12800, 12800], -2)).count).toBe(0)
    expect(await core.pickNearest([1200.25, 650.5], 50)).toBeNull()
  })
})

describe('DEM vector grid (T-090.5.4 — sea band + contours, manifest-orthogonal)', () => {
  // A 3×3 grid: ocean band on the left/top, land ridge on the right — enough to produce both
  // a sea fill and a positive contour, on a core that never loaded an objects manifest (R11).
  const demGrid = {
    data: new Float32Array([
      -20, 40, 100,
      -20, 40, 100,
      -20, 40, 100,
    ]),
    cols: 3,
    rows: 3,
    cellX: 100,
    cellY: 100,
    originX: 0,
    originY: 0,
    maxElevM: 100,
  }

  it('builds sea band + contours from a pushed grid with no manifest loaded', () => {
    const core = createWorldObjectsCore({ fetchBytes: async () => null })
    expect(core.getStatus().ready).toBe(false) // no objects export
    core.setDemGrid({ ...demGrid, data: demGrid.data.slice() })
    const sea = core.buildSeaBand()
    expect(sea && sea.polygonCount).toBeGreaterThan(0)
    const contours = core.buildContours(20)
    expect(contours?.intervalM).toBe(20)
    expect(contours && contours.segments.length).toBeGreaterThan(0)
  })

  it('returns null before a grid is pushed and after unload clears it', async () => {
    const core = createWorldObjectsCore({ fetchBytes: async () => null })
    expect(core.buildSeaBand()).toBeNull()
    expect(core.buildContours(20)).toBeNull()
    core.setDemGrid({ ...demGrid, data: demGrid.data.slice() })
    expect(core.buildSeaBand()).not.toBeNull()
    await core.unload()
    expect(core.buildSeaBand()).toBeNull()
    expect(core.buildContours(20)).toBeNull()
  })
})
