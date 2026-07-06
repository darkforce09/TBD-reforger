import { describe, it, expect } from 'vitest'
import {
  uint16ToMeters,
  worldToPixel,
  bilinearSample,
} from '@/features/tactical-map/dem/sampleElevation'
import type { TerrainManifest } from '@/features/tactical-map/dem/terrainManifest'
import { downsampleDemGrid, reduceGrid2x } from '@/features/tactical-map/worldmap/demGrid'
import { buildSeaBandGeometry } from '@/features/tactical-map/worldmap/seaBand'
import { contourSegments, contourLevels } from '@/features/tactical-map/worldmap/contours'
import * as wasm from '@/wasm/pkg/map_engine_wasm'
import { f32BytesEqual, firstF32Mismatch, intArrayEqual } from './parity'

const MIN_M = -204.78
const MAX_M = 375.53

/** Radial hill spanning sea (≤ -50) → peak (+80), stored to f32 (the exact bytes both impls read). */
function syntheticMeters(n: number): Float32Array {
  const r = new Float32Array(n * n)
  const c = (n - 1) / 2
  for (let y = 0; y < n; y++) {
    for (let x = 0; x < n; x++) {
      r[y * n + x] = 80 - 3 * Math.hypot(x - c, y - c)
    }
  }
  return r
}

describe('map-engine-wasm dem::sample — Class R (bit-identical)', () => {
  it('uint16_to_meters matches the TS across the range', () => {
    for (const u of [0, 1, 255, 12345, 32768, 54321, 65535]) {
      expect(wasm.uint16_to_meters(u, MIN_M, MAX_M)).toBe(uint16ToMeters(u, MIN_M, MAX_M))
    }
  })

  it('world_to_pixel px/py match the TS worldToPixel', () => {
    const man = {
      worldBounds: [0, 0, 12800, 12800],
      dem: {
        widthPx: 64,
        heightPx: 64,
        axisFlip: {},
        heightRangeMinM: MIN_M,
        heightRangeMaxM: MAX_M,
      },
    } as unknown as TerrainManifest
    for (const [x, z] of [
      [0, 0],
      [6400, 6400],
      [12800, 12800],
      [123.4, 9876.5],
    ]) {
      const ts = worldToPixel(x, z, man)
      const w = wasm.world_to_pixel(x, z, 0, 0, 12800, 12800, 64, 64, false, false)
      expect(w[2]).toBe(ts.px)
      expect(w[3]).toBe(ts.py)
    }
  })

  it('bilinear_sample_f32 matches the TS bilinearSample on the meters grid', () => {
    const meters = syntheticMeters(64)
    for (const [px, py] of [
      [0.5, 0.5],
      [10.25, 20.75],
      [63, 63],
      [31.9, 0.01],
    ]) {
      expect(wasm.bilinear_sample_f32(meters, 64, 64, px, py)).toBe(
        bilinearSample(meters, 64, 64, px, py),
      )
    }
  })
})

describe('map-engine-wasm dem::downsample + geometry — Class R (byte-identical)', () => {
  const meters = syntheticMeters(64)
  const tsGrid = downsampleDemGrid(meters, 64, 64, 4, 12800, 12800)
  const wGrid = wasm.DemGrid.downsample(meters, 64, 64, 4, 12800, 12800)

  it('downsample data + dims + maxElev are byte-identical', () => {
    expect(wGrid.cols).toBe(tsGrid.cols)
    expect(wGrid.rows).toBe(tsGrid.rows)
    expect(wGrid.cell_x).toBe(tsGrid.cellX)
    expect(wGrid.cell_y).toBe(tsGrid.cellY)
    expect(wGrid.max_elev_m).toBe(tsGrid.maxElevM)
    expect(firstF32Mismatch(wGrid.data, tsGrid.data)).toBe(-1)
    expect(f32BytesEqual(wGrid.data, tsGrid.data)).toBe(true)
  })

  it('reduceGrid2x is byte-identical', () => {
    const tsR = reduceGrid2x(tsGrid)
    const wR = wGrid.reduce()
    expect(wR.max_elev_m).toBe(tsR.maxElevM)
    expect(f32BytesEqual(wR.data, tsR.data)).toBe(true)
  })

  it('sea band geometry is byte-identical', () => {
    const ts = buildSeaBandGeometry(tsGrid)
    const w = wGrid.sea_band()
    expect(w.polygon_count).toBe(ts.polygonCount)
    expect(f32BytesEqual(w.fill_positions, ts.fillPositions)).toBe(true)
    expect(intArrayEqual(w.fill_start_indices, ts.fillStartIndices)).toBe(true)
    expect(intArrayEqual(w.fill_colors, ts.fillColors)).toBe(true)
  })

  it('contour segments are byte-identical', () => {
    const levels = contourLevels(10, tsGrid.maxElevM)
    const ts = contourSegments(tsGrid, levels)
    const w = wGrid.contours(new Float64Array(levels))
    expect(firstF32Mismatch(w, ts)).toBe(-1)
    expect(f32BytesEqual(w, ts)).toBe(true)
  })
})
