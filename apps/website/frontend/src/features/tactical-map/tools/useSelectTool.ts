// Pointer gesture state machine for the Select tool (Phase 7b). We own every pointer gesture
// here so the Eden interaction contract holds: left-drag an icon = move, left-drag empty =
// marquee box-select, middle/right-drag = pan, and a sub-threshold left release = click
// (select / toggle / deselect). All picking goes through the slotSpatialIndex R-tree (T-063) —
// Deck's `slot-icons` layer is no longer pickable, so click-select moved out of Deck's onClick
// into the pending-left pointerUp branch below. Mutations are reported to the host via
// onEntitiesMove (one Y.Doc transaction = one undo step); selection + transient drag/marquee
// live in the store. The map never pans on left-drag (Deck's dragPan is disabled in TacticalMap).

import { useCallback, useEffect, useRef } from 'react'
import type { OrthographicView } from '@deck.gl/core'
import { useMapStore } from '../state/useMapStore'
import * as slotIconCache from '../state/slotIconCache'
import * as slotSpatialIndex from '../state/slotSpatialIndex'
import * as slotClusterIndex from '../state/slotClusterIndex'
import type { ID } from '../state/schema'
import type { MapViewState } from '../types'

type Pt = [number, number]

/** ~4 px of motion separates a click (Deck handles it) from a drag (we handle it). */
const DRAG_THRESHOLD = 4

interface Viewport {
  unproject: (xy: number[]) => number[]
}

type Gesture =
  | { type: 'pending-left'; startPx: Pt; iconId: ID | null }
  | { type: 'move'; ids: ID[]; startWorld: Pt; vp: Viewport }
  | { type: 'marquee'; startPx: Pt; startWorld: Pt; vp: Viewport }
  | { type: 'pan'; startTarget: Pt; startPx: Pt; vp: Viewport }
  | null

interface UseSelectToolArgs {
  containerRef: React.RefObject<HTMLDivElement | null>
  view: OrthographicView
  viewState: MapViewState
  onViewStateChange: (params: { viewState: MapViewState }) => void
  /** Commit a group move (delta in world meters) — host wraps it in one transaction. */
  onEntitiesMove: (ids: ID[], delta: { x: number; y: number }) => void
  /** T-065: when zoomed out on a large mission, a left click drills into the cluster under the
   *  cursor instead of picking an individual (hidden) slot. */
  clusterMode: boolean
  /** Drill-in handler — recenter on the cluster centroid and zoom in (host owns the camera). */
  onClusterDrill: (centroid: { x: number; y: number }) => void
}

export interface SelectToolHandlers {
  onPointerDown: (e: React.PointerEvent) => void
  onPointerMove: (e: React.PointerEvent) => void
  onPointerUp: (e: React.PointerEvent) => void
  onContextMenu: (e: React.MouseEvent) => void
}

