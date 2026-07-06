import { describe, it, expect, beforeAll } from 'vitest'
import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { worldToPixel, bilinearSample } from '@/features/tactical-map/dem/sampleElevation'
import type { TerrainManifest } from '@/features/tactical-map/dem/terrainManifest'
import * as wasm from '@/wasm/pkg/map_engine_wasm'

// The wasm PNG decode (dem::png_decode → meters cache) is validated on the REAL committed Everon
// 16-bit PNG against the same 11 ground-truth anchors the pngjs path uses (sampleElevation.test.ts,
// T-091.0). Sampling the meters cache directly == bilinear-on-uint16-then-convert (affine), so a
// match within ±0.01 m proves the Rust decode reproduces pngjs on real data (endianness + channel).
const MAP_ASSETS = resolve(process.cwd(), '../../../packages/map-assets')
const EVERON_MANIFEST = resolve(MAP_ASSETS, 'everon/manifest.json')
const EVERON_PNG = resolve(MAP_ASSETS, 'everon/dem/everon-dem-16bit.png')

describe('map-engine-wasm dem::png_decode — real Everon PNG anchors (±0.01 m)', () => {
  let manifest: TerrainManifest
  let meters: Float32Array
  let width: number
  let height: number

  beforeAll(() => {
    manifest = JSON.parse(readFileSync(EVERON_MANIFEST, 'utf8')) as TerrainManifest
    const bytes = new Uint8Array(readFileSync(EVERON_PNG))
    const decoded = wasm.dem_decode_png_to_meters(
      bytes,
      manifest.dem.heightRangeMinM,
      manifest.dem.heightRangeMaxM,
    )
    meters = decoded.meters
    width = decoded.width
    height = decoded.height
    decoded.free()
  })

  it('decodes to the manifest raster dims', () => {
    expect(width).toBe(manifest.dem.widthPx)
    expect(height).toBe(manifest.dem.heightPx)
    expect(meters.length).toBe(width * height)
  })

  // [id, x, z(=editor y), expected demY meters] — identical to sampleElevation.test.ts.
  const anchors: Array<[string, number, number, number]> = [
    ['bridgehead-sl', 4839.2, 6620.8, 121.784],
    ['bridgehead-tl0', 4836.9, 6626.5, 123.328],
    ['bridgehead-tl1', 4831.2, 6628.8, 123.602],
    ['coast-w', 1000, 6400, 0.054],
    ['valley-inland', 5000, 5000, 80.871],
    ['hill-north', 9600, 3200, 221.652],
    ['peak-central', 6400, 6400, 157.882],
    ['coast-sw', 2000, 2000, -7.408],
    ['seabed-e', 11000, 6400, -84.86],
    ['shelf-ne', 8000, 8000, -18.314],
    ['mid-s', 3200, 9600, -47.743],
  ]

  for (const [id, x, z, expected] of anchors) {
    it(`${id} (${x}, ${z}) → ${expected} m`, () => {
      const { px, py } = worldToPixel(x, z, manifest)
      const got = bilinearSample(meters, width, height, px, py)
      expect(Math.abs(got - expected)).toBeLessThanOrEqual(0.01)
    })
  }
})
