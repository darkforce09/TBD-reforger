// T-090.5.4 — Main-thread DEM-vector store: owns the sea-band + contour geometry composites
// and the pipeline that produces them (DEM ready → downsample → push grid to the world-objects
// worker → build sea band + current contour interval). Same shape as forestMassStore: a factory
// with injectable deps (worker client + DEM source) for node vitest; the app uses the singleton.
//
// Grid is NEVER retained main-side (plan): the downsampled buffer is TRANSFERRED to the worker.
// If the worker was later terminated (mission unmount → terminateWorldObjects), a build returns
// null; withGrid re-downsamples from DemController's surviving meters cache, re-pushes, retries
// once. Geometry is whole-island + static (landcover precedent) — nothing here runs per frame,
// so the composites double as the useSyncExternalStore snapshot (stable ref between commits).

import { yieldToUi } from '../state/yieldToUi'
import {
  DEM_VECTOR_GRID_FACTOR,
  demGridDims,
  downsampleDemGridBand,
  type DemVectorGrid,
} from './demGrid'
import { EMPTY_SEA_BAND, type SeaBandGeometry } from './seaBand'
import {
  loadDemForTerrain,
  subscribeDem,
  getDemRasterForOverlay,
  isDemDegraded,
} from '../dem/DemController'
import {
  buildWorldContours,
  buildWorldSeaBand,
  setWorldDemGrid,
} from '../workers/worldObjectsClient'
import type { ContourResult } from '../workers/worldObjectsCore'
import type { TerrainDef } from '../coords/terrains'
import type { TerrainId } from '../coords/terrains'

/** Contour geometry for the active interval (Deck line wire form + count for the layer). */
export interface ContourComposite {
  intervalM: number
  segments: Float32Array
  segmentCount: number
}

export const EMPTY_CONTOURS: ContourComposite = {
  intervalM: 0,
  segments: new Float32Array(0),
  segmentCount: 0,
}

/** Combined snapshot — a fresh object on each commit (useSyncExternalStore identity). */
export interface DemVectorSnapshot {
  seaBand: SeaBandGeometry
  contours: ContourComposite
}

/** Worker-client surface the store consumes (injectable for tests). */
export interface DemVectorClient {
  setDemGrid(grid: DemVectorGrid): Promise<void>
  buildSeaBand(): Promise<SeaBandGeometry | null>
  buildContours(intervalM: number): Promise<ContourResult | null>
}

/** DEM raster source (DemController singleton in the app; a fake in tests). */
export interface DemSource {
  load(terrainId: string): Promise<void> | void
  subscribe(cb: () => void): () => void
  getRaster(): { metersCache: ArrayLike<number>; width: number; height: number } | null
  isDegraded(): boolean
}

export interface DemVectorStore {
  ensureDemVectors(terrain: TerrainDef): void
  setContourInterval(intervalM: number): void
  getDemVectors(): DemVectorSnapshot
  getDemVectorsRevision(): number
  subscribeDemVectors(cb: () => void): () => void
  resetDemVectors(): void
}

