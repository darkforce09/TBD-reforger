// T-090.5.3 — Main-thread chunk store gates: skip-when-invisible, preload chunk-set math
// (border + oversized ring), request dedupe, LRU eviction that never touches the pinned
// visible set, the ≤4 ms/frame apply budget (fake clock), and composite parity with the
// T-090.5.2 loader output (the "roads/buildings look identical" regression bar).
import { describe, it, expect } from 'vitest'
import {
  APPLY_BUDGET_MS,
  HYDRATE_RENDER_CLASSES,
  LRU_MIN_CHUNKS,
  createChunkStore,
  type WorldStreamClient,
} from './chunkStore'
import { chunkIdsForViewport, type Bbox } from './chunkMath'
import { buildingPrefabLookup, buildingsFromChunkInstances } from './buildingLayer'
import { TERRAINS } from '../coords/terrains'
import type {
  ChunkLoadResult,
  ChunkPayload,
  LoadChunksOpts,
  WorldManifestLite,
  WorldPrefabRow,
} from '../workers/worldObjectsCore'

const EVERON = TERRAINS.everon
const CHUNK = 512
const GRID = EVERON.width / CHUNK // 25×25 cells

/** Two building-group prefabs (f32-exact extents) — enough for parity + streaming checks. */
const PREFABS: WorldPrefabRow[] = [
  {
    prefabId: 9,
    kind: 'building',
    class: 'residential',
    spatial: { halfExtentsM: { x: 5, y: 5, z: 4 } },
  },
  { prefabId: 7, kind: 'water', class: 'pier', spatial: { halfExtentsM: { x: 4, y: 1, z: 0.5 } } },
  { prefabId: 0, kind: 'tree', class: 'conifer', spatial: { halfExtentsM: { x: 1, y: 1, z: 6 } } },
]

/** Deterministic per-chunk rows: one building + one pier per cell, at f32-exact offsets. */
function rowsFor(cx: number, cy: number): [number, number, number, number, number][] {
  const x = cx * CHUNK
  const y = cy * CHUNK
  return [
    [9, x + 100.5, y + 200.25, 10, 45],
    [7, x + 300.25, y + 400.5, 0.5, 90],
  ]
}

function buildingGroup(rows: [number, number, number, number, number][]) {
  const wanted = rows.filter(([pid]) => pid === 9 || pid === 7)
  return {
    count: wanted.length,
    positions: Float32Array.from(wanted.flatMap(([, x, y]) => [x, y])),
    prefabIdx: Uint16Array.from(wanted.map(([pid]) => pid)),
    rotations: Float32Array.from(wanted.map(([, , , , rot]) => rot)),
    z: Float32Array.from(wanted.map(([, , , z]) => z)),
  }
}

interface Harness {
  client: WorldStreamClient
  calls: { requestedIds: string[][]; manifestCalls: number; unloads: number }
  /** Resolve queued manifest/chunk promises (client is async — flush microtasks). */
  flush: () => Promise<void>
  /** Run n scheduled apply frames. */
  pump: (n?: number) => void
  now: { t: number; step: number }
  store: ReturnType<typeof createChunkStore>
}

