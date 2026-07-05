// T-090.5.4 — DEM vector-source grid: the downsampled meters grid the sea-band + contour
// geometry marches over. Pure + worker-safe (no dem/ imports — pngjs/Buffer stay main-thread
// only): the main thread box-averages DemController's 6400² Float32 meters cache into a fresh
// ~1600² buffer (never the live cache — transferring that would detach hillshade +
// sampleElevation) and ships it to the world-objects worker once per terrain.
//
// Sampling convention: output sample (i,j) sits at world (originX + i·cellX, originY + j·cellY)
// with cellX = worldWidthM/(cols−1), so the first/last samples land exactly on the terrain
// bounds (0 / 12800) — no uncovered sliver at map edges. Each sample averages a factor×factor
// source window centered on its position (≤ half-native-pixel phase error, irrelevant at 8 m
// cells).

/** A regular meters-ASL grid in world space (row-major, rows of `cols` samples). */
export interface DemVectorGrid {
  data: Float32Array
  cols: number
  rows: number
  /** World meters between column samples. */
  cellX: number
  /** World meters between row samples. */
  cellY: number
  originX: number
  originY: number
  /** Max elevation in the grid (drives the contour level list). */
  maxElevM: number
}

/** Downsample factor for the base vector grid: 6400² @ 2 m/px → 1600² @ 8 m cells. */
export const DEM_VECTOR_GRID_FACTOR = 4

/** Output dims for a source raster + factor (used by the banded downsampler + tests). */
export function demGridDims(
  width: number,
  height: number,
  factor: number,
): { cols: number; rows: number } {
  return {
    cols: Math.max(2, Math.round(width / factor)),
    rows: Math.max(2, Math.round(height / factor)),
  }
}

/** Per-output-index source windows [x0, x1) centered on the sample position. */
function sourceWindows(outCount: number, srcCount: number, factor: number): Uint32Array {
  const win = new Uint32Array(2 * outCount)
  const half = factor / 2
  for (let i = 0; i < outCount; i++) {
    const center = outCount > 1 ? (i * (srcCount - 1)) / (outCount - 1) : (srcCount - 1) / 2
    let a = Math.round(center - half)
    let b = Math.round(center + half)
    if (a < 0) a = 0
    if (b > srcCount) b = srcCount
    if (b <= a) b = Math.min(srcCount, a + 1)
    win[2 * i] = a
    win[2 * i + 1] = b
  }
  return win
}

/**
 * Box-average one band of output rows [jStart, jEnd) into `out` (a cols×rows Float32Array).
 * Returns the band's max elevation. The store calls this in row bands with yieldToUi between
 * them so the one-time ~40–80 ms full-grid pass never blocks a frame.
 */
export function downsampleDemGridBand(
  data: ArrayLike<number>,
  width: number,
  height: number,
  factor: number,
  out: Float32Array,
  jStart: number,
  jEnd: number,
): number {
  const { cols, rows } = demGridDims(width, height, factor)
  const colWin = sourceWindows(cols, width, factor)
  const rowWin = sourceWindows(rows, height, factor)
  let max = -Infinity
  for (let j = jStart; j < jEnd; j++) {
    const y0 = rowWin[2 * j]
    const y1 = rowWin[2 * j + 1]
    for (let i = 0; i < cols; i++) {
      const x0 = colWin[2 * i]
      const x1 = colWin[2 * i + 1]
      let sum = 0
      for (let y = y0; y < y1; y++) {
        const rowBase = y * width
        for (let x = x0; x < x1; x++) sum += data[rowBase + x]
      }
      const v = sum / ((y1 - y0) * (x1 - x0))
      out[j * cols + i] = v
      if (v > max) max = v
    }
  }
  return max
}

/** One-shot downsample (tests + non-yielding callers). Allocates a fresh buffer. */
export function downsampleDemGrid(
  data: ArrayLike<number>,
  width: number,
  height: number,
  factor: number,
  worldWidthM: number,
  worldHeightM: number,
): DemVectorGrid {
  const { cols, rows } = demGridDims(width, height, factor)
  const out = new Float32Array(cols * rows)
  const maxElevM = downsampleDemGridBand(data, width, height, factor, out, 0, rows)
  return {
    data: out,
    cols,
    rows,
    cellX: worldWidthM / (cols - 1),
    cellY: worldHeightM / (rows - 1),
    originX: 0,
    originY: 0,
    maxElevM,
  }
}

/**
 * 2× reduction for the per-interval contour pyramid (coarse intervals march coarse grids —
 * plan R8). Sample i2 averages the 2×2 source block at 2·i2; cell size doubles. The far edge
 * can fall one source cell short — acceptable for the 100/50 m interval bands this feeds.
 */
export function reduceGrid2x(grid: DemVectorGrid): DemVectorGrid {
  const cols = Math.max(2, Math.ceil(grid.cols / 2))
  const rows = Math.max(2, Math.ceil(grid.rows / 2))
  const out = new Float32Array(cols * rows)
  let max = -Infinity
  for (let j = 0; j < rows; j++) {
    const sj = Math.min(2 * j, grid.rows - 1)
    const sj1 = Math.min(sj + 1, grid.rows - 1)
    for (let i = 0; i < cols; i++) {
      const si = Math.min(2 * i, grid.cols - 1)
      const si1 = Math.min(si + 1, grid.cols - 1)
      const v =
        (grid.data[sj * grid.cols + si] +
          grid.data[sj * grid.cols + si1] +
          grid.data[sj1 * grid.cols + si] +
          grid.data[sj1 * grid.cols + si1]) /
        4
      out[j * cols + i] = v
      if (v > max) max = v
    }
  }
  return {
    data: out,
    cols,
    rows,
    cellX: grid.cellX * 2,
    cellY: grid.cellY * 2,
    originX: grid.originX,
    originY: grid.originY,
    maxElevM: max,
  }
}
