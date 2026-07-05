// T-090.5.3 — Main-thread chunk store: the streaming half of Map Engine v2 (plan §6, A3
// `landSave` analogue). Subscribes the render hook to a viewport-driven cache of hydrated
// world-object chunks:
//
//   setWorldViewport(bbox, zoom)
//     → chunk ids via chunkMath (border preload + oversized ring), ∩ export cell set
//     → skip entirely when no instance class is visible at this zoom (gates closed)
//     → diff against cache/in-flight → worker fetch (typed-array payloads, transferables)
//     → apply queue: ≤ APPLY_BUDGET_MS (4 ms) per frame converting building groups to
//       BuildingInstance rows (obbCorners — same geometry as the T-090.5.2 loader)
//     → refcount-pinned visible set, LRU eviction max(64, 3× viewport chunks) beyond it
//
// The composite getWorldBuildings() array only changes when a drain/evict/pin-set change
// bumps the revision, so the layer memo stays pan-stable (T-057 rule: nothing here runs per
// frame — the hook's effect calls setWorldViewport, which early-exits on an unchanged set).
//
// Factory + module default instance: tests build their own store with a fake client/clock
// (no test hooks in the prod path); the app uses the singleton wired to worldObjectsClient.

import { classVisible, type WorldRenderClass } from './lodGates'
import { chunkIdsForViewport, type Bbox } from './chunkMath'
import {
  badgeIconKey,
  buildingPrefabLookup,
  obbCorners,
  type BuildingInstance,
  type BuildingPrefabInfo,
} from './buildingLayer'
import {
  loadWorldChunksInBbox,
  loadWorldManifest,
  unloadWorldObjects,
} from '../workers/worldObjectsClient'
import type {
  ChunkLoadResult,
  ChunkPayload,
  InstanceRenderClass,
  LoadChunksOpts,
  WorldManifestLite,
} from '../workers/worldObjectsCore'
import type { TerrainDef } from '../coords/terrains'

/** Main-thread apply budget per frame (plan §6 hydrate budget — A3 TCLoadMapObjects). */
export const APPLY_BUDGET_MS = 4
/** LRU floor: never shrink the cache below this many chunks (plan §6 cache policy). */
export const LRU_MIN_CHUNKS = 64
/** Instance classes this slice hydrates + renders. T-090.5.5 adds 'tree' (glyph band);
 *  piers/docks ride the 'building' class (T-090.5.2.2 taxonomy). */
export const HYDRATE_RENDER_CLASSES: readonly InstanceRenderClass[] = ['building']

export interface WorldStreamStats {
  chunksApplied: number
  applyFrames: number
  maxApplyMs: number
  framesOverBudget: number
}

export type WorldStreamStatus = 'idle' | 'loading' | 'ready' | 'absent'

/** The worker-client surface the store consumes (injectable for tests). */
export interface WorldStreamClient {
  loadManifest(terrainId: string): Promise<WorldManifestLite | null>
  loadChunksInBbox(bbox: Bbox, marginCells: number, opts: LoadChunksOpts): Promise<ChunkLoadResult>
  unload(): Promise<void>
}

export interface ChunkStoreDeps {
  client: WorldStreamClient
  /** Monotonic ms clock (performance.now in prod; fake in tests). */
  now: () => number
  /** Schedule the next apply frame (rAF in prod, hidden-tab safe; manual pump in tests). */
  schedule: (cb: () => void) => void
}

export interface WorldChunkStore {
  ensureWorldStream(terrain: TerrainDef): void
  setWorldViewport(bbox: Bbox | null, deckZoom: number): void
  getWorldBuildings(): BuildingInstance[]
  getWorldStreamStatus(): WorldStreamStatus
  getWorldRevision(): number
  subscribeWorldStream(cb: () => void): () => void
  resetWorldStream(): void
  getWorldStreamStats(): WorldStreamStats
}

