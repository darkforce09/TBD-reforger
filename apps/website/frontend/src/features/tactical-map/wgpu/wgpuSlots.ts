// T-151.6 W6 — mission slot / selection / drag / cluster GPU bridge for WgpuTacticalMap.
// Positions from MissionDoc SoA (refresh → slot_xy_ptr / slot_len); never slotsById as SoT.
// T-151.7.1: selection tint (cluster short-lane), drag start-vs-delta, no per-frame buffer recreate.

import * as wasmBg from '@/wasm/pkg/map_engine_wasm_bg.wasm'
import type { WasmMissionDoc } from '../state/wasmDoc'
import { useMapStore } from '../state/useMapStore'
import {
  CLUSTER_SLOT_THRESHOLD,
  ZOOM_CLUSTER_MAX,
} from '../state/constants'
import {
  getClusterMarkers,
  getClusterMarkersVersion,
} from '../state/slotClusterIndex'
import type { RenderEngine } from './wasmRender'
import {
  buildSlotAtlas,
  packClusterInstances,
  packSlotInstances,
  pxToMAtZoom,
  SLOT_ICON_STRIDE,
  SLOT_SELECTED_PX,
  SLOT_SELECTED_RGBA,
  packRgbaU32,
  packIconInstance,
} from './slotAtlas'

export function clusterMode(slotLen: number, deckZoom: number): boolean {
  return slotLen > CLUSTER_SLOT_THRESHOLD && deckZoom <= ZOOM_CLUSTER_MAX
}

/**
 * Pure helper (T-151.7.1): classify a drag store transition for the GPU bridge.
 * - `start` / `restart` → one overlay upload; `delta` → set_slot_drag_delta only; `end` → clear.
 */
export type DragGpuPhase = 'idle' | 'start' | 'delta' | 'restart' | 'end'

export function classifyDragTransition(
  prevIds: string[] | null | undefined,
  nextIds: string[] | null | undefined,
  idsChanged: boolean,
  deltaChanged: boolean,
): DragGpuPhase {
  const had = Boolean(prevIds?.length)
  const has = Boolean(nextIds?.length)
  if (!had && has) return 'start'
  if (had && !has) return 'end'
  if (had && has && idsChanged) return 'restart'
  if (had && has && deltaChanged) return 'delta'
  return 'idle'
}

/**
 * Controller: atlas once + doc subscribe → refresh → dirty/full upload; selection; T-061 drag;
 * T-065 clusters.
 */
export class WgpuSlotsController {
  private readonly engine: RenderEngine
  private disposed = false
  private atlasReady = false
  private md: WasmMissionDoc | null = null
  private unsubDoc: (() => void) | null = null
  private unsubStore: (() => void) | null = null
  /** Last SoA id order (join key for selection / drag). */
  private lastIds: string[] = []
  private lastLen = 0
  private lastClusterMode = false
  private lastClusterVersion = -1
  private lastZoomBucket = Number.NaN
  private dragActive = false
  private hiddenRows = new Set<number>()
  /**
   * True when the Slots GPU lane holds only the k selected instances (cluster detail=false path).
   * Full-doc row patches must not run against this lane (T-151.7.1 B1).
   */
  private slotsLaneIsSelectionOnly = false

  constructor(engine: RenderEngine) {
    this.engine = engine
  }

  async init(): Promise<void> {
    if (this.disposed || this.atlasReady) return
    const atlas = buildSlotAtlas()
    this.engine.upload_slot_atlas(atlas.rgba, atlas.width, atlas.height, atlas.uv)
    this.atlasReady = true
    this.syncZoomUniform()
    this.pushFromDoc(true)
    this.syncSelection()
    this.syncDragFromStore()
    this.syncClusters()
    this.publishDebug()
  }

