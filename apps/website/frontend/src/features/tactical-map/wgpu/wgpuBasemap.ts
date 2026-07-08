// T-151.1 — wgpu basemap controller (imperative; no React). Drives the `RenderEngine` texture/line
// lanes to visual parity with the Deck basemap stack, reusing the PROVEN JS resolve/decode:
//   • satellite unified (TBDS) — parseTbdSat + pickBaseLevel + per-mip/block decode → engine mip
//     upload (copy_external_image_to_texture on WebGPU; RGBA write_texture on WebGL2), 153 MB fetch
//     progress 0→0.8, decode 0.8→1; failure forces a pyramid re-resolve (L11).
//   • pyramid (map style / satellite fallback) — computeLod → visible tiles packed into ONE atlas
//     texture (tileUrl is the sole Y inversion; pack offsets mirror lanes::pack_offset), re-run on
//     camera move.
//   • single-bitmap / none — full-extent ortho / grid-only + onDegraded.
//   • hillshade — reuse DemController's decoded metersCache → wasm.hillshade → RGBA quad (role 1).
//   • grid — engine.set_grid (Rust lanes::grid_lines, the useBaseMapLayer.ts mirror).
// The engine never parses TBDS bytes (L2). All math lives in the tested pure helpers.

import { parseTbdSat, pickBaseLevel } from '../layers/satelliteUnified'
import {
  computeLod,
  resolveBasemapMode,
  type Resolved,
} from '../layers/basemapResolve'
import { tileUrl } from '../layers/tileUrl'
import { PAPER_TINT } from '../worldmap/styleModes'
import type { TerrainDef } from '../coords/terrains'
import type { BasemapView } from '../state/basemapView'
import type { MapViewState } from '../types'
import {
  getDemRasterForOverlay,
  isDemReady,
  loadDemForTerrain,
} from '../dem/DemController'
import { hillshade } from '@/wasm/pkg/map_engine_wasm'
import type { RenderEngine } from './wasmRender'

/** Engine `mode` tags (mirror `BasemapMode::from_u32` in engine.rs). */
const MODE = { unified: 0, pyramid: 1, single: 2, hillshade: 3 } as const
/** Basemap texture role in the engine. */
const ROLE_BASEMAP = 0
const ROLE_HILLSHADE = 1
const TILE_PX = 256
/** Debounce for pyramid LOD recompute on camera move. */
const LOD_DEBOUNCE_MS = 120

interface Callbacks {
  onProgress?: (fraction: number | null) => void
  onDegraded?: (view: BasemapView) => void
}

interface PyramidRange {
  z: number
  txMin: number
  txMax: number
  tyMin: number
  tyMax: number
}

/** Draw an ImageBitmap into an RGBA byte array (WebGL2 upload path — no copyExternalImageToTexture).
 *  `colorSpaceConversion: 'none'` on the source keeps the bytes decode-exact. */
function bitmapToRgba(bmp: ImageBitmap): Uint8Array {
  const canvas = new OffscreenCanvas(bmp.width, bmp.height)
  const ctx = canvas.getContext('2d')
  if (!ctx) throw new Error('wgpu-basemap: no 2d context for RGBA fallback')
  ctx.drawImage(bmp, 0, 0)
  const { data } = ctx.getImageData(0, 0, bmp.width, bmp.height)
  return new Uint8Array(data.buffer, data.byteOffset, data.byteLength)
}

export class WgpuBasemapController {
  private readonly engine: RenderEngine
  private terrain: TerrainDef
  private readonly cb: Callbacks
  private readonly webgl2: boolean

  /** Active satellite/map resolve + last pyramid range (to skip no-op LOD reloads). */
  private resolved: Resolved | null = null
  private satOpacity = 1
  private lastRange: PyramidRange | null = null

  private loadAc: AbortController | null = null
  private lodTimer: ReturnType<typeof setTimeout> | null = null
  private disposed = false

  constructor(engine: RenderEngine, terrain: TerrainDef, cb: Callbacks = {}) {
    this.engine = engine
    this.terrain = terrain
    this.cb = cb
    this.webgl2 = engine.backend() === 'webgl2'
  }

  dispose(): void {
    this.disposed = true
    this.loadAc?.abort()
    if (this.lodTimer) clearTimeout(this.lodTimer)
    this.cb.onProgress?.(null)
  }

