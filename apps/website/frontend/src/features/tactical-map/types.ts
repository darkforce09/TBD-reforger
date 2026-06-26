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

/** dataTransfer MIME used to drag an Asset Browser leaf onto the map. */
export const ASSET_DND_MIME = 'application/x-tbd-asset'

/** Payload carried in dataTransfer when dragging a catalog asset onto the map. */
export interface AssetDropPayload {
  /** Catalog asset id (registry classname once the worker lands). */
  assetId: string
  /** Human role/label → the new slot's `role` until the registry feed exists. */
  role: string
  /** What entity to materialize. Only 'slot' is wired today. */
  kind: 'slot'
}

export interface TacticalMapProps {
  /** Which terrain's world bounds the camera and base map are sized to. */
  terrain?: TerrainId
  /** Draw the procedural 1 km grid (off by default while the shell is in test mode). */
  showGrid?: boolean
  /** Extra classes for the absolutely-positioned canvas container. */
  className?: string
  /** Fired on hover with the world (meters) cursor position, or null when off-map.
   *  `z` is 0 on the flat map and stays 0 until Phase 2 DEM sampling lands. */
  onCursorMove?: (world: { x: number; y: number; z: number } | null) => void
  /** Receives the imperative map API (e.g. flyTo) for use by sibling panels. */
  onReady?: (api: TacticalMapApi) => void
  /** Fired when an entity icon is double-clicked (e.g. open the Attributes modal). */
  onEntityActivate?: (id: string) => void
  /** Fired when an Asset Browser leaf is dropped, with the unprojected world pos. */
  onAssetDrop?: (payload: AssetDropPayload, world: { x: number; y: number }) => void
  /** Commit a drag-move of one or more entities by a world-meter delta (Phase 7b). */
  onEntitiesMove?: (ids: string[], delta: { x: number; y: number }) => void
}

/** Imperative handle exposed via onReady. */
export interface TacticalMapApi {
  /** Center the camera on a world (meters) position. */
  flyTo: (world: { x: number; y: number }) => void
}
