// T-151.3 W3 — wgpu world-object residency controller (imperative; no React). The wgpu path's
// replacement for the Deck `chunkStore` + Comlink worker: a thin JS fetch shim feeds gz bytes to
// the Rust `WorldResidency` (which parses once, holds the multi-chunk LRU, and composes the
// building GPU buffers), then this pushes those buffers to the `RenderEngine` building lanes.
//
// T-151.5 W5: also loads the world glyph atlas once and pushes tree/prop/badge icon lanes from
// the same residency (replace-not-accumulate, INSTANCE_BUDGET, LOD + prefs).
//
// D2 framing (t145 kickoff): JS fetches, Rust parses (gunzip via flate2 inside
// `WorldResidency.ingest_chunk_gz`). No per-frame JS consumer, so no SharedArrayBuffer.
// Since T-151.9 this drives the ONLY Mission Creator map engine — the Deck
// worker/chunkStore/rbush path was deleted at the flip (T-151.11.2 comment refresh, audit A-07).
//
// Flow per camera move (debounced): residency.set_viewport(bounds, zoom) → missing ids → 12-way
// concurrent chunk fetch → budgeted ingest loop (≤ APPLY_BUDGET_MS/frame) → engine building+glyph lanes.

import { WorldResidency, WorldStore, DemGrid, dem_apron_grid_factor, atlas_glyph_count } from '@/wasm/pkg/map_engine_wasm'
import { loadWorldGlyphAtlas } from '../layers/worldGlyphAtlas'
import { getClassToggles } from '../state/worldLayerPrefs'
import { getDemRasterForOverlay } from '../dem/DemController'
import type { TerrainDef } from '../coords/terrains'
import type { RenderEngine } from './wasmRender'

/** Engine role ids (match map-engine-render `lane_role_from_u32`). */
const ROLE_LANDCOVER = 1
const ROLE_ROADS_CASING = 3
const ROLE_ROADS = 4
/** T-152.5 airfield apron polygon (`LaneRole::WorldAirfieldApron`). */
const ROLE_AIRFIELD_APRON = 8

