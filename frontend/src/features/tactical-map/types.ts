// Engine-public types. The tactical-map engine is terrain-agnostic and will be
// reused by the future Mission Planner, so this is the only type surface consumers
// should import (via the barrel `index.ts`).

import type { TerrainId } from './coords/terrains'

export type { TerrainId } from './coords/terrains'

/** OrthographicView state. target is [x, y] in Deck common/pixel space; zoom is
 *  log2 scale (higher = closer). */
export interface MapViewState {
  target: [number, number]
  zoom: number
  minZoom: number
  maxZoom: number
}

export interface TacticalMapProps {
  /** Which terrain's world bounds the camera and base map are sized to. */
  terrain?: TerrainId
  /** Extra classes for the absolutely-positioned canvas container. */
  className?: string
  /** Fired on an empty-map click with the world (meters) position picked. */
  onMapClick?: (world: { x: number; y: number }) => void
}
