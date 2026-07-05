// T-090.5.1 — export-chunk math: floor(x/512) keying to match on-disk {cx}_{cy} artifacts,
// terrain clamping (never request impossible ids), the §6 border-preload rule
// max(5% span, 1 chunk), and the oversized-object ring.
import { describe, it, expect } from 'vitest'
import {
  DEFAULT_CHUNK_SIZE_M,
  chunkId,
  chunkIdsForRect,
  chunkIdsForViewport,
  chunkRectForBbox,
  expandBbox,
  expandChunkRect,
  preloadMarginM,
  type Bbox,
} from './chunkMath'

const EVERON = { width: 12800, height: 12800 } // 25×25 chunks @ 512

describe('chunk keying', () => {
  it('floors world meters to the export grid (T-090.3.1 contract)', () => {
    expect(chunkRectForBbox([0, 0, 511.9, 511.9], EVERON)).toEqual({
      cx0: 0,
      cy0: 0,
      cx1: 0,
      cy1: 0,
    })
    expect(chunkRectForBbox([512, 512, 1024, 1024], EVERON)).toEqual({
      cx0: 1,
      cy0: 1,
      cx1: 2,
      cy1: 2,
    })
  })

  it('formats ids as the on-disk {cx}_{cy} stem', () => {
    expect(chunkId(3, 17)).toBe('3_17')
    expect(chunkIdsForRect({ cx0: 1, cy0: 2, cx1: 2, cy1: 3 })).toEqual([
      '1_2',
      '2_2',
      '1_3',
      '2_3',
    ])
  })

  it('clamps to the terrain grid — negative and past-edge coords never leak ids', () => {
    expect(chunkRectForBbox([-5000, -5000, -1, -1], EVERON)).toEqual({
      cx0: 0,
      cy0: 0,
      cx1: 0,
      cy1: 0,
    })
    expect(chunkRectForBbox([12799, 12799, 99999, 99999], EVERON)).toEqual({
      cx0: 24,
      cy0: 24,
      cx1: 24,
      cy1: 24,
    })
  })

  it('honors a manifest chunkSizeM override', () => {
    expect(chunkRectForBbox([0, 0, 300, 300], { width: 1024, height: 1024 }, 256)).toEqual({
      cx0: 0,
      cy0: 0,
      cx1: 1,
      cy1: 1,
    })
  })
})

describe('border preload (plan §6)', () => {
  it('margin = max(5% of larger span, one chunk)', () => {
    // Small viewport: 5% of 2000 = 100 < 512 → one-chunk floor wins.
    expect(preloadMarginM([0, 0, 2000, 2000])).toBe(DEFAULT_CHUNK_SIZE_M)
    // Full island: 5% of 12800 = 640 > 512.
    expect(preloadMarginM([0, 0, 12800, 12800])).toBeCloseTo(640)
    // Asymmetric viewport uses the larger span.
    expect(preloadMarginM([0, 0, 12800, 100])).toBeCloseTo(640)
  })

  it('expandBbox is symmetric and unclamped (chunk conversion clamps)', () => {
    expect(expandBbox([100, 200, 300, 400], 50)).toEqual([50, 150, 350, 450])
  })

  it('viewport ids include the preload ring', () => {
    // 512-wide viewport exactly inside chunk (2,2): margin = 512 → 3×3 ring around it.
    const ids = chunkIdsForViewport([1024, 1024, 1536, 1536] as Bbox, EVERON)
    expect(ids).toHaveLength(16) // bbox+512 spans 512..2048 → chunks 1..4 → 4×4 (edges touch)
    expect(ids).toContain('1_1')
    expect(ids).toContain('4_4')
    expect(ids).not.toContain('0_0')
  })
})

describe('oversized-object ring (plan §6)', () => {
  it('adds one clamped ring per side', () => {
    expect(expandChunkRect({ cx0: 0, cy0: 5, cx1: 1, cy1: 6 }, 1, EVERON)).toEqual({
      cx0: 0, // clamped
      cy0: 4,
      cx1: 2,
      cy1: 7,
    })
  })

  it('chunkIdsForViewport extraRing widens the fetch set', () => {
    const base = chunkIdsForViewport([6000, 6000, 6100, 6100], EVERON)
    const wide = chunkIdsForViewport([6000, 6000, 6100, 6100], EVERON, { extraRing: 1 })
    expect(wide.length).toBeGreaterThan(base.length)
    for (const id of base) expect(wide).toContain(id)
  })
})
