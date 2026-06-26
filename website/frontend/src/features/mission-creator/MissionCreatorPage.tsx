// Full-bleed route shell for the 2D Mission Creator — the Eden docked shell (Phase 3.5).
// The live map is full-bleed behind a frosted overlay; the Top Command Strip spans the top
// and the Left Sidebar (w-64) + Right Asset Palette (w-80) dock flush to the edges, with the
// map between them. The route carries `chromeless` so AppLayout hides the platform nav and
// gives the editor the whole viewport.

import { useCallback, useEffect, useRef, useState } from 'react'
import { useParams } from 'react-router-dom'
import { TacticalMap, addSlot, moveEntities, pasteSlots, removeEntities, useMapStore } from '@/features/tactical-map'
import type { AssetDropPayload, ClipboardSlot, TacticalMapApi } from '@/features/tactical-map'
import { Dialog, DialogContent } from '@/components/ui/dialog'
import { useMissionEditor } from './hooks/useMissionEditor'
import { TopCommandStrip } from './layout/TopCommandStrip'
import { BottomToolbelt } from './layout/BottomToolbelt'
import { LeftSidebar } from './layout/LeftOutliner/LeftSidebar'
import { AssetPalette } from './layout/RightInspector/AssetPalette'
import { AttributesModal } from './layout/AttributesModal'
import { FpsCounter } from './FpsCounter'

/** A paste larger than this does not auto-select its result — see the Ctrl+V branch (T-059). */
const BULK_SELECT_CAP = 500

