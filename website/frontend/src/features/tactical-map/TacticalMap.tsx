// <TacticalMap> — the Deck.gl root. Owns the OrthographicView state, assembles the
// layer array, and provides MapContext. Phase 1 renders only the procedural-grid
// base map; later phases append DEM, icon, line, polygon, viewshed and selection
// layers (Ultra Plan §4.3). Deck owns all entity rendering — React never draws
// per-entity DOM — which is what holds 60 fps with hundreds of slots.

import { memo, useCallback, useEffect, useMemo, useRef } from 'react'
import DeckGL from '@deck.gl/react'
import type { DeckGLRef } from '@deck.gl/react'
import { getTerrain } from './coords/terrains'
import { useOrthographicView } from './view/useOrthographicView'
import { useBaseMapLayer } from './layers/useBaseMapLayer'
import { useIconLayer, useDragIconLayer } from './layers/useIconLayer'
import { useClusterIconLayer } from './layers/useClusterIconLayer'
import { useSelectionLayer } from './layers/useSelectionLayer'
import { useSelectTool } from './tools/useSelectTool'
import { MapContextProvider, createMapContextValue } from './context/MapContext'
import * as slotSpatialIndex from './state/slotSpatialIndex'
import * as slotClusterIndex from './state/slotClusterIndex'
import * as slotIconCache from './state/slotIconCache'
import { useMapStore } from './state/useMapStore'
import { ZOOM_CLUSTER_MAX, CLUSTER_SLOT_THRESHOLD } from './state/constants'
import { ASSET_DND_MIME, type AssetDropPayload, type MapViewState, type TacticalMapProps } from './types'

