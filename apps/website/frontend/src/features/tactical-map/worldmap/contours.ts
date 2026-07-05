// T-090.5.4 — Contour geometry (A3 DrawCountlines analogue): iso polylines over the DEM grid
// (demGrid.ts). Pure + worker-safe (no deck.gl/DOM); the world-objects worker runs it and
// ships transferable typed arrays. Segments only — no fill (the sea band owns the 0 m line and
// everything under it; contours are the positive relief).
//
// Positive levels only (interval, 2·interval, … ≤ maxElev): bathymetry is the sea band's job
// (A3 parity), so the 0 m and negative isolines are deliberately absent. Output is the
// forest-outline wire form: interleaved [x0,y0,x1,y1] per segment, drawn by a LineLayer.

import type { DemVectorGrid } from './demGrid'

/** Coarse intervals march a coarser grid (plan R8): factor is 2×-reductions of the 8 m base
 *  grid. 100 m → 4× (32 m), 50 m → 2× (16 m), 20/10 m → base (8 m). */
export function contourGridReductions(intervalM: number): number {
  if (intervalM >= 100) return 2
  if (intervalM >= 50) return 1
  return 0
}

/** Positive iso levels for an interval up to the grid's max elevation (never the 0 line). */
export function contourLevels(intervalM: number, maxElevM: number): number[] {
  const levels: number[] = []
  if (intervalM <= 0 || !Number.isFinite(maxElevM)) return levels
  for (let lv = intervalM; lv <= maxElevM; lv += intervalM) levels.push(lv)
  return levels
}

/** Cell corners + world box for one marching-squares cell. */
interface Cell {
  v00: number // BL
  v10: number // BR
  v11: number // TR
  v01: number // TL
  x0: number
  y0: number
  x1: number
  y1: number
}

/** Edge index → the two connected edges per marching-squares case (non-saddle). Edges:
 *  0=A bottom (c00–c10), 1=B right (c10–c11), 2=C top (c11–c01), 3=D left (c01–c00). Cases 5
 *  and 10 are saddles resolved by the cell centre (below), so their rows here are empty. */
const CASE_EDGES: ReadonlyArray<ReadonlyArray<readonly [number, number]>> = [
  [], // 0
  [[0, 3]], // 1  c00
  [[0, 1]], // 2  c10
  [[1, 3]], // 3  c00,c10
  [[1, 2]], // 4  c11
  [], // 5  saddle
  [[0, 2]], // 6  c10,c11
  [[2, 3]], // 7  all but c01
  [[2, 3]], // 8  c01
  [[0, 2]], // 9  c00,c01
  [], // 10 saddle
  [[1, 2]], // 11 all but c11
  [[1, 3]], // 12 c11,c01
  [[0, 1]], // 13 all but c10
  [[0, 3]], // 14 all but c00
  [], // 15
]

/** Linear iso crossing between two corners (caller guarantees they straddle `level`). */
function lerp(va: number, ax: number, ay: number, vb: number, bx: number, by: number, level: number): number[] {
  const t = (level - va) / (vb - va)
  return [ax + t * (bx - ax), ay + t * (by - ay)]
}

/** Crossing points on the 4 cell edges (undefined where the edge's corners don't straddle). */
function edgePoints(cell: Cell, level: number): (number[] | undefined)[] {
  const { v00, v10, v11, v01, x0, y0, x1, y1 } = cell
  const b0 = v00 >= level
  const b1 = v10 >= level
  const b2 = v11 >= level
  const b3 = v01 >= level
  return [
    b0 !== b1 ? lerp(v00, x0, y0, v10, x1, y0, level) : undefined, // A bottom
    b1 !== b2 ? lerp(v10, x1, y0, v11, x1, y1, level) : undefined, // B right
    b2 !== b3 ? lerp(v11, x1, y1, v01, x0, y1, level) : undefined, // C top
    b3 !== b0 ? lerp(v01, x0, y1, v00, x0, y0, level) : undefined, // D left
  ]
}

/** Saddle (case 5/10) edge pairs, chosen by whether the cell centre is inside. */
function saddleEdges(c: number, centerIn: boolean): ReadonlyArray<readonly [number, number]> {
  const connected: ReadonlyArray<readonly [number, number]> = [[0, 1], [2, 3]]
  const split: ReadonlyArray<readonly [number, number]> = [[0, 3], [1, 2]]
  // c=5 (c00,c11 in): centre-inside isolates the OUT corners → connected pairs; c=10 mirrors.
  if (c === 5) return centerIn ? connected : split
  return centerIn ? split : connected
}

/** March one cell at one level; append each segment's [x0,y0,x1,y1] to `seg`. */
function marchCell(cell: Cell, level: number, seg: number[]): void {
  const { v00, v10, v11, v01 } = cell
  const c =
    (v00 >= level ? 1 : 0) | (v10 >= level ? 2 : 0) | (v11 >= level ? 4 : 0) | (v01 >= level ? 8 : 0)
  if (c === 0 || c === 15) return
  const pts = edgePoints(cell, level)
  const segEdges =
    c === 5 || c === 10 ? saddleEdges(c, (v00 + v10 + v11 + v01) / 4 >= level) : CASE_EDGES[c]
  for (const [e0, e1] of segEdges) {
    const p = pts[e0]
    const q = pts[e1]
    if (p && q) seg.push(p[0], p[1], q[0], q[1])
  }
}

/**
 * Marching-squares isolines for many levels in ONE grid sweep. Per cell we take the corner
 * min/max once and only march the levels that actually cross it (`lo < level ≤ hi`) — O(cells
 * + crossings), not O(levels × cells). Inside test is `corner ≥ level`; saddles resolved by the
 * cell centre (see marchCell). Output is interleaved [x0,y0,x1,y1] per segment.
 */
export function contourSegments(grid: DemVectorGrid, levels: number[]): Float32Array {
  const { data, cols, rows, cellX, cellY, originX, originY } = grid
  const seg: number[] = []
  if (cols < 2 || rows < 2 || levels.length === 0) return new Float32Array(0)

  // Sorted levels so the per-cell scan can early-out past the cell max.
  const sorted = [...levels].sort((a, b) => a - b)

  for (let j = 0; j < rows - 1; j++) {
    const y0 = originY + j * cellY
    const y1 = y0 + cellY
    for (let i = 0; i < cols - 1; i++) {
      const v00 = data[j * cols + i]
      const v10 = data[j * cols + i + 1]
      const v11 = data[(j + 1) * cols + i + 1]
      const v01 = data[(j + 1) * cols + i]
      const lo = Math.min(v00, v10, v11, v01)
      const hi = Math.max(v00, v10, v11, v01)
      if (sorted[0] > hi) continue // no level reaches this cell
      const x0 = originX + i * cellX
      const cell: Cell = { v00, v10, v11, v01, x0, y0, x1: x0 + cellX, y1 }
      for (const level of sorted) {
        if (level <= lo) continue
        if (level > hi) break
        marchCell(cell, level, seg)
      }
    }
  }
  return Float32Array.from(seg)
}
