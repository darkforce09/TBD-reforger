// World bbox of the current viewport (T-067.0). Mirrors the flipY:false makeViewport +
// unproject math already used in TacticalMap (onDrop / emitCursor): the icon cull needs the
// visible world rectangle to pick chunks. Kept out of TacticalMap so the cull math is
// testable and the layer hook stays presentation-only.

import type { OrthographicView } from '@deck.gl/core'
import type { TerrainDef } from '../coords/terrains'
import type { MapViewState } from '../types'
import { CHUNK_SIZE_M } from './spatialChunks'

/** Screen → world: unproject the [0,0] and [width,height] container corners through a viewport
 *  built from the live view + viewState, then min/max into [minX,minY,maxX,maxY] world meters.
 *  Returns null when the viewport can't be built or the container is unmeasured (0×0). */
export function worldBboxFromViewport(
  view: OrthographicView,
  viewState: MapViewState,
  width: number,
  height: number,
): [number, number, number, number] | null {
  if (width <= 0 || height <= 0) return null
  const viewport = view.makeViewport({ width, height, viewState })
  if (!viewport) return null
  const [ax, ay] = viewport.unproject([0, 0])
  const [bx, by] = viewport.unproject([width, height])
  return [Math.min(ax, bx), Math.min(ay, by), Math.max(ax, bx), Math.max(ay, by)]
}

function clamp(v: number, min: number, max: number): number {
  return v < min ? min : v > max ? max : v
}

/** Pad a world bbox by `halo` chunks (meters) on every side, clamped to terrain bounds. The
 *  cull region is the viewport bbox grown by one chunk ring so a pan never reveals a seam hole
 *  before the next bbox recompute. */
export function expandedBboxForHalo(
  bbox: [number, number, number, number],
  terrain: TerrainDef,
  halo: number,
): [number, number, number, number] {
  const pad = halo * CHUNK_SIZE_M
  return [
    clamp(bbox[0] - pad, 0, terrain.width),
    clamp(bbox[1] - pad, 0, terrain.height),
    clamp(bbox[2] + pad, 0, terrain.width),
    clamp(bbox[3] + pad, 0, terrain.height),
  ]
}