export default function MissionCreatorPage() {
  const { id } = useParams<{ id: string }>()
  const editor = useMissionEditor(id)
  const { md, undo } = editor

  const [attributesId, setAttributesId] = useState<string | null>(null)

  // In-editor clipboard for Ctrl+C/V copy-paste (T-056); a ref so a copy/paste never
  // re-renders the page. The live cursor lives in the engine store (set rAF-throttled by
  // TacticalMap, T-057) so the page never re-renders on pointer move — paste reads it via
  // useMapStore.getState().cursor and only BottomToolbelt subscribes to it.
  const clipboardRef = useRef<ClipboardSlot[]>([])

  // Terrain comes from the hydrated mission meta (Everon 12.8km vs Arland 10.24km); the
  // `key` remounts the viewport so the camera/base-map resize to the new bounds.
  const terrainId = useMapStore((s) => s.meta?.terrain ?? 'everon')

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

  // Cursor read-out goes straight into the engine store (rAF-throttled upstream) so the page
  // never re-renders on pointer move — only BottomToolbelt subscribes to it (T-057).
  const onCursorMove = useCallback(
    (c: { x: number; y: number; z: number } | null) => useMapStore.getState().setCursor(c),
    [],
  )

  // Stable so the memoized AttributesModal doesn't re-render on unrelated page renders.
  const onAttributesOpenChange = useCallback((open: boolean) => {
    if (!open) setAttributesId(null)
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

  // Keyboard: Cmd/Ctrl+Z/Y undo-redo (reuses the existing UndoController); Spacebar centers
  // on the selection centroid (no auto-fly on click — Decisions log); Delete/Backspace removes
  // the selection in one undoable step. All skipped while a form field is focused.
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      const target = e.target as HTMLElement | null
      if (
        target &&
        (target.tagName === 'INPUT' ||
          target.tagName === 'SELECT' ||
          target.tagName === 'TEXTAREA' ||
          target.isContentEditable)
      ) {
        return
      }
      // Undo/redo: Cmd/Ctrl+Z = undo, Cmd/Ctrl+Shift+Z or Ctrl+Y = redo. preventDefault on a
      // match (stops the browser's "undo typing" from bleeding into page chrome) but only
      // drives the stack when there's something to undo/redo.
      const mod = e.metaKey || e.ctrlKey
      if (mod && !e.altKey && e.code === 'KeyZ' && !e.shiftKey) {
        e.preventDefault()
        if (undo.canUndo()) undo.undo()
        return
      }
      if (mod && !e.altKey && ((e.code === 'KeyZ' && e.shiftKey) || e.code === 'KeyY')) {
        e.preventDefault()
        if (undo.canRedo()) undo.redo()
        return
      }
      // Copy/paste (T-056): Cmd/Ctrl+C snapshots the slot selection to the in-editor
      // clipboard; Cmd/Ctrl+V pastes at the cursor (relative layout preserved). Both no-op
      // (no preventDefault) when they can't act, so native text copy/paste is unaffected.
      if (mod && !e.altKey && !e.shiftKey && e.code === 'KeyC') {
        const { selection, slotsById } = useMapStore.getState()
        if (selection.kind !== 'slot' || !selection.ids.length) return
        const clip: ClipboardSlot[] = selection.ids
          .map((sid) => slotsById[sid])
          .filter((s): s is NonNullable<typeof s> => Boolean(s))
          .map((s) => ({
            role: s.role,
            ...(s.tag ? { tag: s.tag } : {}),
            ...(s.assetId ? { assetId: s.assetId } : {}),
            stance: s.stance,
            position: { ...s.position },
            squadId: s.squadId,
          }))
        if (!clip.length) return
        clipboardRef.current = clip
        e.preventDefault()
        return
      }
      if (mod && !e.altKey && !e.shiftKey && e.code === 'KeyV') {
        if (!clipboardRef.current.length) return
        e.preventDefault()
        const { activeLayerId, cursor: cur, setSelection: setSel } = useMapStore.getState()
        const ids = pasteSlots(md, clipboardRef.current, {
          anchorAt: cur ? { x: cur.x, y: cur.y } : null,
          layerId: activeLayerId ?? undefined,
        })
        // Cap the post-paste selection: putting 10k ids in selection.ids blows up the
        // highlight Set + outliner re-render (T-059). Above the cap, clear selection — OBJ
        // still updates (it reads slotsById); the user box-selects if they need it.
        if (ids.length) {
          setSel(ids.length <= BULK_SELECT_CAP ? { kind: 'slot', ids } : { kind: 'none', ids: [] })
        }
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
  }, [md, undo])

  return (
    <div className="relative h-full w-full overflow-hidden bg-background">
      {/* Full-bleed map behind everything. */}
      <TacticalMap
        key={terrainId}
        terrain={terrainId}
        showGrid
        className="absolute inset-0 z-0 bg-background"
        onReady={onReady}
        onCursorMove={onCursorMove}
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

        {/* Outliner builds a tree over every slot — defer its mount until the doc is
            ready so a 10k+ load doesn't pay for it mid-boot (T-060). */}
        {editor.docStatus === 'ready' && (
          <div className="absolute bottom-0 left-0 top-12 w-64">
            <LeftSidebar md={md} onActivateSlot={setAttributesId} />
          </div>
        )}

        <div className="absolute bottom-0 right-0 top-12 w-80">
          <AssetPalette />
        </div>

        <div className="absolute bottom-5 left-1/2 -translate-x-1/2">
          <BottomToolbelt />
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

      {/* Load gate: covers the shell until the local snapshot (and any server hydrate) is in
          the store. Determinate bar (T-060.1) driven by loadProgress — download → apply →
          reflect phases, each chunked so the % advances at scale. */}
      {editor.docStatus === 'loading' && (
        <div className="pointer-events-auto absolute inset-0 z-30 flex flex-col items-center justify-center gap-3 bg-background/80 backdrop-blur-sm">
          <p className="text-label-md text-on-surface-variant">{editor.loadProgress.label}</p>
          {(() => {
            const lp = editor.loadProgress
            // IndexedDB replay has no total → indeterminate sweep + count-only readout, so
            // the bar shows motion instead of a frozen 0%. Every other phase is determinate.
            const restoring = lp.phase === 'restoring' && lp.total == null
            return (
              <>
                <div className="h-1 w-56 overflow-hidden rounded-full bg-white/10">
                  {restoring ? (
                    <div className="animate-mc-load-bar h-full w-1/3 rounded-full bg-primary" />
                  ) : (
                    <div
                      className="h-full rounded-full bg-primary transition-[width] duration-150"
                      style={{ width: `${Math.round(lp.value * 100)}%` }}
                    />
                  )}
                </div>
                <p className="font-mono text-code-md tabular-nums text-outline">
                  {restoring
                    ? `${(lp.done ?? 0).toLocaleString()} objects restored`
                    : lp.done != null && lp.total
                      ? `${lp.done.toLocaleString()} / ${lp.total.toLocaleString()} objects`
                      : `${Math.round(lp.value * 100)}%`}
                </p>
              </>
            )
          })()}
        </div>
      )}

      <AttributesModal
        md={md}
        slotId={attributesId}
        onOpenChange={onAttributesOpenChange}
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
