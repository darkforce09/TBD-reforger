// T-151.3 W3 — wgpu world-object residency controller (imperative; no React). The wgpu path's
// replacement for the Deck `chunkStore` + Comlink worker: a thin JS fetch shim feeds gz bytes to
// the Rust `WorldResidency` (which parses once, holds the multi-chunk LRU, and composes the
// building GPU buffers), then this pushes those buffers to the `RenderEngine` building lanes.
//
// D2 framing (t145 kickoff): JS fetches, Rust parses. No per-frame JS consumer, so no
// SharedArrayBuffer. Deck's worker/chunkStore/rbush path is untouched — this only drives the
// `?engine=wgpu` mount.
//
// Flow per camera move (debounced): residency.set_viewport(bounds, zoom) → missing ids → 12-way
// concurrent chunk fetch → budgeted ingest loop (≤ APPLY_BUDGET_MS/frame) → engine building lanes.

import { WorldResidency } from '@/wasm/pkg/map_engine_wasm'
import type { TerrainDef } from '../coords/terrains'
import type { RenderEngine } from './wasmRender'

/** Concurrent chunk fetches (mirror `worldObjectsCore` `DEFAULT_FETCH_CONCURRENCY`). */
const FETCH_CONCURRENCY = 12
/** Per-frame ingest budget, ms (mirror `chunkStore` `APPLY_BUDGET_MS`; enforced JS-side). */
const APPLY_BUDGET_MS = 4
/** Debounce camera-move → residency recompute (matches the basemap LOD debounce). */
const MOVE_DEBOUNCE_MS = 120

/** Fetch a static asset to bytes. Vite dev SPA-fallbacks unknown paths to index.html with 200, so
 *  an HTML content-type reads as missing (the same rule the Deck loader uses). */
async function httpFetchBytes(url: string, signal: AbortSignal): Promise<Uint8Array | null> {
  const res = await fetch(url, { signal })
  const type = res.headers.get('content-type') ?? ''
  if (!res.ok || type.includes('text/html')) return null
  return new Uint8Array(await res.arrayBuffer())
}

interface ObjectsBlock {
  prefabsPath?: string
  chunksPath?: string
}

/** The two object-export paths, present ⇒ a v2 world export exists. */
interface ObjectExport {
  prefabsPath: string
  chunksPath: string
}

interface PendingChunk {
  id: string
  bytes: Uint8Array | null
}

/**
 * Drives the Rust `WorldResidency` for the wgpu mount. Created once in `WgpuTacticalMap`'s mount
 * effect (effect-local, like the engine); `dispose()` frees the wasm handle exactly once.
 */
export class WgpuWorldController {
  private readonly engine: RenderEngine
  private readonly terrain: TerrainDef
  private residency: WorldResidency | null = null

  private ready = false
  private disposed = false
  private assetBase = ''

  private fetchAc: AbortController | null = null
  private moveTimer: ReturnType<typeof setTimeout> | null = null
  private readonly pending: PendingChunk[] = []
  private drainScheduled = false

  constructor(engine: RenderEngine, terrain: TerrainDef) {
    this.engine = engine
    this.terrain = terrain
    this.residency = new WorldResidency()
  }

  /** Load the manifest + prefab table + chunk index, then run the first viewport pass. Idempotent —
   *  a second call after `ready` is a no-op (the component remounts on terrain switch). */
  async init(): Promise<void> {
    if (this.disposed || this.ready || !this.residency) return
    const manifestUrl = this.terrain.manifestUrl
    if (!manifestUrl) return
    const ac = new AbortController()
    this.fetchAc = ac
    const exp = await this.loadManifest(manifestUrl, ac.signal)
    if (!exp || this.disposed || !this.residency) return
    this.assetBase = manifestUrl.slice(0, manifestUrl.lastIndexOf('/'))
    await this.loadPrefabsAndIndex(exp, ac.signal)
    if (this.disposed || !this.residency) return
    this.ready = true
    this.runViewport()
  }

  /** Fetch + load the terrain manifest; returns the object-export paths iff a v2 export exists. */
  private async loadManifest(url: string, signal: AbortSignal): Promise<ObjectExport | null> {
    let text: string
    try {
      const res = await fetch(url, { signal })
      if (!res.ok) return null
      text = await res.text()
    } catch {
      return null
    }
    if (this.disposed || !this.residency) return null
    try {
      this.residency.load_manifest_json(text) // objects block + worldBounds
      const objects = (JSON.parse(text) as { objects?: ObjectsBlock }).objects
      if (!objects?.prefabsPath || !objects.chunksPath) return null
      return { prefabsPath: objects.prefabsPath, chunksPath: objects.chunksPath }
    } catch (err) {
      console.warn('[wgpu-world] manifest parse failed — world lane off', err)
      return null
    }
  }