  setMissionDoc(md: WasmMissionDoc | null): void {
    if (this.disposed) return
    this.unsubDoc?.()
    this.unsubDoc = null
    this.md = md
    if (!md) {
      this.engine.clear_slot_lanes()
      this.lastIds = []
      this.lastLen = 0
      this.slotsLaneIsSelectionOnly = false
      return
    }
    this.unsubDoc = md.subscribe(() => {
      this.pushFromDoc(false)
      this.syncClusters()
      this.publishDebug()
    })
    // Zustand selection / drag / zoom
    this.unsubStore?.()
    this.unsubStore = useMapStore.subscribe((s, prev) => {
      if (s.selection !== prev.selection) this.syncSelection()
      // T-151.7.1 B2: split id-change (upload once) vs delta-only (uniform only).
      const idsChanged = s.dragPreviewIds !== prev.dragPreviewIds
      const deltaChanged = s.dragPreviewDelta !== prev.dragPreviewDelta
      if (idsChanged || deltaChanged) {
        const phase = classifyDragTransition(
          prev.dragPreviewIds,
          s.dragPreviewIds,
          idsChanged,
          deltaChanged,
        )
        this.applyDragPhase(phase, s.dragPreviewIds, s.dragPreviewDelta)
      }
      if (s.deckZoom !== prev.deckZoom) {
        this.onCameraMoved()
      }
    })
    if (this.atlasReady) {
      this.pushFromDoc(true)
      this.syncSelection()
      this.syncDragFromStore()
      this.syncClusters()
    }
  }

  onCameraMoved(): void {
    if (this.disposed || !this.atlasReady) return
    this.syncZoomUniform()
    const zoom = this.engine.zoom
    useMapStore.getState().setDeckZoom(zoom)
    this.syncClusters()
    // Detail ↔ cluster mode flip may hide/show slot lane.
    const len = this.lastLen
    const cm = clusterMode(len, zoom)
    if (cm !== this.lastClusterMode) {
      this.lastClusterMode = cm
      this.applyClusterVisibility(cm)
    }
    this.publishDebug()
  }

  dispose(): void {
    this.disposed = true
    this.unsubDoc?.()
    this.unsubDoc = null
    this.unsubStore?.()
    this.unsubStore = null
    this.md = null
    try {
      this.engine.clear_slot_lanes()
    } catch {
      /* engine may already be freed */
    }
  }

  /** Full or structural re-upload from SoA after refresh. */
  private pushFromDoc(forceFull: boolean): void {
    if (this.disposed || !this.atlasReady) return
    const handle = this.md?.wasm
    if (!handle) {
      this.engine.clear_slot_lanes()
      this.lastIds = []
      this.lastLen = 0
      this.slotsLaneIsSelectionOnly = false
      return
    }
    handle.refresh()
    const n = handle.slot_len
    const ids = handle.slot_ids() as string[]
    const xy = new Float32Array(wasmBg.memory.buffer, handle.slot_xy_ptr, n * 2)
    // Copy out of wasm memory before any later growth detaches the view.
    const xyCopy = new Float32Array(xy)

    const sel = this.selectedMask(ids)
    const bytes = packSlotInstances(xyCopy, sel)

    // Structural: full upload. Same len + same id multiset with only moves still full-uploads
    // when forceFull; otherwise when len/ids change. Position-only patches when len stable and
    // id order matches (yrs order can reshuffle — fall back to full).
    const orderStable =
      !forceFull &&
      n === this.lastLen &&
      ids.length === this.lastIds.length &&
      ids.every((id, i) => id === this.lastIds[i])

    const zoom = this.engine.zoom
    const cm = clusterMode(n, zoom)
    this.lastClusterMode = cm

    if (cm) {
      // Cluster: short selection-only lane (never full-n under clusters).
      this.applySelectionOnlyVisible(ids, xyCopy, sel)
    } else if (!orderStable) {
      this.engine.upload_slot_lane(bytes, true)
      this.slotsLaneIsSelectionOnly = false
    } else {
      this.engine.upload_slot_lane(bytes, true)
      this.slotsLaneIsSelectionOnly = false
    }

    this.lastIds = ids
    this.lastLen = n
    // Drag exclude rows may be stale after structural change.
    if (this.dragActive) this.restartDragOverlay()
  }

