// T-151.1 L4 — Class R golden matrix for `pickBaseLevel` (the device-fit mip level the wgpu engine
// allocates the unified satellite texture at). Uses the EXPORTED `pickBaseLevel` from
// satelliteUnified.ts verbatim (JS owns the TBDS parse; the engine never re-implements it).
import { describe, expect, it } from 'vitest'
import { pickBaseLevel, type TbdSatIndex, type TbdSatMip } from '../layers/satelliteUnified'

/** Everon unified index: base 12800², 14 mips, GL-rule halving (`max(1, floor(w/2))`).
 *  Levels 0..13 → 12800, 6400, 3200, 1600, 800, 400, 200, 100, 50, 25, 12, 6, 3, 1. */
function everonIndex(): TbdSatIndex {
  const mips: TbdSatMip[] = []
  let w = 12800
  let h = 12800
  for (let level = 0; level < 14; level++) {
    mips.push({ level, width: w, height: h, tiles: [] })
    w = Math.max(1, Math.floor(w / 2))
    h = Math.max(1, Math.floor(h / 2))
  }
  return {
    formatVersion: 1,
    terrainId: 'everon',
    worldBounds: [0, 0, 12800, 12800],
    baseWidthPx: 12800,
    baseHeightPx: 12800,
    mipCount: 14,
    mips,
  }
}

describe('pickBaseLevel (T-151.1 L4 device-fit)', () => {
  const index = everonIndex()

  // The locked matrix: the first mip whose max(w,h) ≤ the device limit.
  it.each([
    [16384, 0], // 12800 ≤ 16384 (desktop default)
    [8192, 1], //  6400 ≤ 8192 (12800 > 8192)
    [4096, 2], //  3200 ≤ 4096 (6400 > 4096)
  ])('maxTextureDimension2D=%i → base level %i', (limit, expected) => {
    expect(pickBaseLevel(index, limit)).toBe(expected)
  })

  it('degenerate limits: 256 → level 6 (200²), 1 → coarsest level 13 (1²)', () => {
    expect(pickBaseLevel(index, 256)).toBe(6)
    expect(pickBaseLevel(index, 1)).toBe(13)
  })
})
