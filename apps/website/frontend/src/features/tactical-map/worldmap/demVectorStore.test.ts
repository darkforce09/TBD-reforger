// T-090.5.4 — DEM-vector store: the DEM→grid→worker→composite pipeline, driven with a fake
// worker client + fake DEM source (no Worker/DemController). Covers push-once, interval cache,
// stale-composite retention, terrain switch, degraded DEM, and the worker-restart re-push.
import { describe, it, expect, vi } from 'vitest'
import { createDemVectorStore, type DemVectorClient, type DemSource } from './demVectorStore'
import type { SeaBandGeometry } from './seaBand'
import type { ContourResult } from '../workers/worldObjectsCore'
import type { TerrainDef } from '../coords/terrains'

const EVERON = { id: 'everon', width: 12800, height: 12800 } as unknown as TerrainDef
const ARLAND = { id: 'arland', width: 4096, height: 4096 } as unknown as TerrainDef

const SEA_GEO: SeaBandGeometry = {
  fillPositions: new Float32Array([0, 0, 1, 0, 1, 1, 0, 0]),
  fillStartIndices: new Uint32Array([0]),
  fillColors: new Uint8Array(16),
  polygonCount: 1,
}

/** Fake worker client: models a worker holding (or having lost) the pushed grid. */
function makeClient() {
  let hasGrid = false
  const calls = { setDemGrid: 0, buildSeaBand: 0, buildContours: [] as number[] }
  let contourOverride: (() => Promise<ContourResult | null>) | null = null
  const client: DemVectorClient = {
    async setDemGrid() {
      calls.setDemGrid++
      hasGrid = true
    },
    async buildSeaBand() {
      calls.buildSeaBand++
      return hasGrid ? SEA_GEO : null
    },
    async buildContours(intervalM: number) {
      calls.buildContours.push(intervalM)
      if (contourOverride) return contourOverride()
      return hasGrid ? { intervalM, segments: new Float32Array([0, 0, 1, 1]) } : null
    },
  }
  return {
    client,
    calls,
    loseGrid: () => {
      hasGrid = false
    },
    setContourOverride: (f: (() => Promise<ContourResult | null>) | null) => {
      contourOverride = f
    },
  }
}

/** Fake DEM source with controllable ready/degraded + a manual notify. */
function makeDem() {
  let ready = true
  let degraded = false
  const listeners = new Set<() => void>()
  const dem: DemSource = {
    load: vi.fn(),
    subscribe(cb) {
      listeners.add(cb)
      return () => listeners.delete(cb)
    },
    getRaster() {
      return ready ? { metersCache: new Float32Array(256).fill(-10), width: 16, height: 16 } : null
    },
    isDegraded() {
      return degraded
    },
  }
  return {
    dem,
    setReady: (v: boolean) => (ready = v),
    setDegraded: (v: boolean) => (degraded = v),
    fire: () => listeners.forEach((l) => l()),
  }
}

const flush = async (): Promise<void> => {
  for (let i = 0; i < 8; i++) await Promise.resolve()
  await new Promise((r) => setTimeout(r, 0))
  for (let i = 0; i < 4; i++) await Promise.resolve()
}

describe('demVectorStore', () => {
  it('DEM-ready: builds the sea band and pushes the grid exactly once (double ensure)', async () => {
    const c = makeClient()
    const d = makeDem()
    const store = createDemVectorStore({ client: c.client, dem: d.dem })
    store.ensureDemVectors(EVERON)
    store.ensureDemVectors(EVERON) // idempotent — started guard
    await flush()
    expect(c.calls.setDemGrid).toBe(1) // one push (first buildSeaBand null → push → retry)
    expect(store.getDemVectors().seaBand.polygonCount).toBe(1)
  })

  it('contour interval is cached — a repeat interval does not re-ask the worker', async () => {
    const c = makeClient()
    const d = makeDem()
    const store = createDemVectorStore({ client: c.client, dem: d.dem })
    store.ensureDemVectors(EVERON)
    await flush()
    store.setContourInterval(20)
    await flush()
    store.setContourInterval(50)
    await flush()
    store.setContourInterval(20) // cached
    await flush()
    expect(c.calls.buildContours.filter((n) => n === 20).length).toBe(1)
    expect(store.getDemVectors().contours.intervalM).toBe(20)
  })

  it('keeps the previous contour composite while a new interval computes (no blanking)', async () => {
    const c = makeClient()
    const d = makeDem()
    const store = createDemVectorStore({ client: c.client, dem: d.dem })
    store.ensureDemVectors(EVERON)
    await flush()
    store.setContourInterval(20)
    await flush()
    expect(store.getDemVectors().contours.intervalM).toBe(20)
    // A pending build for the next interval must not clear the shown composite.
    let resolvePending!: (v: ContourResult | null) => void
    c.setContourOverride(() => new Promise<ContourResult | null>((res) => (resolvePending = res)))
    store.setContourInterval(50)
    await flush()
    expect(store.getDemVectors().contours.intervalM).toBe(20) // old kept
    resolvePending({ intervalM: 50, segments: new Float32Array([0, 0, 2, 2]) })
    await flush()
    expect(store.getDemVectors().contours.intervalM).toBe(50)
  })

  it('terrain switch resets the composites', async () => {
    const c = makeClient()
    const d = makeDem()
    const store = createDemVectorStore({ client: c.client, dem: d.dem })
    store.ensureDemVectors(EVERON)
    await flush()
    expect(store.getDemVectors().seaBand.polygonCount).toBe(1)
    d.setReady(false) // new terrain has no DEM yet
    store.ensureDemVectors(ARLAND)
    await flush()
    expect(store.getDemVectors().seaBand.polygonCount).toBe(0)
  })

  it('degraded DEM (Arland): no grid push, empty composites', async () => {
    const c = makeClient()
    const d = makeDem()
    d.setDegraded(true)
    const store = createDemVectorStore({ client: c.client, dem: d.dem })
    store.ensureDemVectors(EVERON)
    store.setContourInterval(20)
    await flush()
    expect(c.calls.setDemGrid).toBe(0)
    expect(store.getDemVectors().seaBand.polygonCount).toBe(0)
    expect(store.getDemVectors().contours.segmentCount).toBe(0)
  })

  it('worker restart (build returns null) → one re-push + retry', async () => {
    const c = makeClient()
    const d = makeDem()
    const store = createDemVectorStore({ client: c.client, dem: d.dem })
    store.ensureDemVectors(EVERON)
    await flush()
    expect(c.calls.setDemGrid).toBe(1)
    c.loseGrid() // simulate terminateWorldObjects on mission unmount
    store.setContourInterval(20)
    await flush()
    expect(c.calls.setDemGrid).toBe(2) // re-pushed
    expect(store.getDemVectors().contours.intervalM).toBe(20)
  })

  it('late DEM readiness rebuilds when the source notifies', async () => {
    const c = makeClient()
    const d = makeDem()
    d.setReady(false)
    const store = createDemVectorStore({ client: c.client, dem: d.dem })
    store.ensureDemVectors(EVERON)
    await flush()
    expect(store.getDemVectors().seaBand.polygonCount).toBe(0) // not ready yet
    d.setReady(true)
    d.fire() // DEM finished loading
    await flush()
    expect(store.getDemVectors().seaBand.polygonCount).toBe(1)
  })
})
