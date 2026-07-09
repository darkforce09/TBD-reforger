// T-151.7 — ULP-0 orthographic camera helpers for the wgpu interaction rewire.
// Shared Viewport surface used by useSelectTool + slotSpatialIndex + slotClusterIndex.
// Frozen gestures snapshot viewState via OrthoCameraJs (Class R vs Deck; no live engine needed).

import { OrthoCameraJs } from '@/wasm/pkg/map_engine_wasm'
import type { MapViewState } from '../types'

/** Minimal viewport surface for picks / gestures — screen CSS px → world meters. */
export interface MapViewport {
  unproject: (xy: number[]) => number[]
}

/** Zoom band mirrors `useOrthographicView` / Rust `OrthoCamera` MIN/MAX. */
export const MAP_MIN_ZOOM = -6
export const MAP_MAX_ZOOM = 6

export function clampMapZoom(zoom: number): number {
  if (zoom < MAP_MIN_ZOOM) return MAP_MIN_ZOOM
  if (zoom > MAP_MAX_ZOOM) return MAP_MAX_ZOOM
  return zoom
}

/**
 * Frozen viewport from a view-state snapshot (flipY:false orthographic).
 *
 * Uses OrthoCameraJs so unproject is ULP-0 vs Deck at integer zooms (same core as
 * RenderEngine.unproject_xy). The camera is retained on the returned object and freed
 * via `dispose()` when the gesture ends — useSelectTool drops the ref on pointer-up
 * and finalizers reclaim if dispose is skipped.
 */
export function viewportFromViewState(
  width: number,
  height: number,
  viewState: Pick<MapViewState, 'target' | 'zoom'>,
): MapViewport {
  const cam = new OrthoCameraJs(
    width,
    height,
    viewState.target[0],
    viewState.target[1],
    viewState.zoom,
  )
  return {
    unproject: (xy: number[]) => {
      const r = cam.unproject_xy(xy[0], xy[1])
      return [r[0], r[1]]
    },
  }
}

/** Live engine camera unproject (RenderEngine.unproject_xy after T-151.7). */
export function viewportFromEngine(engine: {
  unproject_xy: (px: number, py: number) => ArrayLike<number>
}): MapViewport {
  return {
    unproject: (xy: number[]) => {
      const r = engine.unproject_xy(xy[0], xy[1])
      return [r[0], r[1]]
    },
  }
}

/** Read MapViewState fields from a live RenderEngine. */
export function viewStateFromEngine(
  engine: { target_x: number; target_y: number; zoom: number },
  minZoom = MAP_MIN_ZOOM,
  maxZoom = MAP_MAX_ZOOM,
): MapViewState {
  return {
    target: [engine.target_x, engine.target_y],
    zoom: engine.zoom,
    minZoom,
    maxZoom,
  }
}

/** Apply a MapViewState to the engine (clamped zoom; target clamp is engine-side via bounds). */
export function applyViewState(
  engine: { set_view: (x: number, y: number, z: number) => void },
  viewState: Pick<MapViewState, 'target' | 'zoom'>,
): void {
  engine.set_view(viewState.target[0], viewState.target[1], viewState.zoom)
}

/**
 * World hit radius in meters for a screen-pixel pick radius (same as slotSpatialIndex):
 * r_world = |unproject(px + r_px) − unproject(px)| on x.
 */
export function worldPickRadius(
  viewport: MapViewport,
  px: [number, number],
  radiusPx: number,
): number {
  const center = viewport.unproject(px)
  const edge = viewport.unproject([px[0] + radiusPx, px[1]])
  return Math.abs(edge[0] - center[0])
}
