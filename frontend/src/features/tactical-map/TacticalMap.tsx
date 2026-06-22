// <TacticalMap> — the Deck.gl root. Owns the OrthographicView state, assembles the
// layer array, and provides MapContext. Phase 1 renders only the procedural-grid
// base map; later phases append DEM, icon, line, polygon, viewshed and selection
// layers (Ultra Plan §4.3). Deck owns all entity rendering — React never draws
// per-entity DOM — which is what holds 60 fps with hundreds of slots.

import { useCallback, useEffect, useMemo, useRef } from 'react'
import DeckGL from '@deck.gl/react'
import type { DeckGLRef } from '@deck.gl/react'
import type { PickingInfo } from '@deck.gl/core'
import { getTerrain } from './coords/terrains'
import { useOrthographicView } from './view/useOrthographicView'
import { useBaseMapLayer } from './layers/useBaseMapLayer'
import { useIconLayer } from './layers/useIconLayer'
import { useSelectionLayer } from './layers/useSelectionLayer'
import { useSelectTool } from './tools/useSelectTool'
import { MapContextProvider, createMapContextValue } from './context/MapContext'
import { useMapStore } from './state/useMapStore'
import type { ID } from './state/schema'
import { ASSET_DND_MIME, type AssetDropPayload, type MapViewState, type TacticalMapProps } from './types'

export function TacticalMap({
  terrain: terrainId,
  showGrid = false,
  className,
  onCursorMove,
  onReady,
  onEntityActivate,
  onAssetDrop,
  onEntitiesMove,
}: TacticalMapProps) {
  const terrain = useMemo(() => getTerrain(terrainId), [terrainId])
  const { view, viewState, onViewStateChange, flyTo: viewFlyTo } =
    useOrthographicView(terrain)
  const baseMap = useBaseMapLayer(terrain)
  const iconLayer = useIconLayer()
  const selectionLayer = useSelectionLayer()
  const setSelection = useMapStore((s) => s.setSelection)
  // Drop zone + pointer-gesture host: Deck's controller ignores HTML5 drag/drop and
  // (with dragPan off) our custom drags, so both bubble to this container.
  const containerRef = useRef<HTMLDivElement>(null)
  const deckRef = useRef<DeckGLRef | null>(null)

  const noopMove = useCallback(() => {}, [])
  const selectTool = useSelectTool({
    deckRef,
    containerRef,
    view,
    viewState,
    onViewStateChange,
    onEntitiesMove: onEntitiesMove ?? noopMove,
  })

  const onClick = useCallback(
    // Deck passes the gesture event as the 2nd arg; its srcEvent carries the modifiers.
    (info: PickingInfo, event?: { srcEvent?: { ctrlKey?: boolean; metaKey?: boolean } }) => {
      const src = event?.srcEvent
      const additive = !!(src && (src.ctrlKey || src.metaKey)) // Ctrl/Cmd toggle (P1-01)
      if (info.layer?.id === 'slot-icons' && info.object) {
        const id = (info.object as { id: ID }).id
        if (additive) {
          // Ctrl/Cmd-click toggles this slot in/out of the current selection.
          const sel = useMapStore.getState().selection
          const cur = sel.kind === 'slot' ? sel.ids : []
          const next = cur.includes(id) ? cur.filter((x) => x !== id) : [...cur, id]
          setSelection(next.length ? { kind: 'slot', ids: next } : { kind: 'none', ids: [] })
          return
        }
        setSelection({ kind: 'slot', ids: [id] })
        return
      }
      // Ctrl/Cmd + empty click preserves the selection; a plain empty click deselects
      // (no teleport — Phase 7b removed it).
      if (additive) return
      setSelection({ kind: 'none', ids: [] })
    },
    [setSelection],
  )

  // Double-click a slot icon → activate it (host opens the Attributes modal). Deck has no
  // onDblClick, so we listen for the container's native dblclick and pick the slot under the
  // cursor — the same pick useSelectTool.onPointerDown does. Matches the outliner trees,
  // which open Attributes via TreeView's native onDoubleClick.
  const onDoubleClick = useCallback(
    (e: React.MouseEvent) => {
      const el = containerRef.current
      const deck = deckRef.current
      if (!el || !deck) return
      const r = el.getBoundingClientRect()
      const info = deck.pickObject({
        x: e.clientX - r.left,
        y: e.clientY - r.top,
        radius: 4,
        layerIds: ['slot-icons'],
      })
      const id = (info?.object as { id: ID } | undefined)?.id
      if (id) onEntityActivate?.(id)
    },
    [onEntityActivate],
  )

  const onHover = useCallback(
    (info: PickingInfo) => {
      onCursorMove?.(
        info.coordinate
          ? { x: info.coordinate[0], y: info.coordinate[1], z: info.coordinate[2] ?? 0 }
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

  // Only accept our asset drags (lets every other drag fall through to the page).
  const onDragOver = useCallback((e: React.DragEvent) => {
    if (!e.dataTransfer.types.includes(ASSET_DND_MIME)) return
    e.preventDefault()
    e.dataTransfer.dropEffect = 'copy'
  }, [])

  // Unproject the drop point: build a viewport from the current view + container
  // size and convert screen px → world meters (same flipY:false math as onClick's
  // info.coordinate, so a drop lands exactly under the cursor).
  const onDrop = useCallback(
    (e: React.DragEvent) => {
      const raw = e.dataTransfer.getData(ASSET_DND_MIME)
      const el = containerRef.current
      if (!raw || !el) return
      e.preventDefault()
      let payload: AssetDropPayload
      try {
        payload = JSON.parse(raw) as AssetDropPayload
      } catch {
        return
      }
      const rect = el.getBoundingClientRect()
      const viewport = view.makeViewport({
        width: rect.width,
        height: rect.height,
        viewState,
      })
      if (!viewport) return
      const [x, y] = viewport.unproject([e.clientX - rect.left, e.clientY - rect.top])
      onAssetDrop?.(payload, { x, y })
    },
    [view, viewState, onAssetDrop],
  )

  // Expose flyTo to sibling panels (Outliner) that live outside MapContext.
  useEffect(() => onReady?.({ flyTo }), [onReady, flyTo])

  const ctx = useMemo(
    () => createMapContextValue(terrain, flyTo),
    [terrain, flyTo],
  )

  return (
    <MapContextProvider value={ctx}>
      <div
        ref={containerRef}
        className={className ?? 'absolute inset-0'}
        onDragOver={onDragOver}
        onDrop={onDrop}
        onPointerDown={selectTool.onPointerDown}
        onPointerMove={selectTool.onPointerMove}
        onPointerUp={selectTool.onPointerUp}
        onDoubleClick={onDoubleClick}
        onContextMenu={selectTool.onContextMenu}
      >
        <DeckGL
          ref={deckRef}
          views={view}
          viewState={viewState}
          onViewStateChange={(params) =>
            onViewStateChange({ viewState: params.viewState as MapViewState })
          }
          // dragPan off: left-drag is select/move, middle/right-drag pans (useSelectTool).
          controller={{ dragPan: false, doubleClickZoom: false }}
          layers={[...(showGrid ? [baseMap] : []), iconLayer, selectionLayer]}
          onClick={onClick}
          onHover={onHover}
          getCursor={({ isHovering }) => (isHovering ? 'pointer' : 'grab')}
          style={{ position: 'absolute', width: '100%', height: '100%' }}
        />
      </div>
    </MapContextProvider>
  )
}