function makeHarness(opts: { hasOversized?: boolean; missingIds?: string[] } = {}): Harness {
  const calls = { requestedIds: [] as string[][], manifestCalls: 0, unloads: 0 }
  const missing = new Set(opts.missingIds ?? [])
  const manifest: WorldManifestLite = {
    terrainId: 'everon',
    chunkSizeM: CHUNK,
    cells: Array.from({ length: GRID * GRID }, (_, i) => {
      const cx = i % GRID
      const cy = Math.floor(i / GRID)
      return { id: `${cx}_${cy}`, cx, cy, path: `objects/chunks/${cx}_${cy}.json.gz` }
    }),
    prefabRows: PREFABS,
    roadsPath: null,
    instanceCount: GRID * GRID * 2,
    hasOversized: opts.hasOversized ?? false,
  }
  const client: WorldStreamClient = {
    async loadManifest() {
      calls.manifestCalls++
      return manifest
    },
    async loadChunksInBbox(_bbox: Bbox, _margin: number, o: LoadChunksOpts) {
      const ids = o.ids ?? []
      calls.requestedIds.push([...ids])
      const chunks: ChunkPayload[] = []
      for (const id of ids) {
        if (missing.has(id)) continue
        const [cx, cy] = id.split('_').map(Number)
        chunks.push({
          id,
          cx,
          cy,
          totalInstances: 2,
          groups: { building: buildingGroup(rowsFor(cx, cy)) },
        })
      }
      const result: ChunkLoadResult = { chunkSizeM: CHUNK, chunks }
      return result
    },
    async unload() {
      calls.unloads++
    },
  }
  const scheduled: (() => void)[] = []
  const now = { t: 0, step: 0 }
  const store = createChunkStore({
    client,
    now: () => {
      const v = now.t
      now.t += now.step
      return v
    },
    schedule: (cb) => scheduled.push(cb),
  })
  return {
    client,
    calls,
    flush: async () => {
      // Two microtask turns cover promise→then chains in the store.
      await Promise.resolve()
      await Promise.resolve()
      await Promise.resolve()
    },
    pump: (n = 50) => {
      for (let i = 0; i < n && scheduled.length > 0; i++) {
        const cb = scheduled.shift() as () => void
        cb()
      }
    },
    now,
    store,
  }
}

/** Boot a harness to ready state with one viewport applied. */
async function boot(h: Harness, bbox: Bbox, zoom = -2): Promise<void> {
  h.store.ensureWorldStream(EVERON)
  await h.flush()
  h.store.setWorldViewport(bbox, zoom)
  await h.flush()
  h.pump()
}

const VP_A: Bbox = [2000, 2000, 2200, 2200]

describe('skip-when-invisible (plan §6)', () => {
  it('does not hydrate below the building band; opens at −2.5', async () => {
    const h = makeHarness()
    h.store.ensureWorldStream(EVERON)
    await h.flush()
    h.store.setWorldViewport(VP_A, -3)
    await h.flush()
    expect(h.calls.requestedIds.length).toBe(0)
    expect(HYDRATE_RENDER_CLASSES.every((c) => c === 'building')).toBe(true)

    h.store.setWorldViewport(VP_A, -2)
    await h.flush()
    expect(h.calls.requestedIds.length).toBe(1)
  })

  it('zooming back out releases pins and empties the composite (cache kept)', async () => {
    const h = makeHarness()
    await boot(h, VP_A)
    expect(h.store.getWorldBuildings().length).toBeGreaterThan(0)
    h.store.setWorldViewport(VP_A, -4)
    expect(h.store.getWorldBuildings()).toEqual([])
    // Zoom back in: chunks come straight from cache — no new client request.
    const before = h.calls.requestedIds.length
    h.store.setWorldViewport(VP_A, -2)
    await h.flush()
    h.pump()
    expect(h.calls.requestedIds.length).toBe(before)
    expect(h.store.getWorldBuildings().length).toBeGreaterThan(0)
  })
})

describe('chunk-set math (border preload + oversized ring)', () => {
  it('requests exactly the chunkMath preload set', async () => {
    const h = makeHarness()
    await boot(h, VP_A)
    const expected = chunkIdsForViewport(VP_A, EVERON, { chunkSizeM: CHUNK })
    expect(h.calls.requestedIds[0]).toEqual(expected)
  })

  it('adds one extra ring when the export carries oversized prefabs', async () => {
    const h = makeHarness({ hasOversized: true })
    await boot(h, VP_A)
    const expected = chunkIdsForViewport(VP_A, EVERON, { chunkSizeM: CHUNK, extraRing: 1 })
    expect(h.calls.requestedIds[0]).toEqual(expected)
  })
})

