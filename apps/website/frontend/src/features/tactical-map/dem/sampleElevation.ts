// Pure DEM sampling math — faithful port of
// packages/tbd-schema/scripts/lib/dem-sample.mjs (the verify-terrain-strict gate).
// Regression rule: if this math changes, update dem-sample.mjs in the same PR or the
// anchor gate will diverge. No module state here — see DemController for lifecycle.

import type { TerrainManifest } from './terrainManifest'

/** uint16-linear sample → meters ASL (Bohemia Terrain Creation Tool encoding). */
export function uint16ToMeters(u16: number, minM: number, maxM: number): number {
  return minM + (u16 / 65535) * (maxM - minM)
}

/**
 * World meters (x, z) → continuous pixel coords on the heightmap.
 * Origin worldBounds [0,0,maxX,maxY]; +x east, +z north (Arma/editor y).
 */
export function worldToPixel(
  x: number,
  z: number,
  manifest: TerrainManifest,
): { u: number; v: number; px: number; py: number } {
  const [minX, minY, maxX, maxY] = manifest.worldBounds
  const wM = maxX - minX
  const hM = maxY - minY
  let u = (x - minX) / wM
  let v = (z - minY) / hM
  const flip = manifest.dem.axisFlip ?? {}
  if (flip.x) u = 1 - u
  if (flip.z) v = 1 - v
  const { widthPx, heightPx } = manifest.dem
  return { u, v, px: u * (widthPx - 1), py: v * (heightPx - 1) }
}

/**
 * Bilinear sample of a row-major (width × height) raster. Generic over array-likes:
 * a Float64Array of uint16 samples (anchor tests) or the Float32Array meters cache
 * (runtime) — uint16ToMeters is affine, so bilinear-on-meters == bilinear-on-uint16-then-convert.
 */
export function bilinearSample(
  raster: ArrayLike<number>,
  width: number,
  height: number,
  px: number,
  py: number,
): number {
  const x0 = Math.floor(px)
  const y0 = Math.floor(py)
  const x1 = Math.min(x0 + 1, width - 1)
  const y1 = Math.min(y0 + 1, height - 1)
  const fx = px - x0
  const fy = py - y0
  const at = (y: number, xx: number) => raster[y * width + xx]
  const v00 = at(y0, x0)
  const v10 = at(y0, x1)
  const v01 = at(y1, x0)
  const v11 = at(y1, x1)
  const top = v00 * (1 - fx) + v10 * fx
  const bot = v01 * (1 - fx) + v11 * fx
  return top * (1 - fy) + bot * fy
}

/**
 * Faithful port of dem-sample.mjs sampleElevationMeters — bilinear on the uint16 grid,
 * THEN convert to meters. Throws on out-of-bounds (anchor-verify contract). The public
 * DemController.sampleElevation clamps first so the editor never throws.
 */
export function sampleElevationMeters(
  x: number,
  z: number,
  manifest: TerrainManifest,
  raster: ArrayLike<number>,
  width: number,
  height: number,
): number {
  const { px, py } = worldToPixel(x, z, manifest)
  if (px < 0 || py < 0 || px > width - 1 || py > height - 1) {
    throw new Error(`(${x}, ${z}) outside DEM raster`)
  }
  const u16 = bilinearSample(raster, width, height, px, py)
  return uint16ToMeters(u16, manifest.dem.heightRangeMinM, manifest.dem.heightRangeMaxM)
}