/** Concurrent chunk fetches (mirror `worldObjectsCore` `DEFAULT_FETCH_CONCURRENCY`). */
const FETCH_CONCURRENCY = 12
// T-151.11.3 (audit B-04): the ≤ 4 ms/frame ingest budget is OWNED BY RUST
// (`residency.rs::APPLY_BUDGET_MS` + begin/exhausted/end frame API) — no TS budget constant.
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
  private lastRoadZoomBand = ''
  private lastAirfieldApronKey = ''
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

  /** Push current worldLayerPrefs toggles into residency (trees/props/buildings/fences).
   *  T-151.11.3 (P-04): buildings toggle now empties/rebuilds the footprint buffers in Rust,
   *  so push the WHOLE lane set (fills + outlines + glyphs + fence strips) — not just glyphs —
   *  or a toggle wouldn't take effect until the next camera move. */
  syncGlyphToggles(): void {
    if (!this.residency || this.disposed) return
    const t = getClassToggles()
    this.residency.set_glyph_toggles(t.trees, t.props, t.buildings)
    this.residency.set_fences_toggle(t.fences)
    this.residency.set_airfield_toggle(t.airfield)
    this.lastRoadZoomBand = ''
    this.lastAirfieldApronKey = ''
    // T-152.20 — the Roads + Forest mass toggles gate their lanes in pushRoads/pushLandcover; force
    // both to recompose (null the landcover memo, cleared road band above) so a flip takes effect now.
    this.lastLandcoverVis = null
    if (this.ready) {
      this.pushRoads()
      this.pushAirfieldApron()
      this.pushLandcover()
      this.pushToEngine()
    }
  }

  /** Decode world-glyphs.webp + JSON → GPU atlas + UV table + key map (T-151.5 L1–L3). */
  private async loadGlyphAtlas(): Promise<void> {
    if (!this.residency) return
    try {
      const atlas = await loadWorldGlyphAtlas()
      if (!atlas || this.disposed || !this.residency) return
      const keys = Object.keys(atlas.iconMapping).sort()
      // Capacity check against the engine's UV-table size (Rust single source of truth), NOT a
      // hardcoded literal — the atlas may hold up to `atlas_glyph_count()` keys. Bail only when it
      // genuinely exceeds engine capacity (would drop glyphs); the count guard test enforces ≤.
      const capacity = atlas_glyph_count()
      if (keys.length > capacity) {
        console.warn(`[wgpu-world] glyph atlas key count ${keys.length} > capacity ${capacity} — glyphs off`)
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
      const uv = new Float32Array(keys.length * 4)
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
          if (this.residency) {
            this.residency.set_airfield_bbox_from_store(this.store)
          }
          this.pushAirfieldApron()
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
   * Land-cover LOD refresh (T-151.5.1; T-152.14 handoff): visibility follows the residency's
   * `forest_fill_effective` — the mega-hull hides at zoom ≥ 0 once tree glyphs actually pack, but
   * persists while they are heatmap-swapped or empty. Not sticky — re-evaluated on every settle.
   */
  private pushLandcover(): void {
    if (!this.store || !this.landcoverReady || !this.residency) return
    // T-152.14: land-cover mass visibility is the residency handoff (`forest_fill_effective`), not
    // the pure-zoom `forestFill` gate — the green mass persists at z ≥ 0 while tree glyphs are
    // heatmap-swapped or the lane packs empty, so zooming into dense forest never blanks.
    // T-152.20: AND the user Forest-mass toggle — the landcover hulls are the low-zoom half of the
    // same green forest as useWgpuForestMass (fill/outline), so `forest` off hides both.
    const vis = this.residency.forest_fill_effective && getClassToggles().forest
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
    // T-152.20 — the Roads toggle gates both road lanes (centerline + casing). Fold it into the
    // band key so a flip recomposes, and into the per-lane `visible` flag so off hides them.
    const roadsOn = getClassToggles().roads
    const airfieldPolish = getClassToggles().airfield
    // Band: integer zoom steps of 0.5 so continuous pan doesn't recompose.
    const band = Math.round(zoom * 2)
    const bandKey = `${band}:${airfieldPolish ? 1 : 0}:${roadsOn ? 1 : 0}`
    if (bandKey === this.lastRoadZoomBand) return
    this.lastRoadZoomBand = bandKey
    try {
      const roads = this.store.compose_roads(zoom, airfieldPolish)
      const roadsVisible = roadsOn && roads.segment_count > 0
      this.engine.upload_strip_tris(ROLE_ROADS_CASING, roads.casing, roads.segment_count, roadsVisible)
      this.engine.upload_strip_tris(ROLE_ROADS, roads.centerline, roads.segment_count, roadsVisible)
      roads.free()
    } catch (err) {
      console.warn('[wgpu-world] roads compose failed', err)
    }
  }

  /** T-152.5 DEM-flat apron polygon at NW Everon airfield. */
  private pushAirfieldApron(): void {
    if (!this.store || !this.roadsLoaded) return
    const enabled = getClassToggles().airfield
    const bbox = this.store.airfield_bbox()
    const key = `${enabled ? 1 : 0}:${bbox.join(',')}`
    if (key === this.lastAirfieldApronKey) return
    this.lastAirfieldApronKey = key
    if (!enabled || bbox.length !== 4) {
      this.engine.clear_vector_lane(ROLE_AIRFIELD_APRON)
      return
    }
    const raster = getDemRasterForOverlay()
    if (!raster?.metersCache) {
      this.engine.clear_vector_lane(ROLE_AIRFIELD_APRON)
      return
    }
    let grid: DemGrid | null = null
    try {
      const worldW = raster.width > 0 ? raster.width * 2 : 12_800
      const worldH = raster.height > 0 ? raster.height * 2 : 12_800
      grid = DemGrid.downsample(
        raster.metersCache as Float32Array,
        raster.width,
        raster.height,
        dem_apron_grid_factor(),
        worldW,
        worldH,
      )
      const mesh = grid.compose_airfield_apron(bbox)
      this.engine.upload_polygon_mesh(
        ROLE_AIRFIELD_APRON,
        mesh.positions,
        mesh.colors,
        mesh.indices,
        mesh.polygon_count,
        true,
      )
      mesh.free()
    } catch (err) {
      console.warn('[wgpu-world] airfield apron compose failed', err)
      this.engine.clear_vector_lane(ROLE_AIRFIELD_APRON)
      this.lastAirfieldApronKey = ''
    } finally {
      grid?.free()
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
    // W4 roads + airfield apron recompose on LOD/toggle change (debounced with camera).
    this.pushRoads()
    this.pushAirfieldApron()
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

  /** Ingest queued chunks under the CORE-owned per-frame budget (T-151.11.3 / B-04): the
   *  loop shape stays here (rAF scheduling is UI domain) but Rust decides when the frame is
   *  spent and records the stats. */
  private drain(): void {
    if (this.disposed || !this.residency) return
    this.residency.begin_ingest_frame()
    let applied = 0
    while (this.pending.length > 0 && !this.residency.frame_budget_exhausted()) {
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
      this.residency.end_ingest_frame()
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
    // T-151.11.3 (P-04): visibility = residency policy (user toggle ∧ zoom gate). Toggle-off
    // arrives as empty buffers + visible=false, which REMOVES the lanes (the empty+visible
    // sticky rule only guards mid-hydration wipes).
    const bVis = this.residency.buildings_visible()
    this.engine.upload_world_buildings(fill, rstats.chunks_pinned, bVis)
    this.engine.upload_world_building_outlines(outline, bVis)
    const strips = this.residency.world_fence_strips()
    // T-152.15 — the shared strip lane carries fences + piers + bridge rails; gate the upload on
    // strips_visible() (fences OR piers OR buildings) so piers don't vanish when Fences is toggled
    // off. Per-class gating already happened in Rust when the buffer was composed.
    const stripsVis = this.residency.strips_visible()
    this.engine.upload_world_fence_strips(strips, strips.length > 0 ? 1 : 0, stripsVis)
    this.pushGlyphsToEngine()
    const engStats = JSON.parse(this.engine.stats()) as {
      world_building_instances: number
      tree_glyphs?: number
      prop_glyphs?: number
      badge_glyphs?: number
    }
    this.publishDebug(rstats, engStats.world_building_instances, engStats)
  }

  /** Sticky icon upload: skip empty mid-hydration so prior GPU lane survives. */
  private pushIconLane(kind: number, bytes: Uint8Array, midHydration: boolean): void {
    if (bytes.length > 0 || !midHydration) {
      this.engine.upload_icon_lane(kind, bytes, bytes.length > 0)
    }
  }

  /** T-151.8 density ladder — thin wasm call only (no cull/ladder math in TS). */
  private pushDensityHeat(): void {
    if (!this.residency) return
    const heat = this.residency.heatmap_trees
    const size = this.residency.density_grid_size()
    const w = size[0] ?? 0
    const h = size[1] ?? 0
    const grid = heat ? this.residency.density_grid_r32_bytes() : new Uint8Array(0)
    this.engine.upload_density_grid(grid, w, h, this.terrain.width, this.terrain.height, heat)
  }

  /** Push tree / prop / badge icon lanes + density heatmap (sticky empty mid-hydration). */
  private pushGlyphsToEngine(): void {
    if (this.disposed || !this.residency || !this.atlasReady) return
    const rstats = JSON.parse(this.residency.stats()) as {
      inflight_count: number
      pin_settled: boolean
    }
    const midHydration =
      rstats.inflight_count > 0 || this.pending.length > 0 || !rstats.pin_settled
    this.pushIconLane(0, this.residency.world_tree_glyphs(), midHydration)
    this.pushIconLane(1, this.residency.world_prop_glyphs(), midHydration)
    this.pushIconLane(2, this.residency.world_badge_glyphs(), midHydration)
    if (!midHydration) this.pushDensityHeat()
  }

  private glyphStat(
    eng: number | undefined,
    fromResidency: number | undefined,
  ): number {
    return eng ?? fromResidency ?? 0
  }

  /** Dev surface for S1 verify: `window.__wgpuWorldStats`. */
  private publishDebug(
    rstats: Record<string, unknown>,
    engineInstances: number,
    engStats?: { tree_glyphs?: number; prop_glyphs?: number; badge_glyphs?: number },
  ): void {
    if (typeof window === 'undefined') return
    const r = this.residency
    ;(window as unknown as { __wgpuWorldStats?: unknown }).__wgpuWorldStats = Object.assign(
      {},
      rstats,
      {
        world_building_instances: engineInstances,
        tree_glyphs: this.glyphStat(engStats?.tree_glyphs, r?.tree_glyph_count),
        prop_glyphs: this.glyphStat(engStats?.prop_glyphs, r?.prop_glyph_count),
        badge_glyphs: this.glyphStat(engStats?.badge_glyphs, r?.badge_glyph_count),
        chunks_draw: rstats.chunks_draw ?? r?.chunks_draw ?? 0,
        heatmap_trees: rstats.heatmap_trees ?? r?.heatmap_trees ?? false,
        atlas_ready: this.atlasReady,
        pending: this.pending.length,
      },
    )
  }

  private chunkUrl(id: string): string {
    return `${this.assetBase}/objects/chunks/${id}.json.gz`
  }
}
