// T-090.5.4 — Sea-band geometry (A3 DrawSea analogue): nested hypsometric fills over the
// downsampled DEM grid (demGrid.ts). Pure + worker-safe (no deck.gl/DOM); the world-objects
// worker runs it and ships transferable typed arrays. Layer styling lives in seaBandLayer.ts;
// the only style here is the SEA_BAND_LEVELS color table + the seaFillAlpha fade ladder.
//
// Model: for each level (inside = elevation ≤ iso, since sea is low ground) emit a fill of
// every cell at/under that iso, nested largest→smallest and appended in that order. Coplanar
// polygons in one SolidPolygonLayer paint in data order (LEQUAL depth), so the deepest level
// draws last and wins its region → a proper depth-band tint. Vertex alpha stays 255; the whole
// layer's translucency comes from seaFillAlpha(zoom) (layer opacity), so overlap never mixes
// colors — only the innermost band shows per pixel.
//
// Fill construction (per level): a full-inside cell (all 4 corners ≤ iso) is merged with its
// row-neighbours into one span rectangle (RLE) — reusing forestMassFromCorners here would emit
// one quad PER cell and explode on the ocean interior (~1M+ quads). Only boundary cells (some
// corners in, some out) get a marching-squares perimeter walk. All-out cells break the run.

import type { DemVectorGrid } from './demGrid'

/** One hypsometric sea level: fill everything at/under `iso` metres with `rgba`. Ordered
 *  shallow→deep; deeper rows draw later (on top) so their colour wins the nested overlap.
 *  Vertex alpha is provisional-opaque — the fade ladder (seaFillAlpha) carries translucency. */
export interface SeaBandLevel {
  iso: number
  rgba: [number, number, number, number]
}

/** Provisional hypsometric palette (operator visual pass tunes). Shallow (+5 faint shore
 *  tint) → waterline → deep. Alpha 255: overlap is painter-order opaque, layer opacity fades
 *  the whole band (see file header). */
export const SEA_BAND_LEVELS: SeaBandLevel[] = [
  { iso: 5, rgba: [126, 158, 178, 255] }, // ≤ +5 m — A3 ±5 shore tint (also tints low land)
  { iso: 0, rgba: [72, 118, 160, 255] }, // ≤ 0 m — waterline
  { iso: -2.5, rgba: [48, 96, 140, 255] }, // shallow
  { iso: -5, rgba: [30, 70, 120, 255] }, // deep
]

/** Sea-band fill layer opacity by deckZoom — discrete steps so the memo keys on the band, not
 *  raw zoom (T-057). N3: fill on ≤ +1, fade +1…+3, off past +3 (SEA_FILL_MAX_ZOOM). */
export function seaFillAlpha(deckZoom: number): number {
  if (deckZoom <= 1) return 1
  if (deckZoom <= 2) return 0.6
  if (deckZoom <= 3) return 0.3
  return 0
}

/** Sea-band geometry for one grid — Deck binary form (transferable). fillPositions = closed
 *  rings ([x,y]·vertex, first repeated last, `_normalize:false` contract); fillStartIndices =
 *  per-ring start VERTEX index; fillColors = RGBA (uint8) per vertex, aligned to fillPositions. */
export interface SeaBandGeometry {
  fillPositions: Float32Array
  fillStartIndices: Uint32Array
  fillColors: Uint8Array
  polygonCount: number
}

export const EMPTY_SEA_BAND: SeaBandGeometry = {
  fillPositions: new Float32Array(0),
  fillStartIndices: new Uint32Array(0),
  fillColors: new Uint8Array(0),
  polygonCount: 0,
}

interface Pt {
  x: number
  y: number
}

interface Corner {
  v: number
  inside: boolean
  x: number
  y: number
}

/** Linear iso crossing on the edge a→b (states differ ⇒ denominator ≠ 0). */
function crossing(a: Corner, b: Corner, iso: number): Pt {
  const t = (iso - a.v) / (b.v - a.v)
  return { x: a.x + t * (b.x - a.x), y: a.y + t * (b.y - a.y) }
}

/**
 * Build the sea-band fill geometry. Levels are processed shallow→deep and appended in that
 * order (nested painter's order). Inside test is `elev ≤ iso`.
 */