  // ── basemap (satellite unified / pyramid / single) ─────────────────────────────────────────
  /** Resolve + load the basemap for `view` (satellite | map) at `satOpacity`. Aborts any prior load. */
  async setBasemapView(view: BasemapView, satOpacity: number): Promise<void> {
    this.satOpacity = satOpacity
    this.loadAc?.abort()
    const ac = new AbortController()
    this.loadAc = ac
    this.lastRange = null

    let resolved: Resolved
    try {
      resolved = await resolveBasemapMode(this.terrain.manifestUrl, view)
    } catch {
      resolved = { mode: 'none', minZoom: 0, maxZoom: 6 }
    }
    if (ac.signal.aborted || this.disposed) return
    this.resolved = resolved

    if (resolved.mode === 'none') {
      this.engine.tex_layer_clear(ROLE_BASEMAP)
      this.cb.onDegraded?.(view)
      return
    }
    if (resolved.mode === 'unified' && resolved.unifiedUrl) {
      try {
        await this.loadUnified(resolved.unifiedUrl, satOpacity, ac.signal)
      } catch (err) {
        if (ac.signal.aborted || this.disposed) return
        console.warn('[wgpu-basemap] unified satellite failed, forcing pyramid:', err)
        this.cb.onProgress?.(null)
        await this.forcePyramid(view, satOpacity, ac.signal)
      }
      return
    }
    await this.loadPyramidOrSingle(resolved, satOpacity, ac.signal)
  }

  /** Hybrid/satellite style change — re-tint the loaded basemap without a reload. */
  setSatOpacity(satOpacity: number): void {
    this.satOpacity = satOpacity
    this.engine.set_lane_opacity(ROLE_BASEMAP, satOpacity, true)
  }

  private async forcePyramid(view: BasemapView, opacity: number, signal: AbortSignal): Promise<void> {
    let r: Resolved
    try {
      r = await resolveBasemapMode(this.terrain.manifestUrl, view, true)
    } catch {
      r = { mode: 'none', minZoom: 0, maxZoom: 6 }
    }
    if (signal.aborted || this.disposed) return
    this.resolved = r
    if (r.mode === 'none') {
      this.engine.tex_layer_clear(ROLE_BASEMAP)
      this.cb.onDegraded?.(view)
      return
    }
    await this.loadPyramidOrSingle(r, opacity, signal)
  }

  private async loadUnified(url: string, opacity: number, signal: AbortSignal): Promise<void> {
    const buf = await this.fetchStreaming(url, signal)
    if (signal.aborted) throw new DOMException('aborted', 'AbortError')
    const index = parseTbdSat(buf)
    const baseLevel = pickBaseLevel(index, this.engine.max_texture_dimension_2d)
    const base = index.mips[baseLevel]
    this.engine.tex_layer_begin(
      ROLE_BASEMAP,
      0,
      0,
      this.terrain.width,
      this.terrain.height,
      base.width,
      base.height,
      index.mipCount - baseLevel,
      MODE.unified,
    )
    const uploadBytes = index.mips
      .slice(baseLevel)
      .reduce((s, m) => s + m.tiles.reduce((a, t) => a + t.length, 0), 0)
    let doneBytes = 0
    let lastPct = -1
    const emit = (f: number) => {
      const pct = Math.floor(f * 50)
      if (pct === lastPct) return
      lastPct = pct
      this.cb.onProgress?.(Math.min(f, 1))
    }
    for (let level = baseLevel; level < index.mipCount; level++) {
      const mip = index.mips[level]
      const bitmaps = await Promise.all(
        mip.tiles.map((t) =>
          createImageBitmap(
            new Blob([new Uint8Array(buf, t.offset, t.length)], { type: 'image/webp' }),
            { colorSpaceConversion: 'none' },
          ),
        ),
      )
      if (signal.aborted) {
        bitmaps.forEach((b) => b.close())
        this.engine.tex_layer_clear(ROLE_BASEMAP)
        throw new DOMException('aborted', 'AbortError')
      }
      mip.tiles.forEach((t, i) => {
        this.uploadBlock(ROLE_BASEMAP, level - baseLevel, t.x, t.y, bitmaps[i])
        bitmaps[i].close()
        doneBytes += t.length
        emit(0.8 + (doneBytes / uploadBytes) * 0.2)
      })
    }
    this.engine.tex_layer_commit(ROLE_BASEMAP, opacity, true)
    emit(1)
    this.cb.onProgress?.(null)
  }

