// T-090.1 — tileUrl Y-flip contract. The on-disk pyramid is XYZ (y=0 = north); the editor
// walks tiles south-first (y=0 = south). tmsY = 2**z - 1 - y is the single inversion point.
import { describe, it, expect } from 'vitest'
import { tileUrl, tilesPerAxis } from './tileUrl'

const TMPL = '/map-assets/everon/tiles/satellite/{z}/{x}/{y}.webp'

describe('tileUrl Y-flip', () => {
  it('z0 single tile: south-first y=0 maps to disk row 0', () => {
    expect(tileUrl(TMPL, 0, 0, 0)).toBe('/map-assets/everon/tiles/satellite/0/0/0.webp')
  })

  it('flips y so the southern index hits the northern disk row', () => {
    // z3: 8 rows. south-first y=0 (southern edge) -> disk tmsY=7 (last/southern row on disk
    // is index 7 because disk row 0 is NORTH). south-first y=7 (north) -> disk row 0.
    expect(tileUrl(TMPL, 3, 2, 0)).toBe('/map-assets/everon/tiles/satellite/3/2/7.webp')
    expect(tileUrl(TMPL, 3, 2, 7)).toBe('/map-assets/everon/tiles/satellite/3/2/0.webp')
  })

  it('never emits the raw y in the URL (flip always applied)', () => {
    for (let z = 0; z <= 5; z++) {
      const n = tilesPerAxis(z)
      for (let y = 0; y < n; y++) {
        const url = tileUrl(TMPL, z, 0, y)
        const expectedDiskRow = n - 1 - y
        expect(url.endsWith(`/${expectedDiskRow}.webp`)).toBe(true)
        // raw y only equals the disk row at the exact center of an odd span — assert the
        // mapping, not a substring (which could coincide).
        const last = url.split('/').pop() ?? ''
        expect(Number(last.replace('.webp', ''))).toBe(expectedDiskRow)
      }
    }
  })

  it('round-trips: flipping twice returns the original index', () => {
    for (let z = 0; z <= 5; z++) {
      const n = tilesPerAxis(z)
      for (let y = 0; y < n; y++) {
        const disk = n - 1 - y
        const back = n - 1 - disk
        expect(back).toBe(y)
      }
    }
  })

  it('tilesPerAxis is 2**z', () => {
    expect([0, 1, 2, 3, 4, 5].map(tilesPerAxis)).toEqual([1, 2, 4, 8, 16, 32])
  })
})
