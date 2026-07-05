// T-090.5.4 — Contour segment extraction + the DEM pyramid reduction it uses. Inside test is
// corner ≥ level; positive levels only (bathymetry is the sea band's job).
import { describe, it, expect } from 'vitest'
import { contourGridReductions, contourLevels, contourSegments } from './contours'
import { reduceGrid2x, type DemVectorGrid } from './demGrid'

/** DemVectorGrid from a row-major 2-D array, unit cells. */
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

function segCount(f: Float32Array): number {
  return f.length / 4
}

describe('contourLevels', () => {
  it('positive multiples of interval up to maxElev (never the 0 line)', () => {
    expect(contourLevels(20, 100)).toEqual([20, 40, 60, 80, 100])
    expect(contourLevels(20, 95)).toEqual([20, 40, 60, 80])
    expect(contourLevels(50, 375)).toEqual([50, 100, 150, 200, 250, 300, 350])
  })
  it('empty when interval ≤ 0 or maxElev below the first level', () => {
    expect(contourLevels(0, 100)).toEqual([])
    expect(contourLevels(20, -5)).toEqual([])
    expect(contourLevels(20, 10)).toEqual([])
  })
})

describe('contourGridReductions (plan R8 pyramid)', () => {
  it('coarse intervals march a coarser grid', () => {
    expect(contourGridReductions(100)).toBe(2)
    expect(contourGridReductions(50)).toBe(1)
    expect(contourGridReductions(20)).toBe(0)
    expect(contourGridReductions(10)).toBe(0)
  })
})

describe('contourSegments', () => {
  it('linear vertical ramp → one horizontal crossing band per level, cols−1 segments each', () => {
    // 11 rows × 3 cols, height = row·10 (0…100). Each level 20/40/60/80/100 crosses exactly
    // one cell-row band → 2 segments (cols−1) per level → 10 total.
    const rows2d = Array.from({ length: 11 }, (_, j) => [j * 10, j * 10, j * 10])
    const g = grid(rows2d)
    const seg = contourSegments(g, contourLevels(20, g.maxElevM))
    expect(segCount(seg)).toBe(10)
  })

  it('interpolates the crossing position linearly', () => {
    // Values 10 (row1) and 20 (row2) straddle level 15 at t=0.5 → y=1.5.
    const g = grid([
      [0, 0],
      [10, 10],
      [20, 20],
    ])
    const seg = contourSegments(g, [15])
    expect(segCount(seg)).toBe(1)
    // Both endpoints of the single horizontal segment sit at y=1.5.
    expect(seg[1]).toBeCloseTo(1.5, 6)
    expect(seg[3]).toBeCloseTo(1.5, 6)
  })

  it('closed contour on a cone → every endpoint has even degree (clean loop topology)', () => {
    // Steep cone so the level-50 isoline (d=2.0) stays clear of the grid edge (min edge d=3) —
    // a fully-interior closed loop, so every crossing is shared by two cells (degree 2).
    const N = 7
    const rows2d: number[][] = []
    for (let j = 0; j < N; j++) {
      const row: number[] = []
      for (let i = 0; i < N; i++) {
        const d = Math.hypot(i - 3, j - 3)
        row.push(100 - d * 25)
      }
      rows2d.push(row)
    }
    const seg = contourSegments(grid(rows2d), [50])
    expect(segCount(seg)).toBeGreaterThan(0)
    const degree = new Map<string, number>()
    for (let k = 0; k < seg.length; k += 4) {
      for (const [x, y] of [[seg[k], seg[k + 1]], [seg[k + 2], seg[k + 3]]]) {
        const key = `${x.toFixed(4)},${y.toFixed(4)}`
        degree.set(key, (degree.get(key) ?? 0) + 1)
      }
    }
    for (const d of degree.values()) expect(d % 2).toBe(0)
  })

  it('single sweep marches only crossing levels (flat cell → no segments)', () => {
    const seg = contourSegments(grid([
      [50, 50],
      [50, 50],
    ]), [20, 40, 60])
    expect(segCount(seg)).toBe(0)
  })

  it('empty for no levels', () => {
    expect(contourSegments(grid([[1, 2], [3, 4]]), []).length).toBe(0)
  })
})

describe('reduceGrid2x (contour pyramid)', () => {
  it('2×2-block averages and doubles the cell size', () => {
    const g = grid([
      [0, 2, 4, 6],
      [0, 2, 4, 6],
      [8, 10, 12, 14],
      [8, 10, 12, 14],
    ])
    const r = reduceGrid2x(g)
    expect(r.cols).toBe(2)
    expect(r.rows).toBe(2)
    // Top-left 2×2 block (0,2,0,2) → 1; next (4,6,4,6) → 5; bottom blocks 9 and 13.
    expect([...r.data]).toEqual([1, 5, 9, 13])
    expect(r.cellX).toBe(2)
    expect(r.cellY).toBe(2)
    expect(r.maxElevM).toBe(13)
  })
})