  private async loadPyramidOrSingle(
    resolved: Resolved,
    opacity: number,
    signal: AbortSignal,
  ): Promise<void> {
    if (resolved.mode === 'single-bitmap' && resolved.image) {
      const bmp = await this.fetchBitmap(resolved.image, signal)
      if (signal.aborted || this.disposed || !bmp) {
        bmp?.close()
        return
      }
      this.engine.tex_layer_begin(
        ROLE_BASEMAP,
        0,
        0,
        this.terrain.width,
        this.terrain.height,
        bmp.width,
        bmp.height,
        1,
        MODE.single,
      )
      this.uploadBlock(ROLE_BASEMAP, 0, 0, 0, bmp)
      bmp.close()
      this.engine.tex_layer_commit(ROLE_BASEMAP, opacity, true)
      return
    }
    if (resolved.mode === 'pyramid') await this.loadPyramid(resolved, opacity, signal)
  }

  private async loadPyramid(
    resolved: Resolved,
    opacity: number,
    signal: AbortSignal,
  ): Promise<void> {
    const lod = computeLod(resolved, this.viewState(), this.viewBounds(), this.terrain, true)
    if (lod.kind !== 'pyramid') {
      this.engine.tex_layer_clear(ROLE_BASEMAP)
      return
    }
    this.lastRange = {
      z: lod.z,
      txMin: lod.txMin,
      txMax: lod.txMax,
      tyMin: lod.tyMin,
      tyMax: lod.tyMax,
    }
    const n = 2 ** lod.z
    const twx = this.terrain.width / n
    const twy = this.terrain.height / n
    const tilesX = lod.txMax - lod.txMin + 1
    const tilesY = lod.tyMax - lod.tyMin + 1
    this.engine.tex_layer_begin(
      ROLE_BASEMAP,
      lod.txMin * twx,
      lod.tyMin * twy,
      (lod.txMax + 1) * twx,
      (lod.tyMax + 1) * twy,
      tilesX * TILE_PX,
      tilesY * TILE_PX,
      1,
      MODE.pyramid,
    )
    for (let ty = lod.tyMin; ty <= lod.tyMax; ty++) {
      for (let tx = lod.txMin; tx <= lod.txMax; tx++) {
        const bmp = await this.fetchBitmap(tileUrl(lod.template, lod.z, tx, ty), signal)
        if (signal.aborted || this.disposed) {
          bmp?.close()
          return
        }
        if (!bmp) continue
        // Pack north-at-top: mirror lanes::pack_offset (subX=(tx-txMin)*256, subY=(tyMax-ty)*256).
        const subX = (tx - lod.txMin) * TILE_PX
        const subY = (lod.tyMax - ty) * TILE_PX
        this.uploadBlock(ROLE_BASEMAP, 0, subX, subY, bmp)
        bmp.close()
      }
    }
    this.engine.tex_layer_commit(ROLE_BASEMAP, opacity, true)
  }

  /** Camera moved — for a pyramid basemap, recompute LOD (debounced) and reload if the tile range
   *  changed. Unified/single/none need no reload (the GPU picks the mip / one texture covers all). */
  onCameraMoved(): void {
    if (this.disposed || this.resolved?.mode !== 'pyramid') return
    if (this.lodTimer) clearTimeout(this.lodTimer)
    this.lodTimer = setTimeout(() => {
      if (this.disposed || !this.resolved) return
      const lod = computeLod(this.resolved, this.viewState(), this.viewBounds(), this.terrain, true)
      if (lod.kind !== 'pyramid') return
      const r = this.lastRange
      if (
        r &&
        r.z === lod.z &&
        r.txMin === lod.txMin &&
        r.txMax === lod.txMax &&
        r.tyMin === lod.tyMin &&
        r.tyMax === lod.tyMax
      )
        return
      const ac = new AbortController()
      this.loadAc?.abort()
      this.loadAc = ac
      void this.loadPyramid(this.resolved, this.satOpacity, ac.signal)
    }, LOD_DEBOUNCE_MS)
  }

