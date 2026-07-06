import { describe, it, expect } from 'vitest'
import * as wasm from '@/wasm/pkg/map_engine_wasm'
import { maxAbsDiff } from './parity'

// Faithful transcription of useDemLayer.buildHillshadeImage (Class T) minus the `ImageData` wrap
// (absent in the node test env). This is the JS oracle; the wasm output must match it within 1 gray
// level (atan/atan2/sin/cos differ across libm below one level except at exact x.5 round edges).
function tsHillshade(
  meters: Float32Array,
  srcW: number,
  srcH: number,
): { data: Uint8Array; w: number; h: number } {
  const MAX_EDGE = 1024
  const AZIMUTH_RAD = (315 * Math.PI) / 180
  const ALTITUDE_RAD = (45 * Math.PI) / 180
  const ZENITH_RAD = Math.PI / 2 - ALTITUDE_RAD
  const scale = Math.max(1, Math.ceil(Math.max(srcW, srcH) / MAX_EDGE))
  const w = Math.max(1, Math.floor(srcW / scale))
  const h = Math.max(1, Math.floor(srcH / scale))
  const cellMeters = srcW / w
  const ds = new Float32Array(w * h)
  for (let y = 0; y < h; y++) {
    const sy = Math.min(srcH - 1, y * scale)
    for (let x = 0; x < w; x++) {
      const sx = Math.min(srcW - 1, x * scale)
      ds[y * w + x] = meters[sy * srcW + sx]
    }
  }
  const data = new Uint8Array(w * h * 4)
  const at = (x: number, y: number) =>
    ds[Math.min(h - 1, Math.max(0, y)) * w + Math.min(w - 1, Math.max(0, x))]
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const a = at(x - 1, y - 1)
      const b = at(x, y - 1)
      const c = at(x + 1, y - 1)
      const d = at(x - 1, y)
      const f = at(x + 1, y)
      const g = at(x - 1, y + 1)
      const hh = at(x, y + 1)
      const i = at(x + 1, y + 1)
      const dzdx = (c + 2 * f + i - (a + 2 * d + g)) / (8 * cellMeters)
      const dzdy = (g + 2 * hh + i - (a + 2 * b + c)) / (8 * cellMeters)
      const slope = Math.atan(Math.sqrt(dzdx * dzdx + dzdy * dzdy))
      const aspect = Math.atan2(dzdy, -dzdx)
      let hs =
        Math.cos(ZENITH_RAD) * Math.cos(slope) +
        Math.sin(ZENITH_RAD) * Math.sin(slope) * Math.cos(AZIMUTH_RAD - aspect)
      if (hs < 0) hs = 0
      const gray = Math.round(hs * 255)
      const o = ((h - 1 - y) * w + x) * 4
      data[o] = gray
      data[o + 1] = gray
      data[o + 2] = gray
      data[o + 3] = 255
    }
  }
  return { data, w, h }
}

function relief(n: number): Float32Array {
  const r = new Float32Array(n * n)
  for (let y = 0; y < n; y++) {
    for (let x = 0; x < n; x++) {
      r[y * n + x] = 100 * Math.sin(x * 0.3) + 80 * Math.cos(y * 0.2) + 0.5 * (x - y)
    }
  }
  return r
}

describe('map-engine-wasm dem::hillshade — Class T (≤ 1 gray level)', () => {
  it('matches the TS Horn hillshade within 1 gray level (scale 1)', () => {
    const meters = relief(128)
    const ts = tsHillshade(meters, 128, 128)
    const w = wasm.hillshade(meters, 128, 128)
    expect(w.width).toBe(ts.w)
    expect(w.height).toBe(ts.h)
    expect(maxAbsDiff(w.data, ts.data)).toBeLessThanOrEqual(1)
  })

  it('matches within 1 gray level through the downsample branch (scale 2)', () => {
    const meters = relief(1100)
    const ts = tsHillshade(meters, 1100, 1100)
    const w = wasm.hillshade(meters, 1100, 1100)
    expect(w.width).toBe(ts.w) // floor(1100/2) = 550
    expect(w.height).toBe(ts.h)
    expect(maxAbsDiff(w.data, ts.data)).toBeLessThanOrEqual(1)
  })
})