export function createChunkStore(deps: ChunkStoreDeps): WorldChunkStore {
  const { client, now, schedule } = deps

  let terrain: TerrainDef | null = null
  let manifest: WorldManifestLite | null = null
  let status: WorldStreamStatus = 'idle'
  let manifestPromise: Promise<void> | null = null
  let buildingInfo = new Map<number, BuildingPrefabInfo>()
  let cellIds: Set<string> | null = null

  /** chunkId → applied building rows ([] = hydrated, nothing drawable). */
  const applied = new Map<string, BuildingInstance[]>()
  const lastUsed = new Map<string, number>()
  const inflight = new Set<string>()
  const applyQueue: ChunkPayload[] = []
  const queuedIds = new Set<string>()
  let pinned = new Set<string>()
  let pinnedKey = ''
  let useTick = 0
  let drainScheduled = false

  let composite: BuildingInstance[] = []
  let revision = 0
  const listeners = new Set<() => void>()
  const stats: WorldStreamStats = { chunksApplied: 0, applyFrames: 0, maxApplyMs: 0, framesOverBudget: 0 }

  /** Last requested viewport — replayed once the manifest resolves. */
  let lastViewport: { bbox: Bbox; deckZoom: number } | null = null

  const notify = (): void => {
    revision++
    listeners.forEach((l) => l())
  }

  function rebuildComposite(): void {
    const next: BuildingInstance[] = []
    // Stable id order so the composite is deterministic regardless of fetch completion order.
    const ids = [...pinned].sort()
    for (const id of ids) {
      const rows = applied.get(id)
      if (rows && rows.length) next.push(...rows)
    }
    composite = next
    notify()
  }

  function applyChunk(payload: ChunkPayload): void {
    const group = payload.groups.building
    const rows: BuildingInstance[] = []
    if (group) {
      for (let k = 0; k < group.count; k++) {
        const info = buildingInfo.get(group.prefabIdx[k])
        if (!info) continue
        const x = group.positions[2 * k]
        const y = group.positions[2 * k + 1]
        const rot = group.rotations[k]
        rows.push({
          position: [x, y],
          polygon: obbCorners(x, y, info.halfX, info.halfY, rot),
          buildingClass: info.buildingClass,
          badgeIconKey: badgeIconKey(info.buildingClass),
        })
      }
    }
    applied.set(payload.id, rows)
    lastUsed.set(payload.id, ++useTick)
    stats.chunksApplied++
  }

  function evictBeyondCap(): boolean {
    const cap = Math.max(LRU_MIN_CHUNKS, 3 * pinned.size)
    if (applied.size <= cap) return false
    const evictable = [...applied.keys()]
      .filter((id) => !pinned.has(id))
      .sort((a, b) => (lastUsed.get(a) ?? 0) - (lastUsed.get(b) ?? 0))
    let evicted = false
    for (const id of evictable) {
      if (applied.size <= cap) break
      applied.delete(id)
      lastUsed.delete(id)
      evicted = true
    }
    return evicted
  }

  function drainFrame(): void {
    drainScheduled = false
    const frameStart = now()
    let appliedThisFrame = 0
    while (applyQueue.length > 0 && now() - frameStart < APPLY_BUDGET_MS) {
      const payload = applyQueue.shift() as ChunkPayload
      queuedIds.delete(payload.id)
      applyChunk(payload)
      appliedThisFrame++
    }
    if (appliedThisFrame > 0) {
      // Hydrate instrumentation (plan §6 budget claim): stats feed the vitest budget hook +
      // getWorldStreamStats(); a frame that blew the budget is a real perf signal, so it
      // warns (lint policy allows warn/error only — no debug chatter on the happy path).
      const ms = now() - frameStart
      stats.applyFrames++
      if (ms > stats.maxApplyMs) stats.maxApplyMs = ms
      if (ms > APPLY_BUDGET_MS) {
        stats.framesOverBudget++
        console.warn(
          `[worldmap] hydrate frame over budget: ${ms.toFixed(1)} ms > ${APPLY_BUDGET_MS} ms ` +
            `(${appliedThisFrame} chunks; ${stats.framesOverBudget} over-budget frames total)`,
        )
      }
      evictBeyondCap()
      rebuildComposite()
    }
    if (applyQueue.length > 0) scheduleDrain()
  }

  function scheduleDrain(): void {
    if (drainScheduled) return
    drainScheduled = true
    schedule(drainFrame)
  }

  function requestMissing(bbox: Bbox, deckZoom: number, classes: InstanceRenderClass[], ids: string[]): void {
    const missing = ids.filter((id) => !applied.has(id) && !inflight.has(id) && !queuedIds.has(id))
    if (missing.length === 0) return
    for (const id of missing) inflight.add(id)
    client
      .loadChunksInBbox(bbox, 0, { deckZoom, classes, ids: missing })
      .then((result) => {
        const delivered = new Set<string>()
        for (const payload of result.chunks) {
          delivered.add(payload.id)
          if (!queuedIds.has(payload.id)) {
            queuedIds.add(payload.id)
            applyQueue.push(payload)
          }
        }
        // Requested but undelivered (missing/empty file) → cache as hydrated-empty so the
        // store never re-requests it.
        for (const id of missing) {
          inflight.delete(id)
          if (!delivered.has(id) && !applied.has(id)) {
            applied.set(id, [])
            lastUsed.set(id, ++useTick)
          }
        }
        if (applyQueue.length > 0) scheduleDrain()
      })
      .catch((e: unknown) => {
        for (const id of missing) inflight.delete(id)
        console.warn('[worldmap] chunk hydrate failed — will retry on next viewport change', e)
      })
  }

  function runViewport(bbox: Bbox, deckZoom: number): void {
    if (!terrain || !manifest) return
    const classes = HYDRATE_RENDER_CLASSES.filter((c) => classVisible(c as WorldRenderClass, deckZoom))
    if (classes.length === 0) {
      // Skip-when-invisible (plan §6): all instance gates closed — release pins, keep cache.
      if (pinned.size > 0) {
        pinned = new Set()
        pinnedKey = ''
        rebuildComposite()
      }
      return
    }
    let ids = chunkIdsForViewport(bbox, terrain, {
      chunkSizeM: manifest.chunkSizeM,
      extraRing: manifest.hasOversized ? 1 : 0,
    })
    const cells = cellIds
    if (cells) ids = ids.filter((id) => cells.has(id))
    const key = ids.join(',')
    if (key === pinnedKey) return
    pinned = new Set(ids)
    pinnedKey = key
    for (const id of ids) if (applied.has(id)) lastUsed.set(id, ++useTick)
    requestMissing(bbox, deckZoom, classes, ids)
    evictBeyondCap()
    rebuildComposite()
  }

  return {
    ensureWorldStream(t: TerrainDef): void {
      if (terrain?.id === t.id && status !== 'idle') return
      // Terrain switch: drop everything (worker side too) before loading the new manifest.
      if (terrain && terrain.id !== t.id) {
        void client.unload().catch(() => undefined)
        applied.clear()
        lastUsed.clear()
        inflight.clear()
        applyQueue.length = 0
        queuedIds.clear()
        pinned = new Set()
        pinnedKey = ''
        composite = []
        manifest = null
        buildingInfo = new Map()
        cellIds = null
        lastViewport = null
      }
      terrain = t
      status = 'loading'
      manifestPromise = client
        .loadManifest(t.id)
        .then((m) => {
          if (terrain?.id !== t.id) return // switched away while loading
          manifest = m
          status = m ? 'ready' : 'absent'
          if (m) {
            // Same lookup the T-090.5.2 loader used — buildingLayer owns the pier/dock +
            // default-extent rules, so streamed footprints are byte-identical.
            buildingInfo = buildingPrefabLookup({ prefabs: m.prefabRows })
            cellIds = m.cells ? new Set(m.cells.map((c) => c.id)) : null
            if (lastViewport) runViewport(lastViewport.bbox, lastViewport.deckZoom)
          }
          notify()
        })
        .catch((e: unknown) => {
          if (terrain?.id !== t.id) return
          status = 'absent'
          console.warn(`[worldmap] world-object manifest load failed for ${t.id} — layers off`, e)
          notify()
        })
      void manifestPromise
    },

    setWorldViewport(bbox: Bbox | null, deckZoom: number): void {
      if (!bbox) return
      lastViewport = { bbox, deckZoom }
      runViewport(bbox, deckZoom)
    },

    getWorldBuildings(): BuildingInstance[] {
      return composite
    },

    getWorldStreamStatus(): WorldStreamStatus {
      return status
    },

    getWorldRevision(): number {
      return revision
    },

    subscribeWorldStream(cb: () => void): () => void {
      listeners.add(cb)
      return () => listeners.delete(cb)
    },

    resetWorldStream(): void {
      void client.unload().catch(() => undefined)
      terrain = null
      manifest = null
      status = 'idle'
      manifestPromise = null
      buildingInfo = new Map()
      cellIds = null
      applied.clear()
      lastUsed.clear()
      inflight.clear()
      applyQueue.length = 0
      queuedIds.clear()
      pinned = new Set()
      pinnedKey = ''
      lastViewport = null
      composite = []
      stats.chunksApplied = 0
      stats.applyFrames = 0
      stats.maxApplyMs = 0
      stats.framesOverBudget = 0
      notify()
    },

    getWorldStreamStats(): WorldStreamStats {
      return { ...stats }
    },
  }
}

/** Prod scheduler: rAF when visible (paint-aligned budget frames), macrotask when hidden —
 *  rAF is suspended in background tabs and the queue must still drain (T-062.2 lesson). */
function scheduleFrame(cb: () => void): void {
  if (
    typeof document !== 'undefined' &&
    document.visibilityState !== 'hidden' &&
    typeof requestAnimationFrame === 'function'
  ) {
    requestAnimationFrame(() => cb())
  } else {
    setTimeout(cb, 0)
  }
}

const defaultStore = createChunkStore({
  client: {
    loadManifest: loadWorldManifest,
    loadChunksInBbox: loadWorldChunksInBbox,
    unload: unloadWorldObjects,
  },
  now: () => performance.now(),
  schedule: scheduleFrame,
})

export const {
  ensureWorldStream,
  setWorldViewport,
  getWorldBuildings,
  getWorldStreamStatus,
  getWorldRevision,
  subscribeWorldStream,
  resetWorldStream,
  getWorldStreamStats,
} = defaultStore
