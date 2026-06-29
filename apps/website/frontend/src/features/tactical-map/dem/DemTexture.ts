// DEM PNG decode + meters cache (T-091.1). Uses pngjs (production dependency) with
// { skipRescale: true } — the same decoder as verify-terrain-strict / dem-sample.mjs,
// so there is no decode drift. createImageBitmap is intentionally NOT used (it is 8-bit
// lossy for a 16-bit grayscale PNG). Browser bundling resolves pngjs via its `browser`
// package field; the `npm run build` gate is authoritative.

import { Buffer } from 'buffer'
import { PNG } from 'pngjs'
import type { TerrainManifest } from './terrainManifest'
import { uint16ToMeters } from './sampleElevation'

export interface DemRaster {
  /** uint16 samples widened to f64 (matches dem-sample.mjs rasterFromPngjs). */
  raster: Float64Array
  width: number
  height: number
}

/**
 * Structural view of a pngjs-parsed image. @types/pngjs omits `.depth` and `.colorType`,
 * and with { skipRescale: true } `.data` is actually a Uint16Array — so we read the shape
 * we need directly rather than fighting the published types.
 */
interface PngLike {
  width: number
  height: number
  data: { length: number; BYTES_PER_ELEMENT?: number; [i: number]: number }
  depth?: number
  bitDepth?: number
  colorType?: number
}

/**
 * pngjs (parsed with { skipRescale: true }) → Float64Array of true 16-bit samples.
 * pngjs exposes bit depth as `.depth` (not `.bitDepth`), expands grayscale to RGBA, and
 * with skipRescale returns a Uint16Array of real 16-bit values. Reading without skipRescale
 * lossily rescales 16-bit → 8-bit (Uint8 Buffer) and collapses elevation precision — reject that.
 */
export function rasterFromPngjs(pngInput: PNG): DemRaster {
  const png = pngInput as unknown as PngLike
  const depth = png.bitDepth ?? png.depth
  if (depth !== 16) {
    throw new Error(`DEM must be 16-bit PNG; got depth=${depth}`)
  }
  if (png.colorType !== 0 && png.colorType !== 4) {
    throw new Error(`DEM must be grayscale; colorType=${png.colorType}`)
  }
  const { width, height, data } = png
  if (data.BYTES_PER_ELEMENT !== 2) {
    throw new Error('DEM raster not 16-bit: read the PNG with { skipRescale: true }')
  }
  const channels = data.length / (width * height) // grayscale expanded to RGBA -> 4
  const raster = new Float64Array(width * height)
  for (let i = 0; i < width * height; i++) {
    raster[i] = data[i * channels] // channel 0 (gray)
  }
  return { raster, width, height }
}

/**
 * Decode a 16-bit grayscale DEM PNG buffer to a Float64 raster. pngjs needs a real Buffer
 * (its parser calls Buffer methods like readUInt32BE); the browser has no global Buffer, so
 * we wrap via the `buffer` polyfill (Node's Buffer in tests). createImageBitmap is avoided —
 * it would 8-bit-quantize the 16-bit samples.
 */
export function decodeDemPng(buffer: Uint8Array): DemRaster {
  const png = PNG.sync.read(Buffer.from(buffer), {
    skipRescale: true,
  } as Parameters<typeof PNG.sync.read>[1])
  return rasterFromPngjs(png)
}

/** Precompute the Float32Array meters cache (one uint16ToMeters per pixel). */
export function buildMetersCache(
  raster: ArrayLike<number>,
  manifest: TerrainManifest,
): Float32Array {
  const { heightRangeMinM, heightRangeMaxM } = manifest.dem
  const out = new Float32Array(raster.length)
  for (let i = 0; i < raster.length; i++) {
    out[i] = uint16ToMeters(raster[i], heightRangeMinM, heightRangeMaxM)
  }
  return out
}