describe('request dedupe', () => {
  it('same viewport twice → one request; unchanged set early-exits', async () => {
    const h = makeHarness()
    await boot(h, VP_A)
    h.store.setWorldViewport(VP_A, -2)
    h.store.setWorldViewport([2001, 2001, 2201, 2201], -2) // same chunk rect
    await h.flush()
    expect(h.calls.requestedIds.length).toBe(1)
  })

  it('in-flight chunks are not re-requested by an overlapping viewport', async () => {
    const h = makeHarness()
    h.store.ensureWorldStream(EVERON)
    await h.flush()
    h.store.setWorldViewport(VP_A, -2)
    // No flush: first request still in flight. Pan east — the new preload set overlaps the
    // in-flight one; only the genuinely new chunks may be requested.
    h.store.setWorldViewport([3000, 2000, 3200, 2200], -2)
    await h.flush()
    expect(h.calls.requestedIds.length).toBe(2)
    const [first, second] = h.calls.requestedIds
    expect(second.length).toBeGreaterThan(0)
    expect(second.every((id) => !first.includes(id))).toBe(true)
  })

  it('missing chunk files are cached as hydrated-empty (no refetch loop)', async () => {
    const h = makeHarness({ missingIds: chunkIdsForViewport(VP_A, EVERON, { chunkSizeM: CHUNK }) })
    await boot(h, VP_A)
    expect(h.store.getWorldBuildings()).toEqual([])
    // Nudge within the same rect, then to a new rect and back: the missing ids never re-fetch.
    h.store.setWorldViewport([4000, 4000, 4200, 4200], -2)
    await h.flush()
    h.pump()
    h.store.setWorldViewport(VP_A, -2)
    await h.flush()
    const requestedAgain = h.calls.requestedIds
      .slice(1)
      .flat()
      .some((id) => h.calls.requestedIds[0].includes(id))
    expect(requestedAgain).toBe(false)
  })
})

describe('apply budget (≤4 ms/frame, plan §6 hydrate budget)', () => {
  it('drains the queue across frames under a slow clock and records stats', async () => {
    const h = makeHarness()
    h.store.ensureWorldStream(EVERON)
    await h.flush()
    // 3 ms per now() call: each frame fits exactly one chunk apply before the budget trips.
    h.now.step = 3
    h.store.setWorldViewport(VP_A, -2)
    await h.flush()
    const queued = h.calls.requestedIds[0].length
    expect(queued).toBeGreaterThan(4)
    // One scheduled drain exists; each run applies one chunk then reschedules.
    for (let i = 0; i < queued; i++) h.pump(1)
    const stats = h.store.getWorldStreamStats()
    expect(stats.chunksApplied).toBe(queued)
    expect(stats.applyFrames).toBe(queued)
    expect(stats.maxApplyMs).toBeGreaterThan(0)
    // The fake clock bills ~3 ms per check, so the over-budget counter engages — proving
    // the instrumentation path (real applies are far under 4 ms; see verify log numbers).
    expect(stats.framesOverBudget).toBeGreaterThan(0)
    expect(APPLY_BUDGET_MS).toBe(4)
  })

  it('fast clock applies the whole queue in one frame', async () => {
    const h = makeHarness()
    await boot(h, VP_A) // now.step = 0 → zero-cost applies
    const stats = h.store.getWorldStreamStats()
    expect(stats.applyFrames).toBe(1)
    expect(stats.chunksApplied).toBe(h.calls.requestedIds[0].length)
    expect(stats.framesOverBudget).toBe(0)
  })
})

