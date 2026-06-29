/**
 * DEM elevation sampling — must match T-091.1 sampleElevation contract.
 * Ref: t091_0_dem_tile_export.md §Mathematical verification
 */

/** uint16-linear PNG → meters ASL (Bohemia Terrain Creation Tool). */
export function uint16ToMeters(u16, minM, maxM) {
  return minM + (u16 / 65535) * (maxM - minM);
}

/**
 * World meters (x, z) → continuous pixel coords on heightmap.
 * Origin: worldBounds [0,0,maxX,maxY]; +x east, +z north (Arma/editor y).
 */
export function worldToPixel(x, z, manifest) {
  const [minX, minY, maxX, maxY] = manifest.worldBounds;
  const wM = maxX - minX;
  const hM = maxY - minY;
  let u = (x - minX) / wM;
  let v = (z - minY) / hM;
  const flip = manifest.dem?.axisFlip ?? {};
  if (flip.x) u = 1 - u;
  if (flip.z) v = 1 - v;
  const { widthPx, heightPx } = manifest.dem;
  return {
    u,
    v,
    px: u * (widthPx - 1),
    py: v * (heightPx - 1),
  };
}

function readUint16BE(data, byteOffset) {
  return (data[byteOffset] << 8) | data[byteOffset + 1];
}

/** Bilinear sample uint16 raster (row-major, width × height). */
export function bilinearSampleUint16(raster, width, height, px, py) {
  const x0 = Math.floor(px);
  const y0 = Math.floor(py);
  const x1 = Math.min(x0 + 1, width - 1);
  const y1 = Math.min(y0 + 1, height - 1);
  const fx = px - x0;
  const fy = py - y0;
  const i = (y, x) => raster[y * width + x];
  const v00 = i(y0, x0);
  const v10 = i(y0, x1);
  const v01 = i(y1, x0);
  const v11 = i(y1, x1);
  const top = v00 * (1 - fx) + v10 * fx;
  const bot = v01 * (1 - fx) + v11 * fx;
  return top * (1 - fy) + bot * fy;
}

/**
 * Decode 16-bit grayscale PNG buffer to Uint16 raster (width × height).
 * Uses PNG IHDR + IDAT; supports bitDepth 16, colorType 0 (grayscale).
 */
export function decodeUint16GrayscalePng(pngBuffer) {
  // Minimal validation — full decode via pngjs in caller
  throw new Error('Use decodeUint16GrayscalePngFromPngjs');
}

/**
 * @param {import('pngjs').PNG} png parsed by pngjs with `{ skipRescale: true }`.
 * pngjs exposes the bit depth as `.depth` (not `.bitDepth`), always expands grayscale
 * to RGBA, and with skipRescale returns a Uint16Array of true 16-bit samples. Reading
 * without skipRescale lossily rescales 16-bit -> 8-bit (Uint8 Buffer), which would
 * collapse the elevation precision below the anchor threshold — reject that here.
 */
export function rasterFromPngjs(png) {
  const depth = png.bitDepth ?? png.depth;
  if (depth !== 16) {
    throw new Error(`DEM must be 16-bit PNG; got depth=${depth}`);
  }
  if (png.colorType !== 0 && png.colorType !== 4) {
    throw new Error(`DEM must be grayscale; colorType=${png.colorType}`);
  }
  const { width, height, data } = png;
  if (data.BYTES_PER_ELEMENT !== 2) {
    throw new Error('DEM raster not 16-bit: read the PNG with { skipRescale: true }');
  }
  const channels = data.length / (width * height); // grayscale expanded to RGBA -> 4
  const raster = new Float64Array(width * height);
  for (let i = 0; i < width * height; i++) {
    raster[i] = data[i * channels]; // channel 0 (gray)
  }
  return { raster, width, height };
}

export function sampleElevationMeters(x, z, manifest, raster, width, height) {
  const { px, py } = worldToPixel(x, z, manifest);
  if (px < 0 || py < 0 || px > width - 1 || py > height - 1) {
    throw new Error(`Anchor (${x}, ${z}) outside DEM raster`);
  }
  const u16 = bilinearSampleUint16(raster, width, height, px, py);
  return uint16ToMeters(
    u16,
    manifest.dem.heightRangeMinM,
    manifest.dem.heightRangeMaxM,
  );
}
