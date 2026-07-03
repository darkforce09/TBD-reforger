// Engine-public types. The tactical-map engine is terrain-agnostic and will be
// reused by the future Mission Planner, so this is the only type surface consumers
// should import (via the barrel `index.ts`).

import type { TerrainId } from './coords/terrains'
import type { BasemapView } from './state/basemapView'

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
  /** Registry `resource_name` — the full Enfusion ResourceName, e.g.
   *  `{GUID}Prefabs/.../File.et` (not a mock id, not a "classname"). */
  assetId: string
  /** Human role/label (registry `display_name`) → the new slot's `role`. */
  role: string
  /** What entity to materialize. Only 'slot' is wired today. */
  kind: 'slot'
}

export interface TacticalMapProps {
  /** Which terrain's world bounds the camera and base map are sized to. */
  terrain?: TerrainId
  /** Draw the procedural 1 km grid (off by default while the shell is in test mode). */
  showGrid?: boolean
  /** Draw the DEM hillshade overlay when the terrain DEM is ready (T-091.2). */
  showHillshade?: boolean
  /** Hillshade overlay blend strength 0–1 (T-090.1.2.6). Default 0.4; ≤0 skips the layer. */
  hillshadeOpacity?: number
  /** Extra classes for the absolutely-positioned canvas container. */
  className?: string
  /** Fired on hover with the world (meters) cursor position, or null when off-map.
   *  `z` is the sampled terrain elevation when the DEM is ready, else 0 (T-091.2). */
  onCursorMove?: (world: { x: number; y: number; z: number } | null) => void
  /** Receives the imperative map API (e.g. flyTo) for use by sibling panels. */
  onReady?: (api: TacticalMapApi) => void
  /** Fired when an entity icon is double-clicked (e.g. open the Attributes modal). */
  onEntityActivate?: (id: string) => void
  /** Fired when an Asset Browser leaf is dropped, with the unprojected world pos. */
  onAssetDrop?: (payload: AssetDropPayload, world: { x: number; y: number }) => void
  /** Commit a drag-move of one or more entities by a world-meter delta (Phase 7b). */
  onEntitiesMove?: (ids: string[], delta: { x: number; y: number }) => void
  /** Fired when a basemap view can't load (404) → host shows a grid-only toast (T-090.1/.1.1). */
  onBasemapDegraded?: (view: BasemapView) => void
  /** Unified satellite bundle load progress (T-090.1.2.8): fraction 0..1 while fetching +
   *  decoding (1 = GPU texture live); null = load abandoned (fallback/unmount) → dismiss. */
  onBasemapProgress?: (fraction: number | null) => void
}

/** Imperative handle exposed via onReady. */
export interface TacticalMapApi {
  /** Center the camera on a world (meters) position. */
  flyTo: (world: { x: number; y: number }) => void
}
