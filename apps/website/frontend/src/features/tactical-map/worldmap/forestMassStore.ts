// T-090.8.1 — Main-thread forest-mass store: viewport-driven streaming of TBDD density
// chunks through the world-objects worker (marching squares runs worker-side; typed arrays
// arrive as transferables). Deliberately simpler than chunkStore: density geometry is tiny
// (625 Everon chunks ≈ 0.4 MB grids → a few MB of polygons), so chunks are cached for the
// session with NO eviction and the composite covers everything loaded so far — the N11 P2b
// "region index pinned" policy. Zoom never invalidates data: the α ladder + class gates are
// layer-builder concerns (forestMassLayer/lodGates); this store only fetches and composites.
//
// The composite arrays' references only change when a hydration batch commits, so
// getForestMass() doubles as the useSyncExternalStore snapshot (T-057 pan-stability rule:
// nothing here runs per frame — setForestViewport early-exits on an unchanged chunk set).
//
// Factory + module default instance, same test shape as chunkStore: tests inject a fake
// client; the app uses the singleton wired to worldObjectsClient.

import { chunkIdsForViewport, type Bbox } from './chunkMath'
import { loadWorldForestMass, loadWorldManifest } from '../workers/worldObjectsClient'
import type { ForestMassChunk, ForestMassResult, WorldManifestLite } from '../workers/worldObjectsCore'
import type { TerrainDef } from '../coords/terrains'

/** Concatenated marching-squares geometry across every hydrated chunk (Deck binary form). */
export interface ForestMassComposite {
  fillPositions: Float32Array
  /** Per-ring start VERTEX index (no trailing sentinel). */
  fillStartIndices: Uint32Array
  outlineSegments: Float32Array
  polygonCount: number
  segmentCount: number
  chunkCount: number
}

export const EMPTY_FOREST_COMPOSITE: ForestMassComposite = {
  fillPositions: new Float32Array(0),
  fillStartIndices: new Uint32Array(0),
  outlineSegments: new Float32Array(0),
  polygonCount: 0,
  segmentCount: 0,
  chunkCount: 0,
}

/** The worker-client surface the store consumes (injectable for tests). */
export interface ForestMassClient {
  loadManifest(terrainId: string): Promise<WorldManifestLite | null>
  loadForestMass(ids: string[]): Promise<ForestMassResult>
}

export interface ForestMassStore {
  ensureForestStream(terrain: TerrainDef): void
  setForestViewport(bbox: Bbox | null): void
  getForestMass(): ForestMassComposite
  getForestRevision(): number
  subscribeForestStream(cb: () => void): () => void
  resetForestStream(): void
}

