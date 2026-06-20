// Full-bleed route shell for the 2D Mission Creator — the Eden docked shell (Phase 3.5).
// The live map is full-bleed behind a frosted overlay; the Top Command Strip spans the top
// and the Left Sidebar (w-64) + Right Asset Palette (w-80) dock flush to the edges, with the
// map between them. The route carries `chromeless` so AppLayout hides the platform nav and
// gives the editor the whole viewport.

import { useCallback, useEffect, useRef, useState } from 'react'
import { useParams } from 'react-router-dom'
import { TacticalMap, addSlot, moveEntity, useMapStore } from '@/features/tactical-map'
import type { AssetDropPayload, TacticalMapApi } from '@/features/tactical-map'
import { useMissionDoc } from './hooks/useMissionDoc'
import { TopCommandStrip } from './layout/TopCommandStrip'
import { BottomToolbelt } from './layout/BottomToolbelt'
import { LeftSidebar } from './layout/LeftOutliner/LeftSidebar'
import { AssetPalette } from './layout/RightInspector/AssetPalette'
import { AttributesModal } from './layout/AttributesModal'
import { FpsCounter } from './FpsCounter'

export default function MissionCreatorPage() {
  const { id } = useParams<{ id: string }>()
  const { md, undo } = useMissionDoc(id)

  const [cursor, setCursor] = useState<{ x: number; y: number } | null>(null)
  const [attributesId, setAttributesId] = useState<string | null>(null)

  // The map's imperative API (flyTo) — captured once for Spacebar centering.
  const mapApi = useRef<TacticalMapApi | null>(null)
  const onReady = useCallback((api: TacticalMapApi) => {
    mapApi.current = api
  }, [])

  // Click empty map with a slot selected → reposition it. (Click-drag-to-move replaces
  // this in Phase 7b; placement is drag-and-drop from the Asset Palette.)
  const onMapClick = useCallback(
    (world: { x: number; y: number }) => {
      const { selection } = useMapStore.getState()
      if (selection.kind === 'slot' && selection.id) {
        moveEntity(md, 'slots', selection.id, world)
      }
    },
    [md],
  )

  // Drop an Asset Palette leaf onto the map → one Y.Doc transaction creates the slot
  // (Arma defaults, Z=0 until the DEM lands) under the active Outliner folder, then
  // selects it so the Attributes modal / trees reflect the new entity.
  const onAssetDrop = useCallback(
    (payload: AssetDropPayload, world: { x: number; y: number }) => {
      if (payload.kind !== 'slot') return
      const layerId = useMapStore.getState().activeLayerId ?? undefined
      const newId = addSlot(md, world, { role: payload.role, layerId })
      useMapStore.getState().setSelection({ kind: 'slot', id: newId })
    },
    [md],
  )

  // Spacebar centers the camera on the current selection (no auto-fly on click —
  // Decisions log). Centroid of a multi-selection is Phase 7b; single slot for now.
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.code !== 'Space') return
      const target = e.target as HTMLElement | null
      if (target && (target.tagName === 'INPUT' || target.tagName === 'SELECT' || target.isContentEditable)) {
        return
      }
      const { selection, slotsById } = useMapStore.getState()
      if (selection.kind === 'slot' && selection.id) {
        const slot = slotsById[selection.id]
        if (slot) {
          e.preventDefault()
          mapApi.current?.flyTo(slot.position)
        }
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [])

  return (
    <div className="relative h-full w-full overflow-hidden bg-background">
      {/* Full-bleed map behind everything. */}
      <TacticalMap
        terrain="everon"
        showGrid={false}
        className="absolute inset-0 z-0 bg-background"
        onReady={onReady}
        onMapClick={onMapClick}
        onCursorMove={setCursor}
        onEntityActivate={setAttributesId}
        onAssetDrop={onAssetDrop}
      />

      {/* Overlay layer: spans the screen and ignores pointer events so the map gap pans;
          each docked panel re-enables hits via the `overlayDocked` recipe. */}
      <div className="pointer-events-none absolute inset-0 z-10">
        <div className="absolute inset-x-0 top-0 h-12">
          <TopCommandStrip md={md} undo={undo} />
        </div>

        <div className="absolute bottom-0 left-0 top-12 w-64">
          <LeftSidebar md={md} onActivateSlot={setAttributesId} />
        </div>

        <div className="absolute bottom-0 right-0 top-12 w-80">
          <AssetPalette />
        </div>

        <div className="absolute bottom-5 left-1/2 -translate-x-1/2">
          <BottomToolbelt cursorWorld={cursor} />
        </div>

        <FpsCounter />
      </div>

      <AttributesModal
        md={md}
        slotId={attributesId}
        onOpenChange={(open) => !open && setAttributesId(null)}
      />
    </div>
  )
}