  private selectedMask(ids: string[]): boolean[] {
    const sel = useMapStore.getState().selection
    if (sel.kind === 'none' || !sel.ids.length) return ids.map(() => false)
    const set = new Set(sel.ids)
    return ids.map((id) => set.has(id))
  }

  /**
   * T-151.7.2: GPU selection tint is always a pure function of the store.
   * Detail = full re-pack (no per-row patch); cluster = selection-only short lane.
   * Fixes sticky yellow rings when SEL 0 (patches skipped / OOB / dragActive stuck).
   */
  private syncSelection(): void {
    if (this.disposed || !this.atlasReady) return
    // Invariant: dragActive only while dragPreviewIds non-empty.
    const dragIds = useMapStore.getState().dragPreviewIds
    if (this.dragActive && !dragIds?.length) {
      this.clearDragOverlay()
      return // clearDragOverlay already re-syncs selection
    }
    if (this.dragActive) return // drag overlay owns tint for those ids

    // Not dragging — drop any stale hide-row bookkeeping.
    this.hiddenRows.clear()

    if (this.lastClusterMode) {
      this.reuploadSelectionOnlyFromDoc()
      return
    }
    this.repackAndUploadDetailSlots()
  }

  /** Detail mode: full-n pack from SoA + current selectedMask → upload (Class R with store). */
  private repackAndUploadDetailSlots(): void {
    const handle = this.md?.wasm
    if (!handle) {
      this.engine.upload_slot_lane(new Uint8Array(0), false)
      this.slotsLaneIsSelectionOnly = false
      this.lastIds = []
      this.lastLen = 0
      return
    }
    handle.refresh()
    const n = handle.slot_len
    const ids = handle.slot_ids() as string[]
    const xy = new Float32Array(wasmBg.memory.buffer, handle.slot_xy_ptr, n * 2)
    const xyCopy = new Float32Array(xy)
    const bytes = packSlotInstances(xyCopy, this.selectedMask(ids))
    this.engine.upload_slot_lane(bytes, n > 0)
    this.slotsLaneIsSelectionOnly = false
    this.lastIds = ids
    this.lastLen = n
  }

  /** Cluster / short-lane path: re-upload k selected instances (no index patch). */
  private reuploadSelectionOnlyFromDoc(): void {
    const handle = this.md?.wasm
    if (!handle) {
      this.engine.upload_slot_lane(new Uint8Array(0), false)
      this.slotsLaneIsSelectionOnly = true
      return
    }
    handle.refresh()
    const n = handle.slot_len
    const ids = handle.slot_ids() as string[]
    const xy = new Float32Array(wasmBg.memory.buffer, handle.slot_xy_ptr, n * 2)
    this.lastIds = ids
    this.lastLen = n
    this.applySelectionOnlyVisible(ids, new Float32Array(xy), this.selectedMask(ids))
  }

  private clearDragOverlay(): void {
    if (!this.dragActive && this.hiddenRows.size === 0) {
      this.engine.clear_slot_drag_lane()
      return
    }
    this.dragActive = false
    this.hiddenRows.clear()
    this.engine.clear_slot_drag_lane()
    // Restore base lane + selection tint from store (detail or cluster).
    this.syncSelection()
  }

