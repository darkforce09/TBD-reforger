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

import { WorldResidency, WorldStore } from '@/wasm/pkg/map_engine_wasm'
import { classVisible } from '../worldmap/lodGates'
import type { TerrainDef } from '../coords/terrains'
import type { RenderEngine } from './wasmRender'

/** Engine role ids (match map-engine-render `lane_role_from_u32`). */
const ROLE_LANDCOVER = 1
const ROLE_ROADS_CASING = 3
const ROLE_ROADS = 4

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
  roadsPath?: string
  regionsPath?: string
}

/** The two object-export paths, present ⇒ a v2 world export exists. */
interface ObjectExport {
  prefabsPath: string
  chunksPath: string
  roadsPath?: string
  regionsPath?: string
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
  /** W4: roads + land-cover (whole-terrain one-shots; parse in wasm). */
  private store: WorldStore | null = null

  private ready = false
  private disposed = false
  private assetBase = ''
  private roadsLoaded = false
  private landcoverPushed = false
  private lastRoadZoomBand = Number.NaN

  private fetchAc: AbortController | null = null
  private moveTimer: ReturnType<typeof setTimeout> | null = null
  private readonly pending: PendingChunk[] = []
  private drainScheduled = false

  constructor(engine: RenderEngine, terrain: TerrainDef) {
    this.engine = engine
    this.terrain = terrain
    this.residency = new WorldResidency()
    this.store = new WorldStore()
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
    await this.loadRoadsAndLandcover(exp, ac.signal)
    if (this.disposed) return
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
      this.store?.load_manifest_json(text)
      const objects = (JSON.parse(text) as { objects?: ObjectsBlock }).objects
      if (!objects?.prefabsPath || !objects.chunksPath) return null
      return {
        prefabsPath: objects.prefabsPath,
        chunksPath: objects.chunksPath,
        roadsPath: objects.roadsPath ?? 'objects/roads.json.gz',
        regionsPath: objects.regionsPath ?? 'objects/forest-regions.json.gz',
      }
    } catch (err) {
      console.warn('[wgpu-world] manifest parse failed — world lane off', err)
      return null
    }
  }

  /** W4: one-shot roads + land-cover load into wasm WorldStore, push landcover immediately. */
  private async loadRoadsAndLandcover(exp: ObjectExport, signal: AbortSignal): Promise<void> {
    if (!this.store) return
    if (exp.roadsPath) {
      const bytes = await httpFetchBytes(`${this.assetBase}/${exp.roadsPath}`, signal)
      if (this.disposed || !this.store) return
      if (bytes) {
        try {
          this.store.load_roads_gz(bytes)
          this.roadsLoaded = true
        } catch (err) {
          console.warn('[wgpu-world] roads load failed', err)
        }
      }
    }
    if (exp.regionsPath) {
      const bytes = await httpFetchBytes(`${this.assetBase}/${exp.regionsPath}`, signal)
      if (this.disposed || !this.store) return
      if (bytes) {
        try {
          this.store.load_forest_regions_gz(bytes)
          this.pushLandcover()
        } catch (err) {
          console.warn('[wgpu-world] landcover load failed', err)
        }
      }
    }
  }

  private pushLandcover(): void {
    if (!this.store || this.landcoverPushed) return
    // Land-cover shares forestFill gate at default zoom (visible as context under mass).
    const zoom = this.engine.zoom
    const vis = classVisible('forestFill', zoom) || zoom <= 1
    try {
      const mesh = this.store.compose_landcover()
      this.engine.upload_polygon_mesh(
        ROLE_LANDCOVER,
        mesh.positions,
        mesh.colors,
        mesh.indices,
        mesh.polygon_count,
        vis,
      )
      mesh.free()
      this.landcoverPushed = true
    } catch (err) {
      console.warn('[wgpu-world] landcover compose failed', err)
    }
  }

  private pushRoads(): void {
    if (!this.store || !this.roadsLoaded) return
    const zoom = this.engine.zoom
    // Band: integer zoom steps of 0.5 so continuous pan doesn't recompose.
    const band = Math.round(zoom * 2)
    if (band === this.lastRoadZoomBand) return
    this.lastRoadZoomBand = band
    try {
      const roads = this.store.compose_roads(zoom)
      this.engine.upload_strip_tris(
        ROLE_ROADS_CASING,
        roads.casing,
        roads.segment_count,
        roads.segment_count > 0,
      )
      this.engine.upload_strip_tris(
        ROLE_ROADS,
        roads.centerline,
        roads.segment_count,
        roads.segment_count > 0,
      )
      roads.free()
    } catch (err) {
      console.warn('[wgpu-world] roads compose failed', err)
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
    this.store?.free()
    this.store = null
  }

  private runViewport(): void {
    if (this.disposed || !this.ready || !this.residency) return
    // W4 roads recompose on LOD band change (debounced with camera).
    this.pushRoads()
    const b = this.engine.visible_bounds()
    const missing = this.residency.set_viewport(b[0], b[1], b[2], b[3], this.engine.zoom)
    if (missing.length > 0) {
      void this.fetchAndQueue(missing)
    } else {
      // Unchanged set / gate closed / all-cached — but never wipe GPU mid-hydration (T-151.4.1).
      this.pushToEngine()
    }
  }

  /** 12-way concurrent fetch of the missing chunk bytes, then a budgeted ingest drain. */
  private async fetchAndQueue(ids: string[]): Promise<void> {
    // T-151.4.1: abort previous fetch, release ALL inflight marks, then re-mark only the ids
    // for this fetch. Without clear_inflight, aborted ids stay excluded forever (Bug B).
    // Without mark_inflight, clear would leave the pin unsettled and double-fetch (same key).
    this.fetchAc?.abort()
    this.residency?.clear_inflight()
    this.residency?.mark_inflight(ids)
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
      if (ac.signal.aborted && this.residency) this.residency.clear_inflight()
      return
    }
    if (ac.signal.aborted || this.disposed) {
      this.residency?.clear_inflight()
      return
    }
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

  /**
   * Push the residency's composed building fill + outline buffers to the engine lanes.
   * T-151.4.1: never call upload with empty fill while hydration is in flight — that removes
   * the WorldBuildings lane and is the root cause of "town buildings disappeared after W4".
   */
  private pushToEngine(): void {
    if (this.disposed || !this.residency) return
    const fill = this.residency.world_building_fill()
    const outline = this.residency.world_building_outline()
    const rstats = JSON.parse(this.residency.stats()) as {
      chunks_pinned: number
      building_instances: number
      inflight_count: number
      pin_settled: boolean
      chunks_resident: number
    }
    // Mid-hydration: keep the previous GPU lane (if any). Empty fill here means rebuild ran
    // before chunks arrived — wiping would drop T-151.3-visible buildings.
    if (fill.length === 0 && (rstats.inflight_count > 0 || this.pending.length > 0 || !rstats.pin_settled)) {
      this.publishDebug(rstats, 0)
      return
    }
    this.engine.upload_world_buildings(fill, rstats.chunks_pinned, true)
    this.engine.upload_world_building_outlines(outline, true)
    const engStats = JSON.parse(this.engine.stats()) as { world_building_instances: number }
    this.publishDebug(rstats, engStats.world_building_instances)
  }

  /** Dev surface for S1 verify: `window.__wgpuWorldStats`. */
  private publishDebug(
    rstats: {
      chunks_pinned: number
      building_instances: number
      inflight_count: number
      pin_settled: boolean
      chunks_resident: number
    },
    engineInstances: number,
  ): void {
    if (typeof window === 'undefined') return
    ;(window as unknown as { __wgpuWorldStats?: unknown }).__wgpuWorldStats = {
      ...rstats,
      world_building_instances: engineInstances,
      pending: this.pending.length,
    }
  }

  private chunkUrl(id: string): string {
    return `${this.assetBase}/objects/chunks/${id}.json.gz`
  }
}
