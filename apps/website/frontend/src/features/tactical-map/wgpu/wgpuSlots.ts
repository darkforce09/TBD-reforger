// T-151.7.3 thin adapter — Rust owns slot GPU policy.
import type { WasmMissionDoc } from '../state/wasmDoc'
import { useMapStore } from '../state/useMapStore'
import { getClusterMarkers } from '../state/slotClusterIndex'
import type { RenderEngine } from './wasmRender'
import { buildSlotAtlas } from './slotAtlas'
import { bind_mission_doc } from '@/wasm/pkg/map_engine_wasm'

export class WgpuSlotsController {
  private e: RenderEngine
  private dead = false
  private uD: (() => void) | null = null
  private uS: (() => void) | null = null
  constructor(engine: RenderEngine) { this.e = engine }
  async init(): Promise<void> {
    if (this.dead) return
    const a = buildSlotAtlas()
    this.e.ensure_slot_atlas(a.rgba, a.width, a.height, a.uv)
    this.e.on_camera_changed(); this.sync(); this.clusters()
  }
  setMissionDoc(md: WasmMissionDoc | null): void {
    if (this.dead) return
    this.uD?.(); this.uS?.(); this.uD = this.uS = null
    if (!md) { this.e.clear_slots(); return }
    const bind = () => { if (md.wasm) bind_mission_doc(this.e, md.wasm); this.clusters() }
    this.uD = md.subscribe(bind)
    this.uS = useMapStore.subscribe((s, p) => {
      if (s.selection !== p.selection || s.dragPreviewIds !== p.dragPreviewIds || s.dragPreviewDelta !== p.dragPreviewDelta) this.sync()
      if (s.deckZoom !== p.deckZoom) this.onCameraMoved()
    })
    bind(); this.sync()
  }
  onCameraMoved(): void {
    if (this.dead) return
    this.e.on_camera_changed()
    useMapStore.getState().setDeckZoom(this.e.zoom)
    this.clusters()
  }
  dispose(): void {
    this.dead = true; this.uD?.(); this.uS?.()
    try { this.e.clear_slots() } catch { /* */ }
  }
  private sync(): void {
    const st = useMapStore.getState()
    this.e.set_selection(st.selection.kind === 'slot' ? st.selection.ids : [])
    this.e.set_drag(st.dragPreviewIds ?? [], st.dragPreviewDelta?.dx ?? 0, st.dragPreviewDelta?.dy ?? 0)
  }
  private clusters(): void {
    if (!this.e.cluster_mode()) {
      this.e.set_cluster_markers(new Float64Array(0), new Float64Array(0), new Uint32Array(0))
      return
    }
    const m = getClusterMarkers(this.e.zoom)
    this.e.set_cluster_markers(Float64Array.from(m, (x) => x.x), Float64Array.from(m, (x) => x.y), Uint32Array.from(m, (x) => x.count))
  }
}
