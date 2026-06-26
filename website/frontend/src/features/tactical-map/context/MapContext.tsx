// Shared map context: coordinate transforms + flyTo(), so panels (Outliner,
// Inspector) added in later phases can centre the camera on an entity without
// touching Deck internals. Phase 1 wires flyTo() to recenter the view target.

import { createContext, useContext } from 'react'
import type { TerrainDef } from '../coords/terrains'
import { worldToPixel } from '../coords/projection'

export interface MapContextValue {
  terrain: TerrainDef
  /** Centre the camera on a world position (Arma meters). */
  flyTo: (world: { x: number; y: number }) => void
  /** World meters -> Deck common/pixel space. */
  worldToPixel: (x: number, y: number) => [number, number]
}

const MapContext = createContext<MapContextValue | null>(null)

export const MapContextProvider = MapContext.Provider

export function useMapContext(): MapContextValue {
  const ctx = useContext(MapContext)
  if (!ctx) {
    throw new Error('useMapContext must be used within a <TacticalMap>')
  }
  return ctx
}

/** Helper for building a context value bound to a terrain. */
export function createMapContextValue(
  terrain: TerrainDef,
  flyTo: MapContextValue['flyTo'],
): MapContextValue {
  return {
    terrain,
    flyTo,
    worldToPixel: (x, y) => worldToPixel(terrain, x, y),
  }
}