export function createForestMassStore(deps: { client: ForestMassClient }): ForestMassStore {
  const { client } = deps

  let terrain: TerrainDef | null = null
  let manifest: WorldManifestLite | null = null
  let started = false
  /** chunkId → geometry (null = hydrated-empty: missing/zero density — never re-requested). */
  const cache = new Map<string, ForestMassChunk | null>()
  const inflight = new Set<string>()
  let lastKey = ''
  let lastViewport: Bbox | null = null

  let composite: ForestMassComposite = EMPTY_FOREST_COMPOSITE
  let revision = 0
  const listeners = new Set<() => void>()

  const notify = (): void => {
    revision++
    listeners.forEach((l) => l())
  }

  function rebuildComposite(): void {
    // Stable id order → deterministic composite regardless of fetch completion order.
    const loaded: ForestMassChunk[] = []
    let vertexTotal = 0
    let ringTotal = 0
    let segFloatTotal = 0
    for (const id of [...cache.keys()].sort()) {
      const chunk = cache.get(id)
      if (!chunk) continue
      loaded.push(chunk)
      vertexTotal += chunk.fillPositions.length / 2
      ringTotal += chunk.fillStartIndices.length
      segFloatTotal += chunk.outlineSegments.length
    }
    if (loaded.length === 0) {
      composite = EMPTY_FOREST_COMPOSITE
      notify()
      return
    }
    const fillPositions = new Float32Array(2 * vertexTotal)
    const fillStartIndices = new Uint32Array(ringTotal)
    const outlineSegments = new Float32Array(segFloatTotal)
    let vertexBase = 0
    let ringBase = 0
    let segBase = 0
    for (const chunk of loaded) {
      fillPositions.set(chunk.fillPositions, 2 * vertexBase)
      for (let k = 0; k < chunk.fillStartIndices.length; k++) {
        fillStartIndices[ringBase + k] = chunk.fillStartIndices[k] + vertexBase
      }
      outlineSegments.set(chunk.outlineSegments, segBase)
      vertexBase += chunk.fillPositions.length / 2
      ringBase += chunk.fillStartIndices.length
      segBase += chunk.outlineSegments.length
    }
    composite = {
      fillPositions,
      fillStartIndices,
      outlineSegments,
      polygonCount: ringTotal,
      segmentCount: segFloatTotal / 4,
      chunkCount: loaded.length,
    }
    notify()
  }

  function requestMissing(ids: string[]): void {
    const missing = ids.filter((id) => !cache.has(id) && !inflight.has(id))
    if (missing.length === 0) return
    for (const id of missing) inflight.add(id)
    client
      .loadForestMass(missing)
      .then((result) => {
        for (const chunk of result.chunks) cache.set(chunk.id, chunk)
        for (const id of result.emptyIds) if (!cache.has(id)) cache.set(id, null)
        for (const id of missing) inflight.delete(id)
        if (result.chunks.length > 0) rebuildComposite()
      })
      .catch((e: unknown) => {
        for (const id of missing) inflight.delete(id)
        console.warn('[worldmap] forest-mass hydrate failed — will retry on next viewport change', e)
      })
  }

  function runViewport(bbox: Bbox): void {
    if (!terrain || !manifest) return
    // Full-grid ids (density files exist independently of the instance-chunk index; misses
    // come back as emptyIds and cache as null). No oversized ring — density is per-chunk.
    const ids = chunkIdsForViewport(bbox, terrain, { chunkSizeM: manifest.chunkSizeM })
    const key = ids.join(',')
    if (key === lastKey) return
    lastKey = key
    requestMissing(ids)
  }

  return {
    ensureForestStream(t: TerrainDef): void {
      if (terrain?.id === t.id && started) return
      if (terrain && terrain.id !== t.id) {
        // Terrain switch: drop local state only — the shared worker core is unloaded by
        // chunkStore's switch path (both stores talk to the same worker session).
        cache.clear()
        inflight.clear()
        lastKey = ''
        lastViewport = null
        composite = EMPTY_FOREST_COMPOSITE
        manifest = null
      }
      terrain = t
      started = true
      client
        .loadManifest(t.id)
        .then((m) => {
          if (terrain?.id !== t.id) return // switched away while loading
          // No export or no density grids → cleanly absent (plan R11 empty state).
          manifest = m?.densityPath ? m : null
          if (manifest && lastViewport) runViewport(lastViewport)
        })
        .catch((e: unknown) => {
          if (terrain?.id !== t.id) return
          console.warn(`[worldmap] forest-mass manifest load failed for ${t.id} — forest off`, e)
        })
    },

    setForestViewport(bbox: Bbox | null): void {
      if (!bbox) return
      lastViewport = bbox
      runViewport(bbox)
    },

    getForestMass(): ForestMassComposite {
      return composite
    },

    getForestRevision(): number {
      return revision
    },

    subscribeForestStream(cb: () => void): () => void {
      listeners.add(cb)
      return () => listeners.delete(cb)
    },

    resetForestStream(): void {
      terrain = null
      manifest = null
      started = false
      cache.clear()
      inflight.clear()
      lastKey = ''
      lastViewport = null
      composite = EMPTY_FOREST_COMPOSITE
      notify()
    },
  }
}

const defaultStore = createForestMassStore({
  client: {
    loadManifest: loadWorldManifest,
    loadForestMass: (ids) => loadWorldForestMass(ids),
  },
})

export const {
  ensureForestStream,
  setForestViewport,
  getForestMass,
  getForestRevision,
  subscribeForestStream,
  resetForestStream,
} = defaultStore
