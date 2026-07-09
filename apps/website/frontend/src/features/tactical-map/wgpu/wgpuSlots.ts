// T-151.6 W6 — mission slot / selection / drag / cluster GPU bridge for WgpuTacticalMap.
// Positions from MissionDoc SoA (refresh → slot_xy_ptr / slot_len); never slotsById as SoT.

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
  SLOT_RING_PX,
  SLOT_SELECTED_PX,
  SLOT_PRIMARY_RGBA,
  SLOT_SELECTED_RGBA,
  packRgbaU32,
  packIconInstance,
} from './slotAtlas'

export function clusterMode(slotLen: number, deckZoom: number): boolean {
  return slotLen > CLUSTER_SLOT_THRESHOLD && deckZoom <= ZOOM_CLUSTER_MAX
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
    this.syncDrag()
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
      return
    }
    this.unsubDoc = md.subscribe(() => {
      this.pushFromDoc(false)
      this.syncClusters()
      this.publishDebug()
    })
    // Zustand selection / drag / zoom (gesture may be scripted until W7).
    this.unsubStore?.()
    this.unsubStore = useMapStore.subscribe((s, prev) => {
      if (s.selection !== prev.selection) this.syncSelection()
      if (
        s.dragPreviewIds !== prev.dragPreviewIds ||
        s.dragPreviewDelta !== prev.dragPreviewDelta
      ) {
        this.syncDrag()
      }
      if (s.deckZoom !== prev.deckZoom) {
        this.onCameraMoved()
      }
    })
    if (this.atlasReady) {
      this.pushFromDoc(true)
      this.syncSelection()
      this.syncDrag()
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

    if (!orderStable) {
      const zoom = this.engine.zoom
      const cm = clusterMode(n, zoom)
      this.lastClusterMode = cm
      // In cluster mode base shows selection-only (Deck detail=false); still upload full buffer
      // and toggle visibility so zoom-in is instant.
      this.engine.upload_slot_lane(bytes, !cm)
      if (cm) this.applySelectionOnlyVisible(ids, xyCopy, sel)
    } else {
      // Order stable — could patch dirty ranges; still full-upload for simplicity when anything
      // changed (caller notified). Selection/drag use dedicated paths.
      this.engine.upload_slot_lane(bytes, !clusterMode(n, this.engine.zoom))
    }

    this.lastIds = ids
    this.lastLen = n
    // Drag exclude rows may be stale after structural change.
    if (this.dragActive) this.syncDrag()
  }

  private selectedMask(ids: string[]): boolean[] {
    const sel = useMapStore.getState().selection
    if (sel.kind === 'none' || !sel.ids.length) return ids.map(() => false)
    const set = new Set(sel.ids)
    return ids.map((id) => set.has(id))
  }

  private syncSelection(): void {
    if (this.disposed || !this.atlasReady || !this.lastIds.length) return
    if (this.dragActive) return // drag overlay owns tint for those ids
    const sel = useMapStore.getState().selection
    const set =
      sel.kind !== 'none' && sel.ids.length ? new Set(sel.ids) : new Set<string>()
    const primary = packRgbaU32(SLOT_PRIMARY_RGBA)
    const yellow = packRgbaU32(SLOT_SELECTED_RGBA)
    // O(selection) + O(prev) — patch size+tint for changed rows. For simplicity patch all
    // selected + unselected that were in previous selection only needs full scan of lastIds
    // for selected flags; k = n is fine at small n, at large n scan once:
    for (let i = 0; i < this.lastIds.length; i++) {
      if (this.hiddenRows.has(i)) continue
      const id = this.lastIds[i]
      if (id === undefined) continue
      const isSel = set.has(id)
      const size = isSel ? SLOT_SELECTED_PX : SLOT_RING_PX
      const tint = isSel ? yellow : primary
      // size@+8 (4) + yaw@+12 (2) + glyph@+14 (2) + tint@+16 (4) = 12 B from offset+8
      const patch = new Uint8Array(12)
      const dv = new DataView(patch.buffer)
      dv.setFloat32(0, size, true)
      dv.setInt16(4, 0, true)
      dv.setUint16(6, 0, true) // ring glyph
      dv.setUint32(8, tint >>> 0, true)
      this.engine.patch_slot_lane(i * SLOT_ICON_STRIDE + 8, patch)
    }
    // Cluster mode: also refresh selection-only overlay path
    if (this.lastClusterMode) {
      this.pushFromDoc(true)
    }
  }

  private clearDragOverlay(): void {
    if (!this.dragActive) return
    this.dragActive = false
    this.hiddenRows.clear()
    this.engine.clear_slot_drag_lane()
    this.pushFromDoc(true)
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
      // Hide base row: alpha 0 tint
      const hide = new Uint8Array(12)
      const dv = new DataView(hide.buffer)
      dv.setFloat32(0, SLOT_SELECTED_PX, true)
      dv.setInt16(4, 0, true)
      dv.setUint16(6, 0, true)
      dv.setUint32(8, 0, true)
      this.engine.patch_slot_lane(row * SLOT_ICON_STRIDE + 8, hide)
      k++
    }
    return { overlay: overlay.subarray(0, k * SLOT_ICON_STRIDE), count: k }
  }

  private syncDrag(): void {
    if (this.disposed || !this.atlasReady) return
    const { dragPreviewIds, dragPreviewDelta } = useMapStore.getState()
    if (!dragPreviewIds?.length) {
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
    const { overlay, count } = this.buildDragOverlay(dragPreviewIds, xyCopy, idToRow)
    this.engine.upload_slot_drag_lane(overlay, count > 0)
    this.engine.set_slot_drag_delta(dragPreviewDelta?.dx ?? 0, dragPreviewDelta?.dy ?? 0)
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
      // Show detail slots
      this.engine.upload_slot_lane(
        // re-push if we had hidden the lane — cheaper: empty visible true sticky doesn't restore.
        // Full re-push from doc.
        this.repackCurrent(),
        true,
      )
      return
    }

    const markers = getClusterMarkers(zoom)
    const xs = markers.map((m) => m.x)
    const ys = markers.map((m) => m.y)
    const counts = markers.map((m) => m.count)
    const bytes = packClusterInstances(xs, ys, counts)
    this.engine.upload_cluster_lane(bytes, bytes.length > 0)
    // Hide full detail lane; selection-only drawn via repack of selected
    const handle = this.md?.wasm
    if (handle) {
      handle.refresh()
      const ids = handle.slot_ids() as string[]
      const xy = new Float32Array(
        wasmBg.memory.buffer,
        handle.slot_xy_ptr,
        handle.slot_len * 2,
      )
      this.applySelectionOnlyVisible(ids, new Float32Array(xy), this.selectedMask(ids))
    } else {
      this.engine.upload_slot_lane(new Uint8Array(0), false)
    }
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
      atlas_ready: this.atlasReady,
      uniform_bytes_last_frame: stats.uniform_bytes_last_frame ?? 0,
      zoom: this.engine.zoom,
    }
  }
}

/** Expose for tests (cluster gate pure). */
export { CLUSTER_SLOT_THRESHOLD, ZOOM_CLUSTER_MAX }