  /** Load `prefabs.json.gz` + the chunk index into the residency (best-effort). */
  private async loadPrefabsAndIndex(exp: ObjectExport, signal: AbortSignal): Promise<void> {
    const prefabBytes = await httpFetchBytes(`${this.assetBase}/${exp.prefabsPath}`, signal)
    if (this.disposed || !this.residency) return
    if (prefabBytes) {
      try {
        this.residency.load_prefabs_gz(prefabBytes)
      } catch (err) {
        console.warn('[wgpu-world] prefabs load failed', err)
      }
    }
    try {
      const idxRes = await fetch(`${this.assetBase}/${exp.chunksPath}/manifest.json`, { signal })
      if (idxRes.ok && this.residency) this.residency.load_chunk_index_json(await idxRes.text())
    } catch {
      /* no chunk index → set_viewport falls back to unbounded (still class-gated) */
    }
  }

  /** Camera moved — recompute residency (debounced; the residency early-exits on an unchanged
   *  chunk set, so an over-eager call is cheap). */
  onCameraMoved(): void {
    if (this.disposed || !this.ready) return
    if (this.moveTimer) clearTimeout(this.moveTimer)
    this.moveTimer = setTimeout(() => this.runViewport(), MOVE_DEBOUNCE_MS)
  }

  dispose(): void {
    this.disposed = true
    this.fetchAc?.abort()
    if (this.moveTimer) clearTimeout(this.moveTimer)
    this.residency?.free() // wasm handle — free exactly once (wasm-react-lifecycle)
    this.residency = null
  }

  private runViewport(): void {
    if (this.disposed || !this.ready || !this.residency) return
    const b = this.engine.visible_bounds()
    const missing = this.residency.set_viewport(b[0], b[1], b[2], b[3], this.engine.zoom)
    if (missing.length > 0) {
      void this.fetchAndQueue(missing)
    } else {
      // Unchanged set / gate closed / all-cached: keep the engine buffers in sync (cheap — the
      // composite is small and only re-pushed on a debounced camera-move end).
      this.pushToEngine()
    }
  }

  /** 12-way concurrent fetch of the missing chunk bytes, then a budgeted ingest drain. */
  private async fetchAndQueue(ids: string[]): Promise<void> {
    this.fetchAc?.abort()
    const ac = new AbortController()
    this.fetchAc = ac
    const fetched: PendingChunk[] = new Array<PendingChunk>(ids.length)
    let cursor = 0
    const worker = async (): Promise<void> => {
      for (;;) {
        const i = cursor++
        if (i >= ids.length) break
        const id = ids[i]
        try {
          fetched[i] = { id, bytes: await httpFetchBytes(this.chunkUrl(id), ac.signal) }
        } catch {
          if (ac.signal.aborted) return
          fetched[i] = { id, bytes: null }
        }
      }
    }
    try {
      await Promise.all(
        Array.from({ length: Math.min(FETCH_CONCURRENCY, ids.length) }, () => worker()),
      )
    } catch {
      return
    }
    if (ac.signal.aborted || this.disposed) return
    this.pending.push(...fetched.filter(Boolean))
    this.drain()
  }

  /** Ingest queued chunks under the per-frame budget, deferring the rest to the next frame. */
  private drain(): void {
    if (this.disposed || !this.residency) return
    const start = performance.now()
    let applied = 0
    while (this.pending.length > 0 && performance.now() - start < APPLY_BUDGET_MS) {
      const next = this.pending.shift()
      if (!next) break
      if (next.bytes) {
        try {
          this.residency.ingest_chunk_gz(next.id, next.bytes)
        } catch {
          this.residency.note_undelivered(next.id)
        }
      } else {
        this.residency.note_undelivered(next.id) // missing/empty file → hydrated-empty
      }
      applied++
    }
    if (applied > 0) {
      this.residency.end_apply_frame(performance.now() - start)
      this.pushToEngine()
    }
    if (this.pending.length > 0) this.scheduleDrain()
  }

  private scheduleDrain(): void {
    if (this.drainScheduled) return
    this.drainScheduled = true
    const run = (): void => {
      this.drainScheduled = false
      this.drain()
    }
    // rAF when visible (paint-aligned); macrotask when hidden (rAF is suspended in background tabs).
    if (
      typeof document !== 'undefined' &&
      document.visibilityState !== 'hidden' &&
      typeof requestAnimationFrame === 'function'
    ) {
      requestAnimationFrame(() => run())
    } else {
      setTimeout(run, 0)
    }
  }

  /** Push the residency's composed building fill + outline buffers to the engine lanes. */
  private pushToEngine(): void {
    if (this.disposed || !this.residency) return
    const fill = this.residency.world_building_fill()
    const outline = this.residency.world_building_outline()
    const pinned = (JSON.parse(this.residency.stats()) as { chunks_pinned: number }).chunks_pinned
    this.engine.upload_world_buildings(fill, pinned, true)
    this.engine.upload_world_building_outlines(outline, true)
  }

  private chunkUrl(id: string): string {
    return `${this.assetBase}/objects/chunks/${id}.json.gz`
  }
}
