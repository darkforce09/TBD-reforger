// T-090.5.4 — Sea-band fill geometry + the DEM downsample it marches over. Fixture grids are
// built directly (bypassing the DEM decode); inside test is elevation ≤ iso.
import { describe, it, expect } from 'vitest'
import { buildSeaBandGeometry, seaFillAlpha, SEA_BAND_LEVELS, type SeaBandGeometry } from './seaBand'
import { DEM_VECTOR_GRID_FACTOR, demGridDims, downsampleDemGrid, type DemVectorGrid } from './demGrid'

/** Build a DemVectorGrid from a row-major 2-D array (row 0 = y origin), unit cells. */
function grid(rows2d: number[][]): DemVectorGrid {
  const rows = rows2d.length
  const cols = rows2d[0].length
  const data = new Float32Array(cols * rows)
  let max = -Infinity
  for (let j = 0; j < rows; j++) {
    for (let i = 0; i < cols; i++) {
      const v = rows2d[j][i]
      data[j * cols + i] = v
      if (v > max) max = v
    }
  }
  return { data, cols, rows, cellX: 1, cellY: 1, originX: 0, originY: 0, maxElevM: max }
}

/** Per-ring view: [startVertex, vertexCount, first RGBA] over the binary geometry. */
function rings(geo: SeaBandGeometry): { start: number; count: number; rgba: number[] }[] {
  const total = geo.fillPositions.length / 2
  const out: { start: number; count: number; rgba: number[] }[] = []
  for (let k = 0; k < geo.fillStartIndices.length; k++) {
    const start = geo.fillStartIndices[k]
    const end = k + 1 < geo.fillStartIndices.length ? geo.fillStartIndices[k + 1] : total
    out.push({ start, count: end - start, rgba: [...geo.fillColors.slice(4 * start, 4 * start + 4)] })
  }
  return out
}

describe('buildSeaBandGeometry', () => {
  it('all-land grid (above every sea iso) → empty geometry', () => {
    const geo = buildSeaBandGeometry(grid([
      [100, 100, 100],
      [100, 100, 100],
      [100, 100, 100],
    ]))
    expect(geo.polygonCount).toBe(0)
    expect(geo.fillPositions.length).toBe(0)
    expect(geo.fillColors.length).toBe(0)
  })

  it('all-ocean grid → one RLE span per row per level (nested fills)', () => {
    const geo = buildSeaBandGeometry(grid([
      [-100, -100, -100],
      [-100, -100, -100],
      [-100, -100, -100],
    ]))
    // 4 levels × 2 cell-rows × 1 merged span = 8 rectangles.
    expect(geo.polygonCount).toBe(8)
    expect(geo.fillStartIndices.length).toBe(8)
    // One RGBA per vertex.
    expect(geo.fillColors.length).toBe((geo.fillPositions.length / 2) * 4)
  })

  it('every ring is closed (first vertex repeated last — _normalize:false contract)', () => {
    const geo = buildSeaBandGeometry(grid([
      [-100, -100, -100],
      [-100, -100, -100],
      [-100, -100, -100],
    ]))
    for (const r of rings(geo)) {
      const fx = geo.fillPositions[2 * r.start]
      const fy = geo.fillPositions[2 * r.start + 1]
      const lx = geo.fillPositions[2 * (r.start + r.count - 1)]
      const ly = geo.fillPositions[2 * (r.start + r.count - 1) + 1]
      expect([lx, ly]).toEqual([fx, fy])
    }
  })

  it('levels are appended shallow→deep (nested painter order darkens deep water)', () => {
    const geo = buildSeaBandGeometry(grid([
      [-100, -100, -100],
      [-100, -100, -100],
      [-100, -100, -100],
    ]))
    const rs = rings(geo)
    // First ring wears the shallowest level colour, last ring the deepest.
    expect(rs[0].rgba).toEqual(SEA_BAND_LEVELS[0].rgba)
    expect(rs[rs.length - 1].rgba).toEqual(SEA_BAND_LEVELS[SEA_BAND_LEVELS.length - 1].rgba)
  })

  it('plateau exactly at iso is inside (≤ convention): all-zero fills +5 and 0 only', () => {
    const geo = buildSeaBandGeometry(grid([
      [0, 0, 0],
      [0, 0, 0],
      [0, 0, 0],
    ]))
    // iso +5 and 0 include 0; iso −2.5 and −5 exclude it → 2 levels × 2 rows = 4 spans.
    expect(geo.polygonCount).toBe(4)
    for (const r of rings(geo)) {
      // Only the two shallowest colours appear.
      expect([SEA_BAND_LEVELS[0].rgba, SEA_BAND_LEVELS[1].rgba]).toContainEqual(r.rgba)
    }
  })

  it('island-in-ocean → boundary cells add marching polygons (non-rectangular rings)', () => {
    const geo = buildSeaBandGeometry(grid([
      [-100, -100, -100, -100, -100],
      [-100, 100, 100, 100, -100],
      [-100, 100, 100, 100, -100],
      [-100, 100, 100, 100, -100],
      [-100, -100, -100, -100, -100],
    ]))
    expect(geo.polygonCount).toBeGreaterThan(0)
    // At least one ring is not a 5-vertex rectangle (a boundary marching walk).
    const counts = rings(geo).map((r) => r.count)
    expect(counts.some((c) => c !== 5)).toBe(true)
  })

  it('ocean flush to the grid edge → a span reaches the exact world bound (x=0)', () => {
    const geo = buildSeaBandGeometry(grid([
      [-100, -100, 100],
      [-100, -100, 100],
      [-100, -100, 100],
    ]))
    let minX = Infinity
    for (let k = 0; k < geo.fillPositions.length; k += 2) minX = Math.min(minX, geo.fillPositions[k])
    expect(minX).toBe(0)
  })
})

describe('seaFillAlpha ladder (N3)', () => {
  it('full ≤ +1, fades +1…+3, off past +3', () => {
    expect(seaFillAlpha(-2)).toBe(1)
    expect(seaFillAlpha(1)).toBe(1)
    expect(seaFillAlpha(2)).toBe(0.6)
    expect(seaFillAlpha(3)).toBe(0.3)
    expect(seaFillAlpha(3.5)).toBe(0)
    expect(seaFillAlpha(6)).toBe(0)
  })
})

describe('downsampleDemGrid (DEM vector source)', () => {
  it('factor 1 is identity + anchors endpoints on the world bounds', () => {
    const g = downsampleDemGrid([1, 2, 3, 4, 5, 6, 7, 8, 9], 3, 3, 1, 12800, 12800)
    expect(g.cols).toBe(3)
    expect(g.rows).toBe(3)
    expect([...g.data]).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9])
    expect(g.maxElevM).toBe(9)
    expect(g.originX).toBe(0)
    // (cols−1) spacing → last sample lands exactly on 12800 (no edge sliver).
    expect(g.cellX).toBe(12800 / 2)
    expect(g.cellY).toBe(12800 / 2)
  })

  it('box-averages a constant field to the constant (fresh buffer, not the source)', () => {
    const src = new Float32Array(64).fill(7)
    const g = downsampleDemGrid(src, 8, 8, DEM_VECTOR_GRID_FACTOR, 100, 100)
    expect(demGridDims(8, 8, DEM_VECTOR_GRID_FACTOR)).toEqual({ cols: 2, rows: 2 })
    expect([...g.data]).toEqual([7, 7, 7, 7])
    expect(g.maxElevM).toBe(7)
    expect(g.data).not.toBe(src) // fresh buffer — never the live meters cache
  })
})