  /** Pack drag overlay + hide base rows (alpha 0). Returns instance count. */
  private buildDragOverlay(
    dragIds: string[],
    xy: Float32Array,
    idToRow: Map<string, number>,
  ): { overlay: Uint8Array; count: number } {
    const overlay = new Uint8Array(dragIds.length * SLOT_ICON_STRIDE)
    let k = 0
    this.hiddenRows.clear()
    const yellow = packRgbaU32(SLOT_SELECTED_RGBA)
    for (const id of dragIds) {
      const row = idToRow.get(id)
      if (row === undefined) continue
      this.hiddenRows.add(row)
      const x = xy[row * 2] ?? 0
      const y = xy[row * 2 + 1] ?? 0
      packIconInstance(overlay, k * SLOT_ICON_STRIDE, x, y, SLOT_SELECTED_PX, 0, yellow)
      // Hide base row: alpha 0 tint (only safe on full-n detail lane).
      if (!this.slotsLaneIsSelectionOnly) {
        const hide = new Uint8Array(12)
        const dv = new DataView(hide.buffer)
        dv.setFloat32(0, SLOT_SELECTED_PX, true)
        dv.setInt16(4, 0, true)
        dv.setUint16(6, 0, true)
        dv.setUint32(8, 0, true)
        this.engine.patch_slot_lane(row * SLOT_ICON_STRIDE + 8, hide)
      }
      k++
    }
    return { overlay: overlay.subarray(0, k * SLOT_ICON_STRIDE), count: k }
  }

  private applyDragPhase(
    phase: DragGpuPhase,
    dragIds: string[] | null | undefined,
    delta: { dx: number; dy: number } | null | undefined,
  ): void {
    if (this.disposed || !this.atlasReady) return
    switch (phase) {
      case 'idle':
        return
      case 'end':
        this.clearDragOverlay()
        return
      case 'delta':
        // T-151.7.1 B2: per-frame = 16 B delta uniform only.
        if (this.dragActive) {
          this.engine.set_slot_drag_delta(delta?.dx ?? 0, delta?.dy ?? 0)
        }
        return
      case 'start':
      case 'restart':
        this.startDragOverlay(dragIds ?? [], delta)
        return
    }
  }

  private syncDragFromStore(): void {
    const { dragPreviewIds, dragPreviewDelta } = useMapStore.getState()
    if (!dragPreviewIds?.length) {
      this.clearDragOverlay()
      return
    }
    this.startDragOverlay(dragPreviewIds, dragPreviewDelta)
  }

  private restartDragOverlay(): void {
    const { dragPreviewIds, dragPreviewDelta } = useMapStore.getState()
    if (!dragPreviewIds?.length) {
      this.clearDragOverlay()
      return
    }
    this.startDragOverlay(dragPreviewIds, dragPreviewDelta)
  }

  /** One-shot overlay upload (drag start / id-set change). */
  private startDragOverlay(
    dragIds: string[],
    delta: { dx: number; dy: number } | null | undefined,
  ): void {
    if (!dragIds.length) {
      this.clearDragOverlay()
      return
    }
    this.dragActive = true
    const handle = this.md?.wasm
    if (!handle) return
    handle.refresh()
    const n = handle.slot_len
    const xyCopy = new Float32Array(
      new Float32Array(wasmBg.memory.buffer, handle.slot_xy_ptr, n * 2),
    )
    const ids = handle.slot_ids() as string[]
    this.lastIds = ids
    this.lastLen = n
    const idToRow = new Map(ids.map((id, i) => [id, i]))
    const { overlay, count } = this.buildDragOverlay(dragIds, xyCopy, idToRow)
    this.engine.upload_slot_drag_lane(overlay, count > 0)
    this.engine.set_slot_drag_delta(delta?.dx ?? 0, delta?.dy ?? 0)
  }