function TacticalMapInner({
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

  // Drop zone + pointer-gesture host: Deck's controller ignores HTML5 drag/drop and
  // (with dragPan off) our custom drags, so both bubble to this container.
  const containerRef = useRef<HTMLDivElement>(null)
  const deckRef = useRef<DeckGLRef | null>(null)

  // Cluster / LOD gating (T-065.2): only at/below ZOOM_CLUSTER_MAX (-4) on a large mission do we draw
  // cluster discs (useClusterIconLayer) instead of every icon, with the base IconLayer rendering the
  // selection only. @ ~367k, detail mode (all rings) already pans at ~160 fps, so clustering is
  // reserved for extreme zoom-out. Derived from store slices that change on edit/zoom/selection.
  const slotCount = useMapStore((s) => s.slotCount)
  const selection = useMapStore((s) => s.selection)
  const clusterMode = slotCount > CLUSTER_SLOT_THRESHOLD && viewState.zoom <= ZOOM_CLUSTER_MAX

  const baseMap = useBaseMapLayer(terrain)
  const iconLayer = useIconLayer({ detail: !clusterMode, selection })
  const dragIconLayer = useDragIconLayer()
  const selectionLayer = useSelectionLayer()
  // Pan-stable cluster layer (T-065.2): reads the full-terrain module cache, so a pan returns the
  // same data reference and the layer is not rebuilt per frame.
  const clusterLayers = useClusterIconLayer({ clusterMode, deckZoom: viewState.zoom })

  // Keep the cluster index's normalization window + the cull chunk grid aligned with the terrain.
  useEffect(() => {
    slotClusterIndex.setTerrain(terrain)
    slotIconCache.setChunkTerrain(terrain)
  }, [terrain])

  // Cluster drill-in (T-065): recenter on a cluster centroid and zoom one step closer.
  const drillIntoCluster = useCallback(
    (world: { x: number; y: number }) => viewFlyTo([world.x, world.y], 1),
    [viewFlyTo],
  )
  // Latest camera for the rAF cursor closure (so it reads fresh viewState without
  // re-scheduling on every render).
  const viewStateRef = useRef(viewState)
  useEffect(() => {
    viewStateRef.current = viewState
  }, [viewState])

  const noopMove = useCallback(() => {}, [])
  const selectTool = useSelectTool({
    containerRef,
    view,
    viewState,
    onViewStateChange,
    onEntitiesMove: onEntitiesMove ?? noopMove,
    clusterMode,
    onClusterDrill: drillIntoCluster,
  })

  // Click-select (select / Ctrl-toggle / deselect) lives in useSelectTool's pending-left
  // pointerUp now — Deck's `slot-icons` layer is no longer pickable (T-063), so there is no
  // Deck onClick handler.

  // Double-click a slot icon → activate it (host opens the Attributes modal). Deck has no
  // onDblClick, so we listen for the container's native dblclick and pick the slot under the
  // cursor via the spatial index (T-063) — same query useSelectTool uses. Matches the outliner
  // trees, which open Attributes via TreeView's native onDoubleClick.
  const onDoubleClick = useCallback(
    (e: React.MouseEvent) => {
      const el = containerRef.current
      if (!el) return
      const r = el.getBoundingClientRect()
      const viewport = view.makeViewport({ width: r.width, height: r.height, viewState })
      if (!viewport) return
      const px: [number, number] = [e.clientX - r.left, e.clientY - r.top]
      // Cluster mode (T-065): a dbl-click drills into the cluster under the cursor (zoom in)
      // rather than opening Attributes on a hidden individual slot.
      if (clusterMode) {
        const marker = slotClusterIndex.pickClusterAt(px, viewport, viewState.zoom)
        if (marker) drillIntoCluster({ x: marker.x, y: marker.y })
        return
      }
      const id = slotSpatialIndex.pickNearest(px, viewport)
      if (id) onEntityActivate?.(id)
    },
    [view, viewState, onEntityActivate, clusterMode, drillIntoCluster],
  )

  // Cursor read-out (toolbelt X/Y/Z) — computed by unprojecting the mouse ourselves and
  // rAF-throttled, instead of Deck's onHover. Deck's hover handler ran a GPU pick pass over
  // every icon on each pointer move just to get cursor coords (the 200-slot fps killer,
  // T-057); the same flipY:false math as onDrop gives us the world position with no pick.
  const cursorRaf = useRef(0)
  const lastClientPt = useRef<{ x: number; y: number } | null>(null)
  const emitCursor = useCallback(
    (e: React.PointerEvent) => {
      if (!onCursorMove) return
      lastClientPt.current = { x: e.clientX, y: e.clientY }
      if (cursorRaf.current) return
      cursorRaf.current = requestAnimationFrame(() => {
        cursorRaf.current = 0
        const el = containerRef.current
        const pt = lastClientPt.current
        if (!el || !pt) return
        const rect = el.getBoundingClientRect()
        const viewport = view.makeViewport({
          width: rect.width,
          height: rect.height,
          viewState: viewStateRef.current,
        })
        if (!viewport) return
        const [x, y] = viewport.unproject([pt.x - rect.left, pt.y - rect.top])
        onCursorMove({ x, y, z: 0 }) // z stays 0 on the flat map until Phase 2 DEM (T-050)
      })
    },
    [onCursorMove, view],
  )

  // Drive both the gesture machine and the cursor read-out from one container pointermove.
  const onPointerMove = useCallback(
    (e: React.PointerEvent) => {
      selectTool.onPointerMove(e)
      emitCursor(e)
    },
    [selectTool, emitCursor],
  )

  // Off-map (pointer over a docked panel or out of the canvas) → blank the read-out.
  const onPointerLeave = useCallback(() => {
    if (cursorRaf.current) {
      cancelAnimationFrame(cursorRaf.current)
      cursorRaf.current = 0
    }
    onCursorMove?.(null)
  }, [onCursorMove])

  // Cancel any pending cursor frame on unmount.
  useEffect(() => () => {
    if (cursorRaf.current) cancelAnimationFrame(cursorRaf.current)
  }, [])

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
        onPointerMove={onPointerMove}
        onPointerUp={selectTool.onPointerUp}
        onPointerLeave={onPointerLeave}
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
          layers={[
            ...(showGrid ? [baseMap] : []),
            ...clusterLayers,
            iconLayer,
            dragIconLayer,
            selectionLayer,
          ]}
          // No onClick / onHover: click-select and picking moved to the slotSpatialIndex R-tree
          // (T-063), cursor coords come from our own rAF unproject (emitCursor). getCursor is
          // constant so Deck never computes isHovering (which would force GPU hover picking).
          getCursor={() => 'crosshair'}
          style={{ position: 'absolute', width: '100%', height: '100%' }}
        />
      </div>
    </MapContextProvider>
  )
}

// Memoized so a host re-render (modal open/close, dirty flag, cursor read-out) doesn't
// re-render the map subtree — its props (terrain + stable callbacks) rarely change (T-057).
export const TacticalMap = memo(TacticalMapInner)
