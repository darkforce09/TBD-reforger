// T-090.8.1 — Forest-mass store gates: viewport → missing-density-only requests, request
// dedupe on an unchanged chunk set, hydrated-empty caching (missing/zero grids never
// refetch), permanent chunk cache (pan away + back = zero requests — N11 P2b pinned
// policy), deterministic composite concatenation, and the absent path for terrains/exports
// without density grids.
import { describe, it, expect } from 'vitest'
import {
  EMPTY_FOREST_COMPOSITE,
  createForestMassStore,
  type ForestMassClient,
} from './forestMassStore'
import { chunkIdsForViewport, type Bbox } from './chunkMath'
import { TERRAINS } from '../coords/terrains'
import type { ForestMassChunk, WorldManifestLite } from '../workers/worldObjectsCore'

const EVERON = TERRAINS.everon
const CHUNK = 512

/** One deterministic triangle + one contour segment per chunk (world-origin offset). */
function chunkGeom(cx: number, cy: number): ForestMassChunk {
  const x = cx * CHUNK
  const y = cy * CHUNK
  return {
    id: `${cx}_${cy}`,
    cx,
    cy,
    fillPositions: Float32Array.from([x, y, x + 24, y, x, y + 24, x, y]),
    fillStartIndices: Uint32Array.from([0]),
    outlineSegments: Float32Array.from([x + 24, y, x, y + 24]),
    treeMax: 4,
  }
}

interface Harness {
  store: ReturnType<typeof createForestMassStore>
  calls: { requested: string[][]; manifestCalls: number }
  flush: () => Promise<void>
}

function makeHarness(opts: { densityPath?: string | null; emptyIds?: string[] } = {}): Harness {
  const calls = { requested: [] as string[][], manifestCalls: 0 }
  const empty = new Set(opts.emptyIds ?? [])
  const manifest: WorldManifestLite = {
    terrainId: 'everon',
    chunkSizeM: CHUNK,
    cells: null,
    prefabRows: [],
    roadsPath: null,
    densityPath: opts.densityPath === undefined ? 'objects/density' : opts.densityPath,
    instanceCount: null,
    hasOversized: false,
  }
  const client: ForestMassClient = {
    async loadManifest() {
      calls.manifestCalls++
      return manifest
    },
    async loadForestMass(ids: string[]) {
      calls.requested.push([...ids])
      const chunks: ForestMassChunk[] = []
      const emptyIds: string[] = []
      for (const id of ids) {
        if (empty.has(id)) {
          emptyIds.push(id)
          continue
        }
        const [cx, cy] = id.split('_').map(Number)
        chunks.push(chunkGeom(cx, cy))
      }
      return { chunks, emptyIds }
    },
  }
  return {
    store: createForestMassStore({ client }),
    calls,
    flush: async () => {
      await Promise.resolve()
      await Promise.resolve()
      await Promise.resolve()
    },
  }
}

/** A one-chunk viewport square inside chunk (cx, cy). */
function viewportOver(cx: number, cy: number): Bbox {
  const x = cx * CHUNK + 200
  const y = cy * CHUNK + 200
  return [x, y, x + 50, y + 50]
}

