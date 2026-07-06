import { describe, it, expect } from 'vitest'
import { uint16ToMeters } from '@/features/tactical-map/dem/sampleElevation'
import { meters_cache } from '@/wasm/pkg/map_engine_wasm'
import { f32BytesEqual, firstF32Mismatch } from './parity'

// The exact TS reference the wasm must match — mirror of DemTexture.buildMetersCache
// (`out[i] = uint16ToMeters(raster[i], min, max)` into a Float32Array).
function tsMetersCache(raster: Uint16Array, minM: number, maxM: number): Float32Array {
  const out = new Float32Array(raster.length)
  for (let i = 0; i < raster.length; i++) out[i] = uint16ToMeters(raster[i], minM, maxM)
  return out
}

describe('map-engine-wasm meters_cache — Class R (byte-identical to TS)', () => {
  // Everon height range — packages/map-assets/everon/manifest.json.
  const MIN_M = -204.78
  const MAX_M = 375.53

  it('is byte-identical over 100k pseudorandom uint16 + exact endpoints', () => {
    const n = 100_000
    const raster = new Uint16Array(n)
    // Deterministic LCG so the case is reproducible across machines.
    let s = 0x12345678 >>> 0
    for (let i = 0; i < n; i++) {
      s = (Math.imul(s, 1103515245) + 12345) >>> 0
      raster[i] = s & 0xffff
    }
    raster[0] = 0 // exact min
    raster[1] = 65535 // full scale
    raster[2] = 32768 // mid

    const ts = tsMetersCache(raster, MIN_M, MAX_M)
    const rs = meters_cache(raster, MIN_M, MAX_M)

    expect(rs.length).toBe(ts.length)
    expect(firstF32Mismatch(ts, rs)).toBe(-1)
    expect(f32BytesEqual(ts, rs)).toBe(true)
  })

  it('handles the empty raster', () => {
    expect(meters_cache(new Uint16Array(0), MIN_M, MAX_M).length).toBe(0)
  })
})