  private syncClusters(): void {
    if (this.disposed || !this.atlasReady) return
    const zoom = this.engine.zoom
    const n = this.lastLen
    const cm = clusterMode(n, zoom)
    const version = getClusterMarkersVersion()
    const zBucket = Math.round(zoom * 10) // fine enough for disc size stability
    if (
      cm === this.lastClusterMode &&
      version === this.lastClusterVersion &&
      zBucket === this.lastZoomBucket
    ) {
      return
    }
    this.lastClusterMode = cm
    this.lastClusterVersion = version
    this.lastZoomBucket = zBucket

    if (!cm) {
      this.engine.upload_cluster_lane(new Uint8Array(0), false)
      // Show detail slots (full-n with selection baked in)
      this.engine.upload_slot_lane(this.repackCurrent(), true)
      this.slotsLaneIsSelectionOnly = false
      return
    }

    const markers = getClusterMarkers(zoom)
    const xs = markers.map((m) => m.x)
    const ys = markers.map((m) => m.y)
    const counts = markers.map((m) => m.count)
    const bytes = packClusterInstances(xs, ys, counts)
    this.engine.upload_cluster_lane(bytes, bytes.length > 0)
    // Hide full detail lane; selection-only rings over clusters
    this.reuploadSelectionOnlyFromDoc()
  }

  private applyClusterVisibility(cm: boolean): void {
    if (!cm) {
      this.pushFromDoc(true)
      this.engine.upload_cluster_lane(new Uint8Array(0), false)
    } else {
      this.syncClusters()
    }
  }

  private applySelectionOnlyVisible(
    _ids: string[],
    xy: Float32Array,
    selected: boolean[],
  ): void {
    // Deck detail=false: only selected rings over clusters
    const selIdx: number[] = []
    for (let i = 0; i < selected.length; i++) if (selected[i]) selIdx.push(i)
    if (!selIdx.length) {
      this.engine.upload_slot_lane(new Uint8Array(0), false)
      this.slotsLaneIsSelectionOnly = true
      return
    }
    const out = new Uint8Array(selIdx.length * SLOT_ICON_STRIDE)
    const yellow = packRgbaU32(SLOT_SELECTED_RGBA)
    for (let k = 0; k < selIdx.length; k++) {
      const i = selIdx[k]
      if (i === undefined) continue
      packIconInstance(
        out,
        k * SLOT_ICON_STRIDE,
        xy[i * 2] ?? 0,
        xy[i * 2 + 1] ?? 0,
        SLOT_SELECTED_PX,
        0,
        yellow,
      )
    }
    this.engine.upload_slot_lane(out, true)
    this.slotsLaneIsSelectionOnly = true
  }

  private repackCurrent(): Uint8Array {
    const handle = this.md?.wasm
    if (!handle) return new Uint8Array(0)
    handle.refresh()
    const n = handle.slot_len
    const ids = handle.slot_ids() as string[]
    const xy = new Float32Array(wasmBg.memory.buffer, handle.slot_xy_ptr, n * 2)
    return packSlotInstances(new Float32Array(xy), this.selectedMask(ids))
  }

  private syncZoomUniform(): void {
    this.engine.set_slot_px_to_m(pxToMAtZoom(this.engine.zoom))
  }

  private publishDebug(): void {
    if (typeof window === 'undefined') return
    let stats: Record<string, unknown> = {}
    try {
      stats = JSON.parse(this.engine.stats()) as Record<string, unknown>
    } catch {
      /* ignore */
    }
    ;(window as unknown as { __wgpuSlotStats?: unknown }).__wgpuSlotStats = {
      slot_len: this.lastLen,
      slot_instances: stats.slot_instances ?? 0,
      slot_drag_instances: stats.slot_drag_instances ?? 0,
      cluster_instances: stats.cluster_instances ?? 0,
      cluster_mode: this.lastClusterMode,
      slots_lane_selection_only: this.slotsLaneIsSelectionOnly,
      drag_active: this.dragActive,
      atlas_ready: this.atlasReady,
      uniform_bytes_last_frame: stats.uniform_bytes_last_frame ?? 0,
      zoom: this.engine.zoom,
    }
  }
}

/** Expose for tests (cluster gate pure). */
export { CLUSTER_SLOT_THRESHOLD, ZOOM_CLUSTER_MAX }