export function createDemVectorStore(deps: { client: DemVectorClient; dem: DemSource }): DemVectorStore {
  const { client, dem } = deps

  let terrain: TerrainDef | null = null
  let started = false
  let demUnsub: (() => void) | null = null

  let seaBand: SeaBandGeometry = EMPTY_SEA_BAND
  let contours: ContourComposite = EMPTY_CONTOURS
  /** Latest requested interval (a stale async reply for a superseded interval is dropped). */
  let requestedInterval = 0
  /** Computed intervals kept so switching zoom bands back is instant (segments per interval). */
  const contourCache = new Map<number, Float32Array>()
  let seaBuilt = false

  let pushInflight: Promise<boolean> | null = null

  let revision = 0
  let snapshot: DemVectorSnapshot = { seaBand, contours }
  const listeners = new Set<() => void>()

  const notify = (): void => {
    revision++
    snapshot = { seaBand, contours }
    listeners.forEach((l) => l())
  }

  /** Downsample DemController's meters cache into a fresh grid, yielding between row bands so
   *  the one-time ~40–80 ms pass never blocks a frame. Reads the LIVE cache but writes a fresh
   *  buffer — the live cache is never transferred (would detach hillshade + sampleElevation). */
  async function downsampleCurrent(): Promise<DemVectorGrid | null> {
    const raster = dem.getRaster()
    if (!raster || !terrain) return null
    const { metersCache, width, height } = raster
    const { cols, rows } = demGridDims(width, height, DEM_VECTOR_GRID_FACTOR)
    const out = new Float32Array(cols * rows)
    let max = -Infinity
    const BAND = 256
    for (let j0 = 0; j0 < rows; j0 += BAND) {
      const j1 = Math.min(rows, j0 + BAND)
      const m = downsampleDemGridBand(metersCache, width, height, DEM_VECTOR_GRID_FACTOR, out, j0, j1)
      if (m > max) max = m
      if (j1 < rows) await yieldToUi()
    }
    return {
      data: out,
      cols,
      rows,
      cellX: terrain.width / (cols - 1),
      cellY: terrain.height / (rows - 1),
      originX: 0,
      originY: 0,
      maxElevM: max,
    }
  }

  /** Downsample + push the grid to the worker (transfer). Returns false when the DEM has no
   *  raster (not ready / degraded) so callers fall through to empty composites. Concurrent
   *  callers share one in-flight push; a completed push is not memoized (a later call re-pushes
   *  — the worker-restart recovery path). */
  function pushGrid(): Promise<boolean> {
    if (pushInflight) return pushInflight
    const forTerrain = terrain?.id
    pushInflight = (async () => {
      if (dem.isDegraded()) return false
      const grid = await downsampleCurrent()
      if (!grid || terrain?.id !== forTerrain) return false
      await client.setDemGrid(grid)
      return true
    })()
      .catch(() => false)
      .finally(() => {
        pushInflight = null
      })
    return pushInflight
  }

  /** Run a worker build; a null reply means the worker lost its grid (restart) → re-push once
   *  and retry. Returns null when there is genuinely no grid (degraded / not ready). */
  async function withGrid<T>(build: () => Promise<T | null>): Promise<T | null> {
    let r = await build()
    if (r === null && (await pushGrid())) r = await build()
    return r
  }

  async function refreshSeaBand(forTerrain: string | undefined): Promise<void> {
    const geo = await withGrid(() => client.buildSeaBand())
    if (terrain?.id !== forTerrain) return
    if (geo) {
      seaBand = geo
      seaBuilt = true
      notify()
    }
  }

  async function refreshContours(intervalM: number, forTerrain: string | undefined): Promise<void> {
    const cached = contourCache.get(intervalM)
    if (cached) {
      if (requestedInterval === intervalM && terrain?.id === forTerrain) {
        contours = { intervalM, segments: cached, segmentCount: cached.length / 4 }
        notify()
      }
      return
    }
    const result = await withGrid(() => client.buildContours(intervalM))
    if (terrain?.id !== forTerrain || !result) return
    contourCache.set(result.intervalM, result.segments)
    // Drop a stale reply for a superseded interval (kept in cache, just not shown).
    if (requestedInterval !== result.intervalM) return
    contours = {
      intervalM: result.intervalM,
      segments: result.segments,
      segmentCount: result.segments.length / 4,
    }
    notify()
  }

  function onDemChange(forTerrain: string | undefined): void {
    if (terrain?.id !== forTerrain) return
    if (dem.isDegraded()) {
      // Arland/no-DEM: leave empty composites (layers cleanly absent — plan R11).
      return
    }
    if (!dem.getRaster()) return // still loading
    if (!seaBuilt) void refreshSeaBand(forTerrain)
    if (requestedInterval > 0) void refreshContours(requestedInterval, forTerrain)
  }

  function resetLocal(): void {
    seaBand = EMPTY_SEA_BAND
    contours = EMPTY_CONTOURS
    contourCache.clear()
    seaBuilt = false
    requestedInterval = 0
    pushInflight = null
  }

  return {
    ensureDemVectors(t: TerrainDef): void {
      if (terrain?.id === t.id && started) return
      if (terrain && terrain.id !== t.id) {
        resetLocal()
        notify() // commit the cleared composites so the old terrain's layers drop immediately
      }
      terrain = t
      started = true
      demUnsub?.()
      demUnsub = dem.subscribe(() => onDemChange(t.id))
      void dem.load(t.id as TerrainId)
      // DEM may already be ready (cached from hillshade/Z) — try immediately.
      onDemChange(t.id)
    },

    setContourInterval(intervalM: number): void {
      if (intervalM <= 0 || !terrain) return
      if (requestedInterval === intervalM && contours.intervalM === intervalM) return
      requestedInterval = intervalM
      void refreshContours(intervalM, terrain.id)
    },

    getDemVectors(): DemVectorSnapshot {
      return snapshot
    },

    getDemVectorsRevision(): number {
      return revision
    },

    subscribeDemVectors(cb: () => void): () => void {
      listeners.add(cb)
      return () => listeners.delete(cb)
    },

    resetDemVectors(): void {
      demUnsub?.()
      demUnsub = null
      terrain = null
      started = false
      resetLocal()
      snapshot = { seaBand, contours }
      notify()
    },
  }
}

const defaultStore = createDemVectorStore({
  client: {
    setDemGrid: setWorldDemGrid,
    buildSeaBand: buildWorldSeaBand,
    buildContours: buildWorldContours,
  },
  dem: {
    load: (id) => loadDemForTerrain(id as TerrainId),
    subscribe: subscribeDem,
    getRaster: getDemRasterForOverlay,
    isDegraded: isDemDegraded,
  },
})

export const {
  ensureDemVectors,
  setContourInterval,
  getDemVectors,
  getDemVectorsRevision,
  subscribeDemVectors,
  resetDemVectors,
} = defaultStore