  // ── hillshade (role 1) ─────────────────────────────────────────────────────────────────────
  /** Load the hillshade lane from the shared DemController metersCache, or clear it. Triggers the
   *  DEM load if needed (the hook re-calls this on demVersion once the cache is ready). */
  setHillshade(show: boolean, opacity: number): void {
    if (!show) {
      this.engine.tex_layer_clear(ROLE_HILLSHADE)
      return
    }
    if (!isDemReady()) {
      void loadDemForTerrain(this.terrain.id)
      return
    }
    const dem = getDemRasterForOverlay()
    if (!dem) return
    const hs = hillshade(dem.metersCache, dem.width, dem.height)
    try {
      this.engine.tex_layer_begin(
        ROLE_HILLSHADE,
        0,
        0,
        this.terrain.width,
        this.terrain.height,
        hs.width,
        hs.height,
        1,
        MODE.hillshade,
      )
      this.engine.tex_layer_write_rgba(ROLE_HILLSHADE, 0, 0, 0, hs.width, hs.height, hs.data)
      this.engine.tex_layer_commit(ROLE_HILLSHADE, opacity, true)
    } finally {
      hs.free()
    }
  }

  /** Blend-strength slider — re-tint without rebuilding the Horn image (L6 cheap-memo). */
  setHillshadeOpacity(opacity: number, show: boolean): void {
    this.engine.set_lane_opacity(ROLE_HILLSHADE, opacity, show)
  }

  // ── grid (role Grid) ───────────────────────────────────────────────────────────────────────
  setGrid(showGrid: boolean, overHillshade: boolean): void {
    this.engine.set_grid(this.terrain.width, this.terrain.height, overHillshade, showGrid)
  }

  // ── paper tint (map-style clear underlay, L8) ──────────────────────────────────────────────
  /** Map style → the cartographic paper tint as the frame clear; else the dark editor field. */
  setPaperTint(isMap: boolean): void {
    if (isMap) {
      this.engine.set_clear_color(PAPER_TINT[0] / 255, PAPER_TINT[1] / 255, PAPER_TINT[2] / 255)
    } else {
      // #0b0f14 — the editor's dark field (matches the container background).
      this.engine.set_clear_color(0x0b / 255, 0x0f / 255, 0x14 / 255)
    }
  }

  // ── helpers ────────────────────────────────────────────────────────────────────────────────
  private uploadBlock(role: number, mip: number, x: number, y: number, bmp: ImageBitmap): void {
    if (this.webgl2) {
      this.engine.tex_layer_write_rgba(role, mip, x, y, bmp.width, bmp.height, bitmapToRgba(bmp))
    } else {
      this.engine.tex_layer_write_bitmap(role, mip, x, y, bmp.width, bmp.height, bmp)
    }
  }

  private viewState(): MapViewState {
    return {
      target: [this.engine.target_x, this.engine.target_y],
      zoom: this.engine.zoom,
      minZoom: -6,
      maxZoom: 6,
    }
  }

  private viewBounds(): [number, number, number, number] {
    const b = this.engine.visible_bounds()
    return [b[0], b[1], b[2], b[3]]
  }

  private async fetchStreaming(url: string, signal: AbortSignal): Promise<ArrayBuffer> {
    const resp = await fetch(url, { signal })
    if (!resp.ok) throw new Error(`fetch ${resp.status} for ${url}`)
    const total = Number(resp.headers.get('content-length') ?? 0)
    if (!resp.body || total <= 0) {
      const b = await resp.arrayBuffer()
      this.cb.onProgress?.(0.8)
      return b
    }
    const reader = resp.body.getReader()
    const chunks: Uint8Array[] = []
    let received = 0
    let lastPct = -1
    for (;;) {
      const { done, value } = await reader.read()
      if (done) break
      chunks.push(value)
      received += value.byteLength
      const f = (received / total) * 0.8
      const pct = Math.floor(f * 50)
      if (pct !== lastPct) {
        lastPct = pct
        this.cb.onProgress?.(f)
      }
    }
    const out = new Uint8Array(received)
    let pos = 0
    for (const c of chunks) {
      out.set(c, pos)
      pos += c.byteLength
    }
    return out.buffer
  }

  private async fetchBitmap(url: string, signal: AbortSignal): Promise<ImageBitmap | null> {
    try {
      const resp = await fetch(url, { signal })
      if (!resp.ok) return null
      const blob = await resp.blob()
      return await createImageBitmap(blob, { colorSpaceConversion: 'none' })
    } catch {
      return null
    }
  }
}
