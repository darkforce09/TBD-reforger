// T-151.4 — DEM vectors for wgpu: sea band + contours (mirror demVectorStore, no worker).
// DemController meters cache → wasm DemGrid.downsample → sea_band/contours → engine upload.
// Contour interval rebuilds only when contourIntervalForZoom band changes (L4/L13).

import { useEffect, useRef } from 'react'
import type { RefObject } from 'react'
// T-151.11.3 (audit B-01): every policy call below is the wasm export — the old live imports
// from worldmap/{lodGates,seaBand,demGrid}.ts were TS twins of existing Rust fns; those files
// are oracle-only now.
import {
  DemGrid,
  contour_levels,
  compose_contours_hairline,
  class_visible,
  contour_interval_for_zoom,
  contour_grid_reductions,
  sea_fill_alpha,
  dem_vector_grid_factor,
} from '@/wasm/pkg/map_engine_wasm'
import {
  loadDemForTerrain,
  getDemRasterForOverlay,
  subscribeDem,
} from '../dem/DemController'
import type { TerrainDef } from '../coords/terrains'
import type { RenderEngine } from './wasmRender'

/** Engine role ids (match map-engine-render `lane_role_from_u32`). */
const ROLE_SEA = 0
const ROLE_CONTOURS = 2

/**
 * Imperative DEM-vector controller (effect-local, like WgpuBasemapController).
 * Call `sync(engine, zoom)` when ready / zoom changes / DEM lands.
 */
export class WgpuDemVectorController {
  private disposed = false
  private seaBuilt = false
  private lastInterval = 0
  private lastSeaAlpha = -1
  private grid: DemGrid | null = null

  dispose(): void {
    this.disposed = true
    this.grid?.free()
    this.grid = null
  }

  /** Rebuild grid from DemController if needed, then push sea/contours under LOD gates. */
  sync(engine: RenderEngine, deckZoom: number): void {
    if (this.disposed) return
    if (!this.ensureGrid()) {
      engine.clear_vector_lane(ROLE_SEA)
      engine.clear_vector_lane(ROLE_CONTOURS)
      return
    }
    this.pushSea(engine, deckZoom)
    this.pushContours(engine, deckZoom)
  }

  private ensureGrid(): boolean {
    if (this.grid) return true
    const raster = getDemRasterForOverlay()
    if (!raster?.metersCache) return false
    try {
      const worldW = raster.width > 0 ? raster.width * 2 : 12_800
      const worldH = raster.height > 0 ? raster.height * 2 : 12_800
      this.grid = DemGrid.downsample(
        raster.metersCache as Float32Array,
        raster.width,
        raster.height,
        dem_vector_grid_factor(),
        worldW,
        worldH,
      )
      this.seaBuilt = false
      this.lastInterval = 0
      return true
    } catch (err) {
      console.warn('[wgpu-dem] downsample failed', err)
      return false
    }
  }

  private pushSea(engine: RenderEngine, deckZoom: number): void {
    if (!this.grid) return
    const seaVis = class_visible('sea', deckZoom)
    const alpha = sea_fill_alpha(deckZoom)
    if (!seaVis || alpha <= 0) {
      engine.clear_vector_lane(ROLE_SEA)
      this.lastSeaAlpha = -1
      return
    }
    if (this.seaBuilt && this.lastSeaAlpha === alpha) return
    try {
      const sea = this.grid.sea_band()
      const mesh = sea.compose_mesh(alpha)
      engine.upload_polygon_mesh(
        ROLE_SEA,
        mesh.positions,
        mesh.colors,
        mesh.indices,
        mesh.polygon_count,
        true,
      )
      mesh.free()
      sea.free()
      this.seaBuilt = true
      this.lastSeaAlpha = alpha
    } catch (err) {
      console.warn('[wgpu-dem] sea compose failed', err)
    }
  }

  private pushContours(engine: RenderEngine, deckZoom: number): void {
    if (!this.grid) return
    const contVis = class_visible('contour', deckZoom)
    const interval = contour_interval_for_zoom(deckZoom)
    if (!contVis) {
      engine.clear_vector_lane(ROLE_CONTOURS)
      this.lastInterval = 0
      return
    }
    if (interval === this.lastInterval) return
    try {
      let g: DemGrid = this.grid
      // Coarser intervals use 2× reductions — Rust SoT (was an inline TS ladder; B-01).
      const reductions = contour_grid_reductions(interval)
      const owned: DemGrid[] = []
      for (let i = 0; i < reductions; i++) {
        const next = g.reduce()
        owned.push(next)
        g = next
      }
      const levels = contour_levels(interval, g.max_elev_m)
      const segs = g.contours(levels)
      const hair = compose_contours_hairline(segs)
      engine.upload_hairline_segments(ROLE_CONTOURS, hair.verts, hair.segment_count, true)
      hair.free()
      for (const o of owned) o.free()
      this.lastInterval = interval
    } catch (err) {
      console.warn('[wgpu-dem] contour compose failed', err)
    }
  }
}

/** React glue: load DEM, subscribe, sync on zoom/DEM. */
export function useWgpuDemVectors(
  engineRef: RefObject<RenderEngine | null>,
  ready: boolean,
  opts: { terrain: TerrainDef },
): void {
  const ctrlRef = useRef<WgpuDemVectorController | null>(null)
  const zoomRef = useRef(-2)

  useEffect(() => {
    if (!ready) return
    const ctrl = new WgpuDemVectorController()
    ctrlRef.current = ctrl
    void loadDemForTerrain(opts.terrain.id)
    const unsub = subscribeDem(() => {
      const eng = engineRef.current
      if (eng) ctrl.sync(eng, zoomRef.current)
    })
    // Initial sync once DEM may already be cached.
    const eng = engineRef.current
    if (eng) ctrl.sync(eng, zoomRef.current)
    return () => {
      unsub()
      ctrl.dispose()
      ctrlRef.current = null
    }
  }, [ready, opts.terrain.id, engineRef])

  // Poll zoom from the engine (same pattern as basemap LOD — cheap getter).
  useEffect(() => {
    if (!ready) return
    let raf = 0
    const tick = () => {
      const eng = engineRef.current
      const ctrl = ctrlRef.current
      if (eng && ctrl) {
        const z = eng.zoom
        if (z !== zoomRef.current) {
          zoomRef.current = z
          ctrl.sync(eng, z)
        }
      }
      raf = requestAnimationFrame(tick)
    }
    raf = requestAnimationFrame(tick)
    return () => cancelAnimationFrame(raf)
  }, [ready, engineRef])
}