export function buildSeaBandGeometry(grid: DemVectorGrid): SeaBandGeometry {
  const { data, cols, rows, cellX, cellY, originX, originY } = grid
  if (cols < 2 || rows < 2) return EMPTY_SEA_BAND

  const positions: number[] = []
  const startIndices: number[] = []
  const colors: number[] = []
  let vertexCount = 0
  let polygonCount = 0

  const emitRing = (pts: Pt[], rgba: [number, number, number, number]): void => {
    if (pts.length < 3) return
    startIndices.push(vertexCount)
    for (const p of pts) {
      positions.push(p.x, p.y)
      colors.push(rgba[0], rgba[1], rgba[2], rgba[3])
    }
    // Close the loop (SolidPolygonLayer _normalize:false contract).
    positions.push(pts[0].x, pts[0].y)
    colors.push(rgba[0], rgba[1], rgba[2], rgba[3])
    vertexCount += pts.length + 1
    polygonCount++
  }

  /** Boundary cell → inside (≤ iso) perimeter polygon, forestMass walk flipped to ≤ with the
   *  saddle split when the centre is OUTSIDE (centre > iso). */
  const emitBoundaryCell = (
    c00: Corner,
    c10: Corner,
    c11: Corner,
    c01: Corner,
    iso: number,
    rgba: [number, number, number, number],
  ): void => {
    const corners = [c00, c10, c11, c01]
    const saddle =
      c00.inside === c11.inside && c10.inside === c01.inside && c00.inside !== c10.inside
    if (saddle && (c00.v + c10.v + c11.v + c01.v) / 4 > iso) {
      // Centre outside → the two inside corners are disconnected: a triangle around each.
      for (let k = 0; k < 4; k++) {
        const c = corners[k]
        if (!c.inside) continue
        const prev = corners[(k + 3) % 4]
        const next = corners[(k + 1) % 4]
        emitRing([{ x: c.x, y: c.y }, crossing(c, next, iso), crossing(c, prev, iso)], rgba)
      }
      return
    }
    const walk: Pt[] = []
    for (let k = 0; k < 4; k++) {
      const a = corners[k]
      const b = corners[(k + 1) % 4]
      if (a.inside) walk.push({ x: a.x, y: a.y })
      if (a.inside !== b.inside) walk.push(crossing(a, b, iso))
    }
    emitRing(walk, rgba)
  }

  for (const level of SEA_BAND_LEVELS) {
    const { iso, rgba } = level
    for (let j = 0; j < rows - 1; j++) {
      const y0 = originY + j * cellY
      const y1 = y0 + cellY
      let runStartI = -1 // first column of the current full-inside run
      const flushRun = (endI: number): void => {
        if (runStartI < 0) return
        const x0 = originX + runStartI * cellX
        const x1 = originX + (endI + 1) * cellX
        emitRing([{ x: x0, y: y0 }, { x: x1, y: y0 }, { x: x1, y: y1 }, { x: x0, y: y1 }], rgba)
        runStartI = -1
      }
      for (let i = 0; i < cols - 1; i++) {
        const v00 = data[j * cols + i]
        const v10 = data[j * cols + i + 1]
        const v11 = data[(j + 1) * cols + i + 1]
        const v01 = data[(j + 1) * cols + i]
        const in00 = v00 <= iso
        const in10 = v10 <= iso
        const in11 = v11 <= iso
        const in01 = v01 <= iso
        const insideCount = (in00 ? 1 : 0) + (in10 ? 1 : 0) + (in11 ? 1 : 0) + (in01 ? 1 : 0)
        if (insideCount === 4) {
          if (runStartI < 0) runStartI = i
          continue
        }
        flushRun(i - 1)
        if (insideCount === 0) continue
        const x0 = originX + i * cellX
        const x1 = x0 + cellX
        emitBoundaryCell(
          { v: v00, inside: in00, x: x0, y: y0 },
          { v: v10, inside: in10, x: x1, y: y0 },
          { v: v11, inside: in11, x: x1, y: y1 },
          { v: v01, inside: in01, x: x0, y: y1 },
          iso,
          rgba,
        )
      }
      flushRun(cols - 2)
    }
  }

  return {
    fillPositions: Float32Array.from(positions),
    fillStartIndices: Uint32Array.from(startIndices),
    fillColors: Uint8Array.from(colors),
    polygonCount,
  }
}
