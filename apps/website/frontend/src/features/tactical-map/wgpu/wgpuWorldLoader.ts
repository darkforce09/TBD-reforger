// T-151.3 W3 — wgpu world-object residency controller (imperative; no React). The wgpu path's
// replacement for the Deck `chunkStore` + Comlink worker: a thin JS fetch shim feeds gz bytes to
// the Rust `WorldResidency` (which parses once, holds the multi-chunk LRU, and composes the
// building GPU buffers), then this pushes those buffers to the `RenderEngine` building lanes.
//
// T-151.5 W5: also loads the world glyph atlas once and pushes tree/prop/badge icon lanes from
// the same residency (replace-not-accumulate, INSTANCE_BUDGET, LOD + prefs).
//
// D2 framing (t145 kickoff): JS fetches, Rust parses. No per-frame JS consumer, so no
// SharedArrayBuffer. Deck's worker/chunkStore/rbush path is untouched — this only drives the
// `?engine=wgpu` mount.
//
// Flow per camera move (debounced): residency.set_viewport(bounds, zoom) → missing ids → 12-way
// concurrent chunk fetch → budgeted ingest loop (≤ APPLY_BUDGET_MS/frame) → engine building+glyph lanes.

import { WorldResidency, WorldStore, class_visible } from '@/wasm/pkg/map_engine_wasm'
import { loadWorldGlyphAtlas } from '../layers/worldGlyphAtlas'
import { getClassToggles } from '../state/worldLayerPrefs'
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
  /** Regions loaded into WorldStore — re-push visibility on every camera settle (T-151.5.1). */
  private landcoverReady = false
  /** Last landcover visibility decision; skip recompose when unchanged. */
  private lastLandcoverVis: boolean | null = null
  private lastRoadZoomBand = Number.NaN
  private atlasReady = false

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
    await this.loadGlyphAtlas()
    if (this.disposed || !this.residency) return
    await this.loadRoadsAndLandcover(exp, ac.signal)
    if (this.disposed) return
    this.syncGlyphToggles()
    this.ready = true
    this.runViewport()
  }

  /** Push current worldLayerPrefs toggles into residency (trees/props/buildings). */
  syncGlyphToggles(): void {
    if (!this.residency || this.disposed) return
    const t = getClassToggles()
    this.residency.set_glyph_toggles(t.trees, t.props, t.buildings)
    if (this.ready) this.pushGlyphsToEngine()
  }

  /** Decode world-glyphs.webp + JSON → GPU atlas + UV table + key map (T-151.5 L1–L3). */
  private async loadGlyphAtlas(): Promise<void> {
    if (!this.residency) return
    try {
      const atlas = await loadWorldGlyphAtlas()
      if (!atlas || this.disposed || !this.residency) return
      const keys = Object.keys(atlas.iconMapping).sort()
      if (keys.length !== 28) {
        console.warn(`[wgpu-world] glyph atlas key count ${keys.length} ≠ 28 — glyphs off`)
        return
      }
      const meta = await fetch(atlas.atlasUrl)
      if (!meta.ok || this.disposed) return
      const blob = await meta.blob()
      const bmp = await createImageBitmap(blob)
      const w = bmp.width
      const h = bmp.height
      const canvas = new OffscreenCanvas(w, h)
      const ctx = canvas.getContext('2d')
      if (!ctx) {
        bmp.close()
        return
      }
      ctx.drawImage(bmp, 0, 0)
      bmp.close()
      const imageData = ctx.getImageData(0, 0, w, h)
      const rgba = new Uint8Array(imageData.data.buffer)
      const uv = new Float32Array(28 * 4)
      for (let i = 0; i < keys.length; i++) {
        const r = atlas.iconMapping[keys[i]]
        uv[i * 4 + 0] = r.x / w
        uv[i * 4 + 1] = r.y / h
        uv[i * 4 + 2] = (r.x + r.width) / w
        uv[i * 4 + 3] = (r.y + r.height) / h
      }
      this.engine.upload_glyph_atlas(rgba, w, h, uv)
      this.residency.set_glyph_key_map(keys)
      this.atlasReady = true
    } catch (err) {
      console.warn('[wgpu-world] glyph atlas load failed — tree/prop glyphs off', err)
    }
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
          this.landcoverReady = true
          this.lastLandcoverVis = null
          this.pushLandcover()
        } catch (err) {
          console.warn('[wgpu-world] landcover load failed', err)
        }
      }
    }
  }

  /**
   * Land-cover LOD refresh (T-151.5.1): same exclusive glyph-band hide as forestFill.
   * Not sticky — re-evaluate on every camera settle so zoom ≥ 0 clears the mega-hull.
   */
  private pushLandcover(): void {
    if (!this.store || !this.landcoverReady) return
    const zoom = this.engine.zoom
    const vis = class_visible('forestFill', zoom)
    if (vis === this.lastLandcoverVis) return
    this.lastLandcoverVis = vis
    if (!vis) {
      this.engine.clear_vector_lane(ROLE_LANDCOVER)
      return
    }
    try {
      const mesh = this.store.compose_landcover()
      this.engine.upload_polygon_mesh(
        ROLE_LANDCOVER,
        mesh.positions,
        mesh.colors,
        mesh.indices,
        mesh.polygon_count,
        true,
      )
      mesh.free()
    } catch (err) {
      console.warn('[wgpu-world] landcover compose failed', err)
      this.lastLandcoverVis = null
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
    // T-151.5.1: landcover visibility tracks forestFill gate on zoom (not one-shot forever).
    this.pushLandcover()
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
    this.pushGlyphsToEngine()
    const engStats = JSON.parse(this.engine.stats()) as {
      world_building_instances: number
      tree_glyphs?: number
      prop_glyphs?: number
      badge_glyphs?: number
    }
    this.publishDebug(rstats, engStats.world_building_instances, engStats)
  }

  /** Push tree / prop / badge icon lanes (sticky empty mid-hydration). */
  private pushGlyphsToEngine(): void {
    if (this.disposed || !this.residency || !this.atlasReady) return
    const rstats = JSON.parse(this.residency.stats()) as {
      inflight_count: number
      pin_settled: boolean
    }
    const midHydration =
      rstats.inflight_count > 0 || this.pending.length > 0 || !rstats.pin_settled
    const trees = this.residency.world_tree_glyphs()
    const props = this.residency.world_prop_glyphs()
    const badges = this.residency.world_badge_glyphs()
    // kind: 0 trees, 1 props, 2 badges — sticky when empty mid-hydration.
    if (trees.length > 0 || !midHydration) {
      this.engine.upload_icon_lane(0, trees, trees.length > 0)
    }
    if (props.length > 0 || !midHydration) {
      this.engine.upload_icon_lane(1, props, props.length > 0)
    }
    if (badges.length > 0 || !midHydration) {
      this.engine.upload_icon_lane(2, badges, badges.length > 0)
    }
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
    engStats?: { tree_glyphs?: number; prop_glyphs?: number; badge_glyphs?: number },
  ): void {
    if (typeof window === 'undefined') return
    ;(window as unknown as { __wgpuWorldStats?: unknown }).__wgpuWorldStats = {
      ...rstats,
      world_building_instances: engineInstances,
      tree_glyphs: engStats?.tree_glyphs ?? this.residency?.tree_glyph_count ?? 0,
      prop_glyphs: engStats?.prop_glyphs ?? this.residency?.prop_glyph_count ?? 0,
      badge_glyphs: engStats?.badge_glyphs ?? this.residency?.badge_glyph_count ?? 0,
      atlas_ready: this.atlasReady,
      pending: this.pending.length,
    }
  }

  private chunkUrl(id: string): string {
    return `${this.assetBase}/objects/chunks/${id}.json.gz`
  }
}