describe('forestMassStore streaming', () => {
  it('requests exactly the viewport chunk set (border preload) and composites the result', async () => {
    const h = makeHarness()
    h.store.ensureForestStream(EVERON)
    await h.flush()
    const bbox = viewportOver(10, 10)
    h.store.setForestViewport(bbox)
    await h.flush()
    const expected = chunkIdsForViewport(bbox, EVERON, { chunkSizeM: CHUNK })
    expect(h.calls.requested).toEqual([expected])
    const mass = h.store.getForestMass()
    expect(mass.chunkCount).toBe(expected.length)
    expect(mass.polygonCount).toBe(expected.length)
    expect(mass.segmentCount).toBe(expected.length)
    expect(mass.fillPositions).toHaveLength(8 * expected.length)
  })

  it('startIndices are rebased across chunks (composite is one flat binary buffer)', async () => {
    const h = makeHarness()
    h.store.ensureForestStream(EVERON)
    await h.flush()
    h.store.setForestViewport(viewportOver(5, 5))
    await h.flush()
    const mass = h.store.getForestMass()
    // Each fixture ring holds 4 vertices → starts at 0, 4, 8, …
    expect([...mass.fillStartIndices]).toEqual(
      Array.from({ length: mass.polygonCount }, (_, k) => 4 * k),
    )
  })

  it('unchanged viewport chunk set → no second request; new chunks only on pan', async () => {
    const h = makeHarness()
    h.store.ensureForestStream(EVERON)
    await h.flush()
    const bbox = viewportOver(10, 10)
    h.store.setForestViewport(bbox)
    await h.flush()
    h.store.setForestViewport([bbox[0] + 1, bbox[1] + 1, bbox[2] + 1, bbox[3] + 1])
    await h.flush()
    expect(h.calls.requested).toHaveLength(1) // same chunk set → early-exit
    const first = new Set(h.calls.requested[0])
    h.store.setForestViewport(viewportOver(12, 10))
    await h.flush()
    expect(h.calls.requested).toHaveLength(2)
    for (const id of h.calls.requested[1]) expect(first.has(id)).toBe(false)
  })

  it('caches chunks permanently — pan away and back issues zero new requests', async () => {
    const h = makeHarness()
    h.store.ensureForestStream(EVERON)
    await h.flush()
    h.store.setForestViewport(viewportOver(10, 10))
    await h.flush()
    h.store.setForestViewport(viewportOver(12, 10))
    await h.flush()
    const countAfterTwo = h.calls.requested.length
    const massAfterTwo = h.store.getForestMass()
    h.store.setForestViewport(viewportOver(10, 10))
    await h.flush()
    expect(h.calls.requested).toHaveLength(countAfterTwo)
    // Composite keeps everything hydrated so far (no eviction) — same reference, pan-stable.
    expect(h.store.getForestMass()).toBe(massAfterTwo)
  })

  it('hydrated-empty ids (missing/zero grids) never refetch and add no geometry', async () => {
    const h = makeHarness({ emptyIds: ['10_10'] })
    h.store.ensureForestStream(EVERON)
    await h.flush()
    const bbox = viewportOver(10, 10)
    h.store.setForestViewport(bbox)
    await h.flush()
    const expected = chunkIdsForViewport(bbox, EVERON, { chunkSizeM: CHUNK })
    expect(h.store.getForestMass().chunkCount).toBe(expected.length - 1)
    h.store.setForestViewport(viewportOver(12, 12))
    await h.flush()
    h.store.setForestViewport(bbox)
    await h.flush()
    expect(h.calls.requested.flat().filter((id) => id === '10_10')).toHaveLength(1)
  })

  it('revision bumps on apply; snapshot reference is stable between commits', async () => {
    const h = makeHarness()
    h.store.ensureForestStream(EVERON)
    await h.flush()
    const r0 = h.store.getForestRevision()
    expect(h.store.getForestMass()).toBe(EMPTY_FOREST_COMPOSITE)
    h.store.setForestViewport(viewportOver(3, 3))
    await h.flush()
    expect(h.store.getForestRevision()).toBeGreaterThan(r0)
    const snap = h.store.getForestMass()
    expect(h.store.getForestMass()).toBe(snap)
  })

  it('viewport before the manifest resolves replays once ready', async () => {
    const h = makeHarness()
    h.store.ensureForestStream(EVERON)
    h.store.setForestViewport(viewportOver(4, 4)) // manifest still in flight
    await h.flush()
    expect(h.calls.requested).toHaveLength(1)
    expect(h.store.getForestMass().chunkCount).toBeGreaterThan(0)
  })

  it('no densityPath in the manifest → absent, zero requests (plan R11)', async () => {
    const h = makeHarness({ densityPath: null })
    h.store.ensureForestStream(EVERON)
    await h.flush()
    h.store.setForestViewport(viewportOver(10, 10))
    await h.flush()
    expect(h.calls.requested).toHaveLength(0)
    expect(h.store.getForestMass()).toBe(EMPTY_FOREST_COMPOSITE)
  })

  it('resetForestStream drops cache and composite', async () => {
    const h = makeHarness()
    h.store.ensureForestStream(EVERON)
    await h.flush()
    h.store.setForestViewport(viewportOver(10, 10))
    await h.flush()
    h.store.resetForestStream()
    expect(h.store.getForestMass()).toBe(EMPTY_FOREST_COMPOSITE)
    h.store.ensureForestStream(EVERON)
    await h.flush()
    h.store.setForestViewport(viewportOver(10, 10))
    await h.flush()
    expect(h.calls.requested).toHaveLength(2) // refetch after reset — cache was dropped
  })
})
