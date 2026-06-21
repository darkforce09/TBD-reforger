// Full-bleed route shell for the 2D Mission Creator — the Eden docked shell (Phase 3.5).
// The live map is full-bleed behind a frosted overlay; the Top Command Strip spans the top
// and the Left Sidebar (w-64) + Right Asset Palette (w-80) dock flush to the edges, with the
// map between them. The route carries `chromeless` so AppLayout hides the platform nav and
// gives the editor the whole viewport.

import { useCallback, useEffect, useRef, useState } from 'react'
import { useParams } from 'react-router-dom'
import { TacticalMap, addSlot, moveEntities, removeEntities, useMapStore } from '@/features/tactical-map'
import type { AssetDropPayload, TacticalMapApi } from '@/features/tactical-map'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import { useMissionEditor } from './hooks/useMissionEditor'
import { TopCommandStrip } from './layout/TopCommandStrip'
import { BottomToolbelt } from './layout/BottomToolbelt'
import { LeftSidebar } from './layout/LeftOutliner/LeftSidebar'
import { AssetPalette } from './layout/RightInspector/AssetPalette'
import { AttributesModal } from './layout/AttributesModal'
import { FpsCounter } from './FpsCounter'

export default function MissionCreatorPage() {
  const { id } = useParams<{ id: string }>()
  const editor = useMissionEditor(id)
  const { md, undo } = editor

  const [cursor, setCursor] = useState<{ x: number; y: number } | null>(null)
  const [attributesId, setAttributesId] = useState<string | null>(null)

  // The map's imperative API (flyTo) — captured once for Spacebar centering.
  const mapApi = useRef<TacticalMapApi | null>(null)
  const onReady = useCallback((api: TacticalMapApi) => {
    mapApi.current = api
  }, [])

  // Drag-move release → commit the group move in one Y.Doc transaction (one undo step).
  const onEntitiesMove = useCallback(
    (ids: string[], delta: { x: number; y: number }) => moveEntities(md, ids, delta),
    [md],
  )

  // Double-click opens the Attributes modal only for a single-entity selection.
  const onEntityActivate = useCallback((id: string) => {
    if (useMapStore.getState().selection.ids.length <= 1) setAttributesId(id)
  }, [])

  // Drop an Asset Palette leaf onto the map → one Y.Doc transaction creates the slot
  // (Arma defaults, Z=0 until the DEM lands) under the active Outliner folder, then
  // selects it so the Attributes modal / trees reflect the new entity.
  const onAssetDrop = useCallback(
    (payload: AssetDropPayload, world: { x: number; y: number }) => {
      if (payload.kind !== 'slot') return
      const layerId = useMapStore.getState().activeLayerId ?? undefined
      const newId = addSlot(md, world, { role: payload.role, layerId, assetId: payload.assetId })
      useMapStore.getState().setSelection({ kind: 'slot', ids: [newId] })
    },
    [md],
  )

  // Keyboard: Spacebar centers on the selection centroid (no auto-fly on click —
  // Decisions log); Delete/Backspace removes the selection in one undoable step.
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null
      if (target && (target.tagName === 'INPUT' || target.tagName === 'SELECT' || target.isContentEditable)) {
        return
      }
      const { selection, slotsById, setSelection } = useMapStore.getState()
      if (e.code === 'Space') {
        if (selection.kind === 'none') return
        const slots = selection.ids.map((sid) => slotsById[sid]).filter(Boolean) as { position: { x: number; y: number } }[]
        if (!slots.length) return
        e.preventDefault()
        const cx = slots.reduce((a, s) => a + s.position.x, 0) / slots.length
        const cy = slots.reduce((a, s) => a + s.position.y, 0) / slots.length
        mapApi.current?.flyTo({ x: cx, y: cy })
      } else if (e.code === 'Delete' || e.code === 'Backspace') {
        if (selection.kind === 'none' || !selection.ids.length) return
        e.preventDefault()
        removeEntities(md, 'slots', selection.ids)
        setSelection({ kind: 'none', ids: [] })
      }
    }
    window.addEventListener('keydown', onKeyDown)
    return () => window.removeEventListener('keydown', onKeyDown)
  }, [md])

  return (
    <div className="relative h-full w-full overflow-hidden bg-background">
      {/* Full-bleed map behind everything. */}
      <TacticalMap
        terrain="everon"
        showGrid
        className="absolute inset-0 z-0 bg-background"
        onReady={onReady}
        onCursorMove={setCursor}
        onEntityActivate={onEntityActivate}
        onAssetDrop={onAssetDrop}
        onEntitiesMove={onEntitiesMove}
      />

      {/* Overlay layer: spans the screen and ignores pointer events so the map gap pans;
          each docked panel re-enables hits via the `overlayDocked` recipe. */}
      <div className="pointer-events-none absolute inset-0 z-10">
        <div className="absolute inset-x-0 top-0 h-12">
          <TopCommandStrip
            md={md}
            undo={undo}
            dirty={editor.dirty}
            suggestedSemver={editor.suggestedSemver}
            onExport={editor.exportJson}
            onSaveVersion={editor.saveVersion}
          />
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

        {editor.invalidMissionId && (
          <div className="absolute left-1/2 top-14 -translate-x-1/2">
            <p className="pointer-events-auto rounded-md border border-tactical-yellow/40 bg-tactical-yellow/15 px-3 py-1.5 text-label-sm normal-case text-tactical-yellow">
              This editor URL needs a real mission id — create one from Mission Library first.
            </p>
          </div>
        )}

        <FpsCounter />
      </div>

      <AttributesModal
        md={md}
        slotId={attributesId}
        onOpenChange={(open) => !open && setAttributesId(null)}
      />

      {/* Load conflict: the server has a saved version and the local draft has edits. */}
      <Dialog open={editor.conflict != null} onOpenChange={() => {}}>
        <DialogContent
          title="Unsaved local changes"
          description="This mission has a saved version on the server, and your local draft has changes. Which do you want to keep?"
        >
          <div className="flex justify-end gap-2">
            <button
              type="button"
              onClick={() => editor.resolveConflict('server')}
              className="rounded-md border border-outline-variant/40 px-3 py-1.5 text-label-md text-on-surface-variant transition-colors hover:bg-white/5"
            >
              Load saved version
            </button>
            <button
              type="button"
              onClick={() => editor.resolveConflict('local')}
              className="rounded-md bg-primary/20 px-3 py-1.5 text-label-md text-primary transition-colors hover:bg-primary/30"
            >
              Keep local draft
            </button>
          </div>
        </DialogContent>
      </Dialog>
    </div>
  )
}
