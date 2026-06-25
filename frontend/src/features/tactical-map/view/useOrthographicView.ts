// OrthographicView config + clamped view state. The clamps keep the camera inside
// the terrain and the zoom in a sane band so panning/zooming can't fling the user
// into empty space — and so we hold 60 fps (all work happens in the change handler,
// never per frame). See Ultra Plan §4.1.

import { useCallback, useMemo, useState } from 'react'
import { OrthographicView } from '@deck.gl/core'
import type { TerrainDef } from '../coords/terrains'
import { terrainCenterPixel } from '../coords/projection'
import type { MapViewState } from '../types'

const MIN_ZOOM = -6 // fully zoomed out: whole terrain visible
const MAX_ZOOM = 6 // close inspection of individual entities

function clamp(value: number, min: number, max: number): number {
  return value < min ? min : value > max ? max : value
}

interface UseOrthographicView {
  view: OrthographicView
  viewState: MapViewState
  onViewStateChange: (params: { viewState: MapViewState }) => void
  /** Recenter the camera on a common-space target [x, y], optionally bumping the zoom by
   *  `zoomDelta` (cluster drill-in, T-065). Stable identity. */
  flyTo: (target: [number, number], zoomDelta?: number) => void
}

export function useOrthographicView(terrain: TerrainDef): UseOrthographicView {
  const view = useMemo(
    () => new OrthographicView({ id: 'tactical-ortho', flipY: false }),
    [],
  )

  const [viewState, setViewState] = useState<MapViewState>(() => ({
    target: terrainCenterPixel(terrain),
    zoom: -2, // opens in detail mode (≥ ZOOM_DETAIL_MIN); pan/zoom out to cluster (T-065)
    minZoom: MIN_ZOOM,
    maxZoom: MAX_ZOOM,
  }))

  const onViewStateChange = useCallback(
    ({ viewState: next }: { viewState: MapViewState }) => {
      const zoom = clamp(next.zoom, MIN_ZOOM, MAX_ZOOM)
      const [tx, ty] = next.target
      setViewState({
        target: [clamp(tx, 0, terrain.width), clamp(ty, 0, terrain.height)],
        zoom,
        minZoom: MIN_ZOOM,
        maxZoom: MAX_ZOOM,
      })
    },
    [terrain.width, terrain.height],
  )

  const flyTo = useCallback(
    (target: [number, number], zoomDelta?: number) => {
      setViewState((prev) => ({
        ...prev,
        target: [clamp(target[0], 0, terrain.width), clamp(target[1], 0, terrain.height)],
        zoom: zoomDelta ? clamp(prev.zoom + zoomDelta, MIN_ZOOM, MAX_ZOOM) : prev.zoom,
      }))
    },
    [terrain.width, terrain.height],
  )

  return { view, viewState, onViewStateChange, flyTo }
}
