// T-151.6 — pure pack + cluster gate unit tests (no GPU).
import { describe, expect, it } from 'vitest'
import {
  CLUSTER_SLOT_THRESHOLD,
  ZOOM_CLUSTER_MAX,
} from '../state/constants'
import {
  clusterDiscSizePx,
  packClusterInstances,
  packRgbaU32,
  packSlotInstances,
  pxToMAtZoom,
  SLOT_ICON_STRIDE,
  SLOT_PRIMARY_RGBA,
  SLOT_RING_PX,
  SLOT_SELECTED_PX,
  SLOT_SELECTED_RGBA,
} from './slotAtlas'
import { classifyDragTransition, clusterMode } from './wgpuSlots'

describe('T-151.6 slot GPU pack + gates', () => {
  it('cluster_mode matches constants.ts gates', () => {
    expect(CLUSTER_SLOT_THRESHOLD).toBe(500)
    expect(ZOOM_CLUSTER_MAX).toBe(-4)
    expect(clusterMode(0, -6)).toBe(false)
    expect(clusterMode(500, -6)).toBe(false)
    expect(clusterMode(501, -3.9)).toBe(false)
    expect(clusterMode(501, -4)).toBe(true)
    expect(clusterMode(10_000, -6)).toBe(true)
    expect(clusterMode(10_000, -2)).toBe(false)
  })

  it('pack_slot_instances count == xy_len/2 and selection size/tint', () => {
    const xy = new Float32Array([0, 0, 100, 200, 300, 400])
    const sel = [false, true, false]
    const bytes = packSlotInstances(xy, sel)
    expect(bytes.length).toBe(3 * SLOT_ICON_STRIDE)
    const dv = new DataView(bytes.buffer)
    expect(dv.getFloat32(8, true)).toBe(SLOT_RING_PX)
    expect(dv.getUint32(16, true)).toBe(packRgbaU32(SLOT_PRIMARY_RGBA))
    expect(dv.getFloat32(20 + 8, true)).toBe(SLOT_SELECTED_PX)
    expect(dv.getUint32(20 + 16, true)).toBe(packRgbaU32(SLOT_SELECTED_RGBA))
  })

  it('pack_cluster_instances disc glyph + size formula', () => {
    const bytes = packClusterInstances([10, 20], [30, 40], [1, 1000])
    expect(bytes.length).toBe(2 * SLOT_ICON_STRIDE)
    const dv = new DataView(bytes.buffer)
    expect(dv.getUint16(14, true)).toBe(1) // disc glyph
    expect(dv.getFloat32(8, true)).toBeCloseTo(clusterDiscSizePx(1), 5)
    expect(clusterDiscSizePx(1000)).toBeCloseTo(48, 5)
  })

  it('px_to_m_at_zoom matches 2^(-zoom)', () => {
    expect(pxToMAtZoom(-2)).toBeCloseTo(4, 6)
    expect(pxToMAtZoom(0)).toBeCloseTo(1, 6)
    expect(pxToMAtZoom(3)).toBeCloseTo(0.125, 6)
  })

  it('drag delta math: base + (dx,dy)', () => {
    const base = { x: 100, y: 200 }
    const dx = 3.5
    const dy = -1.25
    expect(base.x + dx).toBeCloseTo(103.5, 12)
    expect(base.y + dy).toBeCloseTo(198.75, 12)
  })
})

describe('T-151.7.1 drag GPU phase classification', () => {
  it('maps store transitions to start / delta / end / restart', () => {
    expect(classifyDragTransition(null, ['a'], true, false)).toBe('start')
    expect(classifyDragTransition(['a'], ['a'], false, true)).toBe('delta')
    expect(classifyDragTransition(['a'], null, true, true)).toBe('end')
    expect(classifyDragTransition(['a'], ['a', 'b'], true, false)).toBe('restart')
    expect(classifyDragTransition(['a'], ['a'], false, false)).toBe('idle')
    expect(classifyDragTransition(null, null, false, true)).toBe('idle')
  })
})
