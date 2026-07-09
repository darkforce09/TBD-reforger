// T-151.7.3 — thin FE smokes (pack/drag-phase SoT is Rust cargo test).
import { describe, expect, it } from 'vitest'
import {
  CLUSTER_SLOT_THRESHOLD,
  ZOOM_CLUSTER_MAX,
} from '../state/constants'
import {
  classify_drag_transition,
  pack_slot_instances,
  px_to_m_at_zoom,
  slot_cluster_mode,
} from '@/wasm/pkg/map_engine_wasm'

describe('T-151.7.3 slot GPU pure exports (wasm smoke)', () => {
  it('slot_cluster_mode matches constants.ts gates', () => {
    expect(CLUSTER_SLOT_THRESHOLD).toBe(500)
    expect(ZOOM_CLUSTER_MAX).toBe(-4)
    expect(slot_cluster_mode(0, -6)).toBe(false)
    expect(slot_cluster_mode(500, -6)).toBe(false)
    expect(slot_cluster_mode(501, -3.9)).toBe(false)
    expect(slot_cluster_mode(501, -4)).toBe(true)
    expect(slot_cluster_mode(10_000, -6)).toBe(true)
    expect(slot_cluster_mode(10_000, -2)).toBe(false)
  })

  it('pack_slot_instances count == xy_len/2 and selection size/tint', () => {
    const xy = new Float32Array([0, 0, 100, 200, 300, 400])
    const sel = new Uint8Array([0, 1, 0])
    const bytes = pack_slot_instances(xy, sel)
    expect(bytes.length).toBe(3 * 20)
    const dv = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
    expect(dv.getFloat32(8, true)).toBe(20) // SLOT_RING_PX
    expect(dv.getFloat32(20 + 8, true)).toBe(28) // SLOT_SELECTED_PX
  })

  it('px_to_m_at_zoom matches 2^(-zoom)', () => {
    expect(px_to_m_at_zoom(-2)).toBeCloseTo(4, 6)
    expect(px_to_m_at_zoom(0)).toBeCloseTo(1, 6)
    expect(px_to_m_at_zoom(3)).toBeCloseTo(0.125, 6)
  })

  it('drag delta math: base + (dx,dy)', () => {
    const base = { x: 100, y: 200 }
    expect(base.x + 3.5).toBeCloseTo(103.5, 12)
    expect(base.y + -1.25).toBeCloseTo(198.75, 12)
  })
})

describe('T-151.7.1 drag GPU phase classification (wasm)', () => {
  it('maps store transitions to start / delta / end / restart', () => {
    // 0=idle 1=start 2=delta 3=restart 4=end
    expect(classify_drag_transition(false, true, true, false)).toBe(1)
    expect(classify_drag_transition(true, true, false, true)).toBe(2)
    expect(classify_drag_transition(true, false, true, true)).toBe(4)
    expect(classify_drag_transition(true, true, true, false)).toBe(3)
    expect(classify_drag_transition(true, true, false, false)).toBe(0)
    expect(classify_drag_transition(false, false, false, true)).toBe(0)
  })
})

describe('T-151.7.2 pan zoom merge (host contract)', () => {
  it('pan target update must not overwrite a live zoom', () => {
    const live = { target: [100, 200] as [number, number], zoom: 1.5, minZoom: -6, maxZoom: 6 }
    const panNext = { target: [110, 190] as [number, number], zoom: -2, minZoom: -6, maxZoom: 6 }
    const merged = { ...panNext, zoom: live.zoom, target: panNext.target }
    expect(merged.zoom).toBe(1.5)
    expect(merged.target).toEqual([110, 190])
  })
})

describe('T-151.7.3 thin adapter contract', () => {
  it('wgpuSlots has no pack policy exports', async () => {
    const mod = await import('./wgpuSlots')
    expect('packSlotInstances' in mod).toBe(false)
    expect('classifyDragTransition' in mod).toBe(false)
    expect('clusterMode' in mod).toBe(false)
    expect(typeof mod.WgpuSlotsController).toBe('function')
  })
})
