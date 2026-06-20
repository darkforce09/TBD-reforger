// <TacticalMap> — the Deck.gl root. Owns the OrthographicView state, assembles the
// layer array, and provides MapContext. Phase 1 renders only the procedural-grid
// base map; later phases append DEM, icon, line, polygon, viewshed and selection
// layers (Ultra Plan §4.3). Deck owns all entity rendering — React never draws
// per-entity DOM — which is what holds 60 fps with hundreds of slots.

import { useCallback, useEffect, useMemo, useRef } from 'react'
import DeckGL from '@deck.gl/react'
import type { PickingInfo } from '@deck.gl/core'
import { getTerrain } from './coords/terrains'
import { useOrthographicView } from './view/useOrthographicView'
import { useBaseMapLayer } from './layers/useBaseMapLayer'
import { useIconLayer } from './layers/useIconLayer'
import { MapContextProvider, createMapContextValue } from './context/MapContext'
import { useMapStore } from './state/useMapStore'
import type { ID } from './state/schema'
import type { MapViewState, TacticalMapProps } from './types'

export function TacticalMap({
  terrain: terrainId,
  className,
  onMapClick,
  onCursorMove,
  onReady,
  onEntityActivate,
}: TacticalMapProps) {
  const terrain = useMemo(() => getTerrain(terrainId), [terrainId])
  const { view, viewState, onViewStateChange, flyTo: viewFlyTo } =
    useOrthographicView(terrain)
  const baseMap = useBaseMapLayer(terrain)
  const iconLayer = useIconLayer()
  const setSelection = useMapStore((s) => s.setSelection)
  // Manual double-click detection (Deck has no onDblClick) for entity activation.
  const lastClick = useRef<{ id: string; ts: number } | null>(null)

  const onClick = useCallback(
    (info: PickingInfo) => {
      if (info.layer?.id === 'slot-icons' && info.object) {
        const id = (info.object as { id: ID }).id
        setSelection({ kind: 'slot', id })
        const prev = lastClick.current
        const now = Date.now()
        if (prev && prev.id === id && now - prev.ts < 350) {
          onEntityActivate?.(id)
          lastClick.current = null
        } else {
          lastClick.current = { id, ts: now }
        }
        return
      }
      lastClick.current = null
      // Empty-map click: let the host act (e.g. move the selected slot), then clear.
      if (info.coordinate) {
        onMapClick?.({ x: info.coordinate[0], y: info.coordinate[1] })
      }
      setSelection({ kind: 'none', id: null })
    },
    [setSelection, onMapClick, onEntityActivate],
  )

  const onHover = useCallback(
    (info: PickingInfo) => {
      onCursorMove?.(
        info.coordinate
          ? { x: info.coordinate[0], y: info.coordinate[1] }
          : null,
      )
    },
    [onCursorMove],
  )

  // Identity projection (flipY:false → common space == world space), so a world
  // position centers directly. Stable across renders for the onReady handle.
  const flyTo = useCallback(
    (world: { x: number; y: number }) => viewFlyTo([world.x, world.y]),
    [viewFlyTo],
  )

  // Expose flyTo to sibling panels (Outliner) that live outside MapContext.
  useEffect(() => onReady?.({ flyTo }), [onReady, flyTo])

  const ctx = useMemo(
    () => createMapContextValue(terrain, flyTo),
    [terrain, flyTo],
  )

  return (
    <MapContextProvider value={ctx}>
      <div className={className ?? 'absolute inset-0'}>
        <DeckGL
          views={view}
          viewState={viewState}
          onViewStateChange={(params) =>
            onViewStateChange({ viewState: params.viewState as MapViewState })
          }
          controller={{ doubleClickZoom: false }}
          layers={[baseMap, iconLayer]}
          onClick={onClick}
          onHover={onHover}
          getCursor={({ isHovering }) => (isHovering ? 'pointer' : 'grab')}
          style={{ position: 'absolute', width: '100%', height: '100%' }}
        />
      </div>
    </MapContextProvider>
  )
}