export function useSelectTool({
  containerRef,
  view,
  viewState,
  onViewStateChange,
  onEntitiesMove,
  clusterMode,
  onClusterDrill,
}: UseSelectToolArgs): SelectToolHandlers {
  const gesture = useRef<Gesture>(null)
  // Pan is rAF-coalesced (T-057): a high-rate mouse can fire several pointermove events per
  // frame, and each onViewStateChange re-renders TacticalMap. We stash the latest target and
  // flush at most once per animation frame.
  const panRaf = useRef(0)
  const pendingPan = useRef<MapViewState | null>(null)
  const flushPan = useCallback(() => {
    panRaf.current = 0
    const next = pendingPan.current
    pendingPan.current = null
    if (next) onViewStateChange({ viewState: next })
  }, [onViewStateChange])
  const cancelPan = useCallback(() => {
    if (panRaf.current) {
      cancelAnimationFrame(panRaf.current)
      panRaf.current = 0
    }
    pendingPan.current = null
  }, [])

  // Drag-move is rAF-coalesced the same way (T-061): at 360k slots, pushing a fresh delta
  // to the store on every pointermove re-rendered the drag overlay several times per frame.
  // We stash the latest world delta and flush at most once per animation frame; the ref is
  // kept (not nulled on flush) so pointer-up can read the final delta to commit the move.
  const dragRaf = useRef(0)
  const lastDragDelta = useRef<{ dx: number; dy: number } | null>(null)
  const flushDragDelta = useCallback(() => {
    dragRaf.current = 0
    if (lastDragDelta.current) {
      useMapStore.getState().setDragPreviewDelta(lastDragDelta.current)
    }
  }, [])
  const cancelDrag = useCallback(() => {
    if (dragRaf.current) {
      cancelAnimationFrame(dragRaf.current)
      dragRaf.current = 0
    }
    lastDragDelta.current = null
  }, [])

  const localPt = useCallback(
    (e: { clientX: number; clientY: number }): Pt | null => {
      const el = containerRef.current
      if (!el) return null
      const r = el.getBoundingClientRect()
      return [e.clientX - r.left, e.clientY - r.top]
    },
    [containerRef],
  )

  const makeViewport = useCallback((): Viewport | null => {
    const el = containerRef.current
    if (!el) return null
    const r = el.getBoundingClientRect()
    return view.makeViewport({ width: r.width, height: r.height, viewState }) as Viewport | null
  }, [containerRef, view, viewState])

  const onPointerDown = useCallback(
    (e: React.PointerEvent) => {
      const px = localPt(e)
      if (!px) return

      // Middle / right button → pan against a viewport frozen at gesture start. Capture
      // now: these never produce a Deck click, so redirecting events is safe.
      if (e.button === 1 || e.button === 2) {
        const vp = makeViewport()
        if (!vp) return
        e.preventDefault()
        containerRef.current?.setPointerCapture(e.pointerId)
        gesture.current = {
          type: 'pan',
          startTarget: [viewState.target[0], viewState.target[1]],
          startPx: px,
          vp,
        }
        return
      }

      if (e.button !== 0) return
      // Left button: defer the decision (click vs move vs marquee) to the threshold. Do NOT
      // capture yet — a sub-threshold release is a click, handled in onPointerUp. Pick via the
      // spatial index (T-063) instead of Deck — the layer is no longer pickable.
      // Cluster mode (T-065.1): never pickNearest the full rbush here — a press near a bubble
      // would grab a slot hidden under it and start a move. Leave iconId null so only marquee /
      // drill-in act until the user zooms into detail.
      const vp = makeViewport()
      const iconId = !clusterMode && vp ? slotSpatialIndex.pickNearest(px, vp) : null
      gesture.current = { type: 'pending-left', startPx: px, iconId }
    },
    [localPt, makeViewport, containerRef, viewState, clusterMode],
  )

  const onPointerMove = useCallback(
    // eslint-disable-next-line complexity -- pointer-move gesture machine: pan/marquee/drag-preview branches, rAF-coalesced; perf hot path
    (e: React.PointerEvent) => {
      const g = gesture.current
      if (!g) return
      const px = localPt(e)
      if (!px) return

      if (g.type === 'pan') {
        const w0 = g.vp.unproject(g.startPx)
        const w1 = g.vp.unproject(px)
        pendingPan.current = {
          ...viewState,
          target: [g.startTarget[0] - (w1[0] - w0[0]), g.startTarget[1] - (w1[1] - w0[1])],
        }
        if (!panRaf.current) panRaf.current = requestAnimationFrame(flushPan)
        return
      }

      // Promote a pending left-press into a move or marquee once it clears the threshold.
      if (g.type === 'pending-left') {
        if (Math.hypot(px[0] - g.startPx[0], px[1] - g.startPx[1]) < DRAG_THRESHOLD) return
        const vp = makeViewport()
        if (!vp) return
        // Now that it's a real drag, capture so it survives leaving the canvas; Deck won't
        // see a matching pointerup, so no stray click fires after the drag.
        containerRef.current?.setPointerCapture(e.pointerId)
        const startWorld = vp.unproject(g.startPx) as Pt
        if (g.iconId) {
          const sel = useMapStore.getState().selection
          const ids =
            sel.kind === 'slot' && sel.ids.includes(g.iconId) ? sel.ids : [g.iconId]
          if (!(sel.kind === 'slot' && sel.ids.includes(g.iconId))) {
            useMapStore.getState().setSelection({ kind: 'slot', ids })
          }
          // Promote the dragged ids to the overlay layer once; the live offset is fed
          // rAF-coalesced below so the base layer (all other icons) never rebuilds (T-061).
          // Lift the dragged ids out of the base icon cache O(k) (T-061.0.1) so the pickup
          // doesn't re-derive ~360k icons — they render in the drag overlay instead.
          slotIconCache.exclude(ids)
          lastDragDelta.current = { dx: 0, dy: 0 }
          useMapStore.getState().setDragPreview(ids)
          useMapStore.getState().setDragPreviewDelta({ dx: 0, dy: 0 })
          useMapStore.getState()._syncIconCache()
          gesture.current = { type: 'move', ids, startWorld, vp }
        } else {
          gesture.current = { type: 'marquee', startPx: g.startPx, startWorld, vp }
        }
      }

      const cur = gesture.current
      if (cur?.type === 'move') {
        const w1 = cur.vp.unproject(px)
        lastDragDelta.current = { dx: w1[0] - cur.startWorld[0], dy: w1[1] - cur.startWorld[1] }
        if (!dragRaf.current) dragRaf.current = requestAnimationFrame(flushDragDelta)
      } else if (cur?.type === 'marquee') {
        const w1 = cur.vp.unproject(px)
        useMapStore
          .getState()
          .setMarquee({ x0: cur.startWorld[0], y0: cur.startWorld[1], x1: w1[0], y1: w1[1] })
      }
    },
    [localPt, makeViewport, viewState, containerRef, flushPan, flushDragDelta],
  )

  const onPointerUp = useCallback(
    // eslint-disable-next-line complexity -- pointer-up: commit drag-move/marquee/pan + selection-update branches
    (e: React.PointerEvent) => {
      const g = gesture.current
      gesture.current = null
      if (containerRef.current?.hasPointerCapture(e.pointerId)) {
        containerRef.current.releasePointerCapture(e.pointerId)
      }
      if (!g) return

      if (g.type === 'pan') {
        // Apply the last coalesced target immediately so the camera lands where the pointer
        // was released (a pending rAF may not have fired yet).
        if (panRaf.current) {
          cancelAnimationFrame(panRaf.current)
          panRaf.current = 0
        }
        flushPan()
        return
      }

      const store = useMapStore.getState()

      if (g.type === 'move') {
        // Read the final delta from the ref (a pending rAF may not have flushed it yet),
        // commit one move transaction if it actually moved, then clear the preview.
        if (dragRaf.current) {
          cancelAnimationFrame(dragRaf.current)
          dragRaf.current = 0
        }
        // Optimistically patch the cached icons O(k) (relative), commit the Y.Doc move,
        // then restore the dragged ids into the base cache at their new spot — so the base
        // layer shows the move with no flicker before the async bindings flush lands (which
        // re-sets the same positions absolutely, idempotently) (T-061.0.1).
        const delta = lastDragDelta.current
        if (delta && (delta.dx !== 0 || delta.dy !== 0)) {
          slotIconCache.patchPositions(g.ids, delta)
          onEntitiesMove(g.ids, { x: delta.dx, y: delta.dy })
        }
        slotIconCache.restore(g.ids)
        lastDragDelta.current = null
        store.clearDragPreview()
        store._syncIconCache()
        return
      }

      if (g.type === 'marquee') {
        const px = localPt(e)
        if (px) {
          const width = Math.abs(px[0] - g.startPx[0])
          const height = Math.abs(px[1] - g.startPx[1])
          if (width >= 1 && height >= 1) {
            // Query the spatial index in WORLD meters (T-063) — start corner + release point,
            // both already unprojected through the gesture's frozen viewport.
            const w1 = g.vp.unproject(px)
            const ids = slotSpatialIndex.pickRect(g.startWorld[0], g.startWorld[1], w1[0], w1[1])
            store.setSelection(ids.length ? { kind: 'slot', ids } : { kind: 'none', ids: [] })
          }
        }
        store.setMarquee(null)
        return
      }

      // 'pending-left' (sub-threshold) = a click. Deck no longer picks on click (T-063), so
      // selection lives here: pick the slot under the release point and apply T-053 rules.
      if (g.type === 'pending-left') {
        const px = localPt(e)
        const vp = px ? makeViewport() : null
        const additive = e.ctrlKey || e.metaKey
        // Cluster mode (T-065): NEVER pickNearest the full rbush — that would select an
        // individual slot hidden under a cluster bubble. A cluster hit drills in (no selection
        // change); otherwise treat it as an empty click (plain = deselect, additive = preserve).
        if (clusterMode) {
          const marker = px && vp ? slotClusterIndex.pickClusterAt(px, vp, viewState.zoom) : null
          if (marker) {
            onClusterDrill({ x: marker.x, y: marker.y })
          } else if (!additive) {
            store.setSelection({ kind: 'none', ids: [] })
          }
          return
        }
        const id = px && vp ? slotSpatialIndex.pickNearest(px, vp) : null
        if (id) {
          if (additive) {
            // Ctrl/Cmd-click toggles this slot in/out of the current selection.
            const cur = store.selection.kind === 'slot' ? store.selection.ids : []
            const next = cur.includes(id) ? cur.filter((x) => x !== id) : [...cur, id]
            store.setSelection(next.length ? { kind: 'slot', ids: next } : { kind: 'none', ids: [] })
          } else {
            store.setSelection({ kind: 'slot', ids: [id] })
          }
        } else if (!additive) {
          // Plain empty click deselects; Ctrl/Cmd + empty preserves the selection (T-053).
          store.setSelection({ kind: 'none', ids: [] })
        }
      }
    },
    [
      containerRef,
      localPt,
      makeViewport,
      onEntitiesMove,
      flushPan,
      clusterMode,
      onClusterDrill,
      viewState,
    ],
  )

  // Drop any in-flight pan/drag frame if the tool unmounts mid-gesture.
  useEffect(
    () => () => {
      cancelPan()
      cancelDrag()
    },
    [cancelPan, cancelDrag],
  )

  const onContextMenu = useCallback((e: React.MouseEvent) => e.preventDefault(), [])

  return { onPointerDown, onPointerMove, onPointerUp, onContextMenu }
}