describe('LRU (cap = max(64, 3× viewport), pinned never evicted)', () => {
  it('keeps a revisited nearby viewport fully cached under the cap', async () => {
    const h = makeHarness()
    await boot(h, VP_A)
    h.store.setWorldViewport([2600, 2000, 2800, 2200], -2)
    await h.flush()
    h.pump()
    const before = h.calls.requestedIds.length
    h.store.setWorldViewport(VP_A, -2)
    await h.flush()
    h.pump()
    // Everything still cached → zero new requests.
    expect(h.calls.requestedIds.length).toBe(before)
  })

  it('evicts the oldest unpinned chunks once the cache exceeds the cap', async () => {
    const h = makeHarness()
    await boot(h, VP_A)
    const firstSet = new Set(h.calls.requestedIds[0])
    // Sweep far across the island: each stop pins ~16 chunks; the cap is 64, so the first
    // viewport's chunks age out. The current viewport must never be evicted (always cached
    // on immediate revisit).
    const stops: Bbox[] = Array.from({ length: 10 }, (_, i) => {
      const x = 3000 + i * 1024
      return [x, 6000, x + 200, 6200] as Bbox
    })
    for (const bbox of stops) {
      h.store.setWorldViewport(bbox, -2)
      await h.flush()
      h.pump()
    }
    expect(LRU_MIN_CHUNKS).toBe(64)
    // Current viewport revisit: still pinned+cached → no request.
    const beforePinned = h.calls.requestedIds.length
    h.store.setWorldViewport(stops[stops.length - 1], -2)
    await h.flush()
    expect(h.calls.requestedIds.length).toBe(beforePinned)
    // First viewport was evicted → returning re-requests (at least some of) its ids.
    h.store.setWorldViewport(VP_A, -2)
    await h.flush()
    const last = h.calls.requestedIds[h.calls.requestedIds.length - 1]
    expect(last.some((id) => firstSet.has(id))).toBe(true)
  })
})

describe('composite parity with the T-090.5.2 loader (regression bar)', () => {
  it('streamed BuildingInstance rows equal buildingsFromChunkInstances output', async () => {
    const h = makeHarness()
    await boot(h, VP_A)
    const lookup = buildingPrefabLookup({ prefabs: PREFABS })
    const ids = chunkIdsForViewport(VP_A, EVERON, { chunkSizeM: CHUNK })
    const expected = [...ids].sort().flatMap((id) => {
      const [cx, cy] = id.split('_').map(Number)
      return buildingsFromChunkInstances(rowsFor(cx, cy), lookup)
    })
    expect(h.store.getWorldBuildings()).toEqual(expected)
  })
})

describe('lifecycle', () => {
  it('reset clears state + unloads the worker side; ensure reloads after reset', async () => {
    const h = makeHarness()
    await boot(h, VP_A)
    h.store.resetWorldStream()
    expect(h.calls.unloads).toBe(1)
    expect(h.store.getWorldStreamStatus()).toBe('idle')
    expect(h.store.getWorldBuildings()).toEqual([])
    h.store.ensureWorldStream(EVERON)
    await h.flush()
    expect(h.calls.manifestCalls).toBe(2)
    expect(h.store.getWorldStreamStatus()).toBe('ready')
  })

  it('ensureWorldStream is idempotent per terrain', async () => {
    const h = makeHarness()
    h.store.ensureWorldStream(EVERON)
    h.store.ensureWorldStream(EVERON)
    await h.flush()
    h.store.ensureWorldStream(EVERON)
    expect(h.calls.manifestCalls).toBe(1)
  })

  it('viewport set before the manifest resolves replays once ready', async () => {
    const h = makeHarness()
    h.store.ensureWorldStream(EVERON)
    h.store.setWorldViewport(VP_A, -2) // manifest still loading
    expect(h.calls.requestedIds.length).toBe(0)
    await h.flush()
    h.pump()
    expect(h.calls.requestedIds.length).toBe(1)
    expect(h.store.getWorldBuildings().length).toBeGreaterThan(0)
  })

  it('revision bumps notify subscribers on data commits', async () => {
    const h = makeHarness()
    let notified = 0
    h.store.subscribeWorldStream(() => notified++)
    await boot(h, VP_A)
    expect(notified).toBeGreaterThan(0)
    expect(h.store.getWorldRevision()).toBeGreaterThan(0)
  })
})
