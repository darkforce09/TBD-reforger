#!/usr/bin/env -S node --max-old-space-size=8192
/**
 * export-water-to-png.mjs
 *
 * Converts Arma Reforger TBD water export data:
 *   - Global Water: bathymetry_mask.txt, bathymetry_depth.txt, water_meta.json, inland_water_meta.json
 *   - Inland Water Vectors: rivers.json, lakes.json, ponds.json
 *   - Legacy Export: TBD_WaterExport_mask.txt, depth.txt, meta.json, vectors.json
 *
 * Features:
 *   - Ground-truth discrete matrix decoding + continuous Catmull-Rom spline river rasterization
 *   - Organic lake and pond polygon scanline rendering with DEM heightfield bathymetric profiling
 *   - Sub-decimeter (0.1m/px) resolution support for Region of Interest (ROI) viewports
 *   - 3D bathymetric depth profiling without discrete stair-step contour slicing
 *   - Pure transparent land 32-bit RGBA water rendering
 *
 * Generates:
 *   1. everon-water-bathymetry.png (32-bit RGBA, transparent land, water colored by depth)
 *   2. everon-water-bathymetry-dark.png (24-bit RGB, dark charcoal land with subtle depth contours)
 *   3. everon-water-depth-16bit.png (16-bit Grayscale raw depth heightfield in decimeters)
 *   4. everon-water-mask.png (8-bit Grayscale classification mask: 0=Land, 1=Ocean, 2=Lake/Pond, 3=River)
 *   5. everon-water-preview.png (24-bit RGB downsampled preview)
 *
 * Usage:
 *   node apps/mod/tbd-export/tools/export-water-to-png.mjs [path_to_water_export] [options]
 *
 * Options:
 *   --export-dir <path>     Source export directory (default: /home/Samuel/Games/ArmaReforger-Base/TBD_Export/$tbd_framework:worlds/water)
 *   --out-dir <path>        Output directory (default: <export-dir>/images or <export-dir>)
 *   --mode <mode>           Mode: water | bathymetry | dark | depth16 | mask | preview | all (default: water)
 *   --terrain <name>        Terrain name prefix (default: everon)
 *   --dem <path>            Custom DEM 16-bit PNG or heightmap.txt path
 *   --inland-only           Process inland water files exclusively
 *   --res <mpp>             Target resolution in meters/pixel (default: from meta, e.g. 1.0 or 0.1 for high-res)
 *   --roi <minX,minZ,maxX,maxZ> Bounding box for Region of Interest rendering (e.g. 4000,5500,5500,7000)
 *   --no-vector-enhance     Disable continuous spline/polygon rasterization
 */

import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { resolve, join, dirname } from 'node:path';
import { deflateSync, inflateSync, crc32 as zlibCrc32 } from 'node:zlib';

function parseArgs() {
  const args = process.argv.slice(2);
  const options = {
    exportDir: '',
    outDir: '',
    mode: 'water',
    terrain: 'everon',
    inlandOnly: false,
    res: 0,
    roi: null,
    vectorEnhance: true,
    demPath: '',
  };

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === '--export-dir' && i + 1 < args.length) {
      options.exportDir = args[++i];
    } else if (arg === '--out-dir' && i + 1 < args.length) {
      options.outDir = args[++i];
    } else if (arg === '--dem' && i + 1 < args.length) {
      options.demPath = args[++i];
    } else if (arg === '--mode' && i + 1 < args.length) {
      options.mode = args[++i].toLowerCase();
    } else if (arg === '--terrain' && i + 1 < args.length) {
      options.terrain = args[++i];
    } else if (arg === '--inland-only') {
      options.inlandOnly = true;
    } else if (arg === '--res' && i + 1 < args.length) {
      options.res = parseFloat(args[++i]);
    } else if (arg === '--roi' && i + 1 < args.length) {
      const parts = args[++i].split(',').map((v) => parseFloat(v.trim()));
      if (parts.length === 4 && !parts.some(isNaN)) {
        options.roi = parts; // [minX, minZ, maxX, maxZ]
      }
    } else if (arg === '--no-vector-enhance') {
      options.vectorEnhance = false;
    } else if (!arg.startsWith('--') && !options.exportDir) {
      options.exportDir = arg;
    }
  }

  // Fallback defaults for export directory
  if (!options.exportDir) {
    const defaultCandidates = [
      '/home/Samuel/Games/ArmaReforger-Base/TBD_Export/$tbd_framework:worlds/water',
      '/home/Samuel/Games/ArmaReforger-Base/TBD_Export',
    ];
    for (const cand of defaultCandidates) {
      if (existsSync(cand)) {
        options.exportDir = cand;
        break;
      }
    }
    if (!options.exportDir) {
      options.exportDir = defaultCandidates[0];
    }
  }

  return options;
}

// --- PNG Low-Level Encoder ---

function computeCrc32(buffer) {
  if (typeof zlibCrc32 === 'function') {
    return zlibCrc32(buffer) >>> 0;
  }
  let c = 0xffffffff;
  for (let i = 0; i < buffer.length; i++) {
    c = (c >>> 8) ^ ((c ^ buffer[i]) & 0xff);
  }
  return (c ^ 0xffffffff) >>> 0;
}

function createChunk(typeStr, data) {
  const len = Buffer.allocUnsafe(4);
  len.writeUInt32BE(data.length, 0);
  const typeBuf = Buffer.from(typeStr, 'ascii');
  const crc = Buffer.allocUnsafe(4);
  const chunkHeaderAndData = Buffer.concat([typeBuf, data]);
  crc.writeUInt32BE(computeCrc32(chunkHeaderAndData), 0);
  return Buffer.concat([len, chunkHeaderAndData, crc]);
}

const PNG_SIGNATURE = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

function encodePng({ width, height, bitDepth, colorType, rawScanlines }) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = bitDepth;
  ihdr[9] = colorType;
  ihdr[10] = 0;
  ihdr[11] = 0;
  ihdr[12] = 0;

  const idat = deflateSync(rawScanlines, { level: 6 });
  return Buffer.concat([
    PNG_SIGNATURE,
    createChunk('IHDR', ihdr),
    createChunk('IDAT', idat),
    createChunk('IEND', Buffer.alloc(0)),
  ]);
}

// --- Color Interpolation & Palette ---

const OCEAN_STOPS = [
  { depth: 0.0, rgb: [160, 232, 242] },   // 0.0m: Caribbean Shore
  { depth: 2.0, rgb: [110, 215, 235] },   // 2.0m: Pale Turquoise
  { depth: 6.0, rgb: [65, 185, 222] },    // 6.0m: Vibrant Cyan-Blue
  { depth: 15.0, rgb: [45, 155, 210] },   // 15.0m: Coastal Teal-Blue
  { depth: 30.0, rgb: [35, 130, 195] },   // 30.0m: Cerulean Ocean Blue
  { depth: 60.0, rgb: [24, 98, 175] },    // 60.0m: Mid Shelf Blue
  { depth: 100.0, rgb: [16, 68, 145] },   // 100.0m: Deep Cobalt
  { depth: 150.0, rgb: [10, 42, 110] },   // 150.0m: Bathypelagic Navy
  { depth: 210.0, rgb: [4, 16, 65] },     // 210.0m: Midnight Abyss
];

const LAKE_STOPS = [
  { depth: 0.0, rgb: [95, 225, 205] },    // Shallow shoreline teal
  { depth: 2.0, rgb: [50, 195, 175] },    // Emerald teal
  { depth: 6.0, rgb: [30, 155, 140] },    // Deep teal
  { depth: 18.0, rgb: [16, 110, 100] },   // Alpine abyss
];

const RIVER_STOPS = [
  { depth: 0.0, rgb: [110, 205, 255] },   // Luminous sky blue
  { depth: 1.5, rgb: [60, 165, 250] },    // River azure
  { depth: 4.0, rgb: [30, 125, 225] },    // Deep channel
  { depth: 25.0, rgb: [15, 85, 180] },    // Torrent trench
];

const DARK_LAND_COLOR = [22, 26, 31];

function interpolatePalette(stops, depthM) {
  if (depthM <= stops[0].depth) return stops[0].rgb;
  const last = stops[stops.length - 1];
  if (depthM >= last.depth) return last.rgb;

  for (let i = 0; i < stops.length - 1; i++) {
    const s0 = stops[i];
    const s1 = stops[i + 1];
    if (depthM >= s0.depth && depthM <= s1.depth) {
      const t = (depthM - s0.depth) / (s1.depth - s0.depth);
      return [
        Math.round(s0.rgb[0] + t * (s1.rgb[0] - s0.rgb[0])),
        Math.round(s0.rgb[1] + t * (s1.rgb[1] - s0.rgb[1])),
        Math.round(s0.rgb[2] + t * (s1.rgb[2] - s0.rgb[2])),
      ];
    }
  }
  return last.rgb;
}

function getBathymetryColor(depthM, maskType) {
  if (maskType === 2) {
    return interpolatePalette(LAKE_STOPS, depthM);
  }
  if (maskType === 3) {
    return interpolatePalette(RIVER_STOPS, depthM);
  }
  return interpolatePalette(OCEAN_STOPS, depthM);
}

function getContourMultiplier(depthM, maskType) {
  if (maskType === 2 || maskType === 3) {
    return 1.0;
  }
  const intervals = [5, 10, 20, 50, 100, 150, 200];
  for (const interval of intervals) {
    const diff = Math.abs(depthM - Math.round(depthM / interval) * interval);
    if (diff < 0.18) {
      return 0.78;
    }
  }
  return 1.0;
}

// --- DEM Heightfield Engine ---

function loadDemHeightfield(customPath, waterDir) {
  const candidates = [
    customPath,
    join(waterDir, '../dem/everon-dem-16bit.png'),
    join(waterDir, '../terrain/everon-dem-16bit.png'),
    join(waterDir, 'everon-dem-16bit.png'),
    resolve(process.cwd(), 'packages/map-assets/everon/dem/everon-dem-16bit.png'),
    '/home/Samuel/Projects/TBD-Reforger/packages/map-assets/everon/dem/everon-dem-16bit.png',
  ].filter(Boolean);

  for (const demPath of candidates) {
    if (existsSync(demPath)) {
      try {
        const buffer = readFileSync(demPath);
        let pos = 8;
        let width = 0, height = 0;
        const idatChunks = [];

        while (pos < buffer.length) {
          const len = buffer.readUInt32BE(pos);
          const type = buffer.toString('ascii', pos + 4, pos + 8);
          const data = buffer.slice(pos + 8, pos + 8 + len);
          pos += 12 + len;

          if (type === 'IHDR') {
            width = data.readUInt32BE(0);
            height = data.readUInt32BE(4);
          } else if (type === 'IDAT') {
            idatChunks.push(data);
          } else if (type === 'IEND') {
            break;
          }
        }

        const raw = inflateSync(Buffer.concat(idatChunks));
        const stride = 1 + width * 2;
        const minM = -204.781;
        const maxM = 375.531;

        const getElev = (wx, wz) => {
          const px = Math.min(width - 1, Math.max(0, Math.round(wx / 2.0)));
          const py = Math.min(height - 1, Math.max(0, Math.round(wz / 2.0)));
          const o = py * stride + 1 + px * 2;
          const u16 = (raw[o] << 8) | raw[o + 1];
          return minM + (u16 / 65535.0) * (maxM - minM);
        };

        console.log(`Loaded DEM Heightfield (${width}x${height}) from ${demPath}`);
        return { width, height, getElev };
      } catch (err) {
        console.warn(`Warning: Could not decode DEM at ${demPath}:`, err.message);
      }
    }
  }
  return null;
}

// --- Spline & Polygon Vector Geometry Engine ---

function evaluateCatmullRom(p0, p1, p2, p3, t) {
  const t2 = t * t;
  const t3 = t2 * t;

  const v0x = (p2[0] - p0[0]) * 0.5;
  const v1x = (p3[0] - p1[0]) * 0.5;
  const x = (2 * p1[0] - 2 * p2[0] + v0x + v1x) * t3 + (-3 * p1[0] + 3 * p2[0] - 2 * v0x - v1x) * t2 + v0x * t + p1[0];

  const v0z = (p2[2] - p0[2]) * 0.5;
  const v1z = (p3[2] - p1[2]) * 0.5;
  const z = (2 * p1[2] - 2 * p2[2] + v0z + v1z) * t3 + (-3 * p1[2] + 3 * p2[2] - 2 * v0z - v1z) * t2 + v0z * t + p1[2];

  const v0y = (p2[1] - p0[1]) * 0.5;
  const v1y = (p3[1] - p1[1]) * 0.5;
  const y = (2 * p1[1] - 2 * p2[1] + v0y + v1y) * t3 + (-3 * p1[1] + 3 * p2[1] - 2 * v0y - v1y) * t2 + v0y * t + p1[1];

  const tx = 3 * (2 * p1[0] - 2 * p2[0] + v0x + v1x) * t2 + 2 * (-3 * p1[0] + 3 * p2[0] - 2 * v0x - v1x) * t + v0x;
  const tz = 3 * (2 * p1[2] - 2 * p2[2] + v0z + v1z) * t2 + 2 * (-3 * p1[2] + 3 * p2[2] - 2 * v0z - v1z) * t + v0z;
  const tLen = Math.hypot(tx, tz) || 1.0;

  return {
    pos: [x, y, z],
    tangent: [tx / tLen, 0, tz / tLen],
    normal: [-tz / tLen, 0, tx / tLen],
  };
}

function rasterizeSplineRivers({ rivers, mask, depth, W, H, minWorldX, minWorldZ, sx, sz }) {
  if (!rivers || !Array.isArray(rivers)) return 0;
  let drawnSegments = 0;

  for (const river of rivers) {
    // Support splinePoints: [[x,y,z], ...] OR nodes: [{ pos: [x,y,z] }, ...]
    const pts = river.splinePoints || river.nodes?.map((n) => (Array.isArray(n) ? n : n.pos)) || [];
    if (pts.length < 2) continue;

    const defaultWidth = river.widthM || river.averageWidthM || 6.0;
    const nLen = pts.length;

    for (let i = 0; i < nLen - 1; i++) {
      const p0 = (i > 0 ? pts[i - 1] : pts[i]);
      const p1 = pts[i];
      const p2 = pts[i + 1];
      const p3 = (i + 2 < nLen ? pts[i + 2] : pts[i + 1]);

      const w1 = (river.nodes && river.nodes[i]?.widthM) || defaultWidth;
      const w2 = (river.nodes && river.nodes[i + 1]?.widthM) || defaultWidth;
      const d1 = (river.nodes && river.nodes[i]?.depthM) || 1.4;
      const d2 = (river.nodes && river.nodes[i + 1]?.depthM) || 1.4;

      const segLen = Math.hypot(p2[0] - p1[0], p2[2] - p1[2]);
      const stepDist = Math.min(sx, sz, 0.5);
      const steps = Math.max(4, Math.ceil(segLen / stepDist));

      for (let s = 0; s <= steps; s++) {
        const t = s / steps;
        const sample = evaluateCatmullRom(p0, p1, p2, p3, t);
        const curWidth = w1 + (w2 - w1) * t;
        const curDepth = d1 + (d2 - d1) * t;
        const halfW = curWidth * 0.5;

        // Stamp perpendicular ribbon samples
        const subSteps = Math.max(2, Math.ceil(halfW / stepDist));
        for (let sub = -subSteps; sub <= subSteps; sub++) {
          const ratio = sub / subSteps; // -1.0 to +1.0
          const offDist = ratio * halfW;
          const wx = sample.pos[0] + sample.normal[0] * offDist;
          const wz = sample.pos[2] + sample.normal[2] * offDist;

          const px = Math.round((wx - minWorldX) / sx);
          const py = Math.round((wz - minWorldZ) / sz);

          if (px >= 0 && px < W && py >= 0 && py < H) {
            const pIdx = py * W + px;
            mask[pIdx] = 3; // River

            // Parabolic cross-section depth
            const crossSectionFactor = Math.max(0.25, 1.0 - ratio * ratio);
            const depthM = curDepth * crossSectionFactor;
            const depthDm = Math.min(65535, Math.round(depthM * 10.0));
            if (depthDm > depth[pIdx]) {
              depth[pIdx] = depthDm;
            }
          }
        }
      }
      drawnSegments++;
    }
  }
  return drawnSegments;
}

function rasterizePolygons({ items, mask, depth, W, H, minWorldX, minWorldZ, sx, sz, dem, maskType = 2 }) {
  if (!items || !Array.isArray(items)) return 0;
  let count = 0;

  for (const item of items) {
    const poly = item.polygon || item.perimeter || item.points;
    if (!poly || !Array.isArray(poly) || poly.length < 3) continue;

    const surfaceY = typeof item.surfaceElevationYM === 'number' ? item.surfaceElevationYM : 0.0;
    const defaultDepthM = typeof item.avgDepthM === 'number' ? item.avgDepthM : (typeof item.maxDepthM === 'number' ? item.maxDepthM * 0.6 : 1.5);
    const maxD = typeof item.maxDepthM === 'number' ? item.maxDepthM : defaultDepthM * 1.5;

    // Determine 2D bounding box
    let minX = Infinity, maxX = -Infinity, minZ = Infinity, maxZ = -Infinity;
    if (item.bounds?.min && item.bounds?.max) {
      minX = item.bounds.min[0];
      minZ = item.bounds.min[2];
      maxX = item.bounds.max[0];
      maxZ = item.bounds.max[2];
    } else {
      for (const p of poly) {
        if (p[0] < minX) minX = p[0];
        if (p[0] > maxX) maxX = p[0];
        if (p[2] < minZ) minZ = p[2];
        if (p[2] > maxZ) maxZ = p[2];
      }
    }

    const minPy = Math.max(0, Math.floor((minZ - minWorldZ) / sz));
    const maxPy = Math.min(H - 1, Math.ceil((maxZ - minWorldZ) / sz));
    if (minPy > maxPy) continue;

    const n = poly.length;

    for (let py = minPy; py <= maxPy; py++) {
      const wz = minWorldZ + py * sz;
      const nodes = [];

      for (let j = 0; j < n; j++) {
        const pA = poly[j];
        const pB = poly[(j + 1) % n];
        const az = pA[2], bz = pB[2];
        if ((az <= wz && bz > wz) || (bz <= wz && az > wz)) {
          const t = (wz - az) / (bz - az);
          nodes.push(pA[0] + t * (pB[0] - pA[0]));
        }
      }

      if (nodes.length < 2) continue;
      nodes.sort((a, b) => a - b);

      const rowOffset = py * W;

      for (let k = 0; k < nodes.length; k += 2) {
        if (k + 1 >= nodes.length) break;
        const xStart = nodes[k];
        const xEnd = nodes[k + 1];
        if (xEnd < minWorldX || xStart > (minWorldX + (W - 1) * sx)) continue;

        const minPx = Math.max(0, Math.floor((xStart - minWorldX) / sx));
        const maxPx = Math.min(W - 1, Math.ceil((xEnd - minWorldX) / sx));

        for (let px = minPx; px <= maxPx; px++) {
          const wx = minWorldX + px * sx;
          const pIdx = rowOffset + px;

          mask[pIdx] = maskType; // 2 = Lake/Pond

          let depthM = defaultDepthM;
          if (dem) {
            const terrainY = dem.getElev(wx, wz);
            if (terrainY < surfaceY) {
              depthM = Math.min(maxD, Math.max(0.1, surfaceY - terrainY));
            }
          }

          const depthDm = Math.min(65535, Math.round(depthM * 10.0));
          if (depthDm > depth[pIdx]) {
            depth[pIdx] = depthDm;
          }
        }
      }
    }
    count++;
  }
  return count;
}

// --- Main Execution ---

async function main() {
  const options = parseArgs();
  console.log(`=== TBD Water Export to PNG Converter ===`);
  console.log(`Source export path: ${options.exportDir}`);
  console.log(`Terrain           : ${options.terrain}`);
  console.log(`Mode              : ${options.mode}`);
  console.log(`Vector enhance    : ${options.vectorEnhance}`);

  // Locate water directory
  const candidateDirs = [
    options.exportDir,
    join(options.exportDir, options.terrain, 'terrain/water'),
    join(options.exportDir, options.terrain, 'water'),
    join(options.exportDir, 'terrain/water'),
    join(options.exportDir, 'water'),
    join(options.exportDir, '$tbd_framework:worlds/water'),
  ];

  let waterDir = options.exportDir;
  for (const d of candidateDirs) {
    if (existsSync(d) && (
      existsSync(join(d, 'water_meta.json')) ||
      existsSync(join(d, 'inland_water_meta.json')) ||
      existsSync(join(d, 'bathymetry_mask.txt')) ||
      existsSync(join(d, 'TBD_WaterExport_meta.json')) ||
      existsSync(join(d, 'TBD_InlandWaterExport_meta.json'))
    )) {
      waterDir = d;
      break;
    }
  }

  console.log(`Resolved water data dir: ${waterDir}`);

  // Resolve metadata file
  let metaPath = join(waterDir, 'water_meta.json');
  let isInland = false;
  if (!existsSync(metaPath)) {
    if (existsSync(join(waterDir, 'inland_water_meta.json'))) {
      metaPath = join(waterDir, 'inland_water_meta.json');
      isInland = true;
    } else if (existsSync(join(waterDir, 'TBD_WaterExport_meta.json'))) {
      metaPath = join(waterDir, 'TBD_WaterExport_meta.json');
    } else if (existsSync(join(waterDir, 'TBD_InlandWaterExport_meta.json'))) {
      metaPath = join(waterDir, 'TBD_InlandWaterExport_meta.json');
      isInland = true;
    }
  }

  // Determine output directory
  const defaultOutCandidates = [
    options.outDir,
    join(waterDir, 'images'),
    resolve(process.cwd(), 'packages/map-assets', options.terrain, 'staging/water/images'),
    resolve(process.cwd(), 'output/water'),
    resolve(process.cwd(), 'images'),
  ].filter(Boolean);

  let targetOutDir = '';
  for (const cand of defaultOutCandidates) {
    try {
      if (!existsSync(cand)) {
        mkdirSync(cand, { recursive: true });
      }
      targetOutDir = cand;
      break;
    } catch {
      // Continue to next candidate if directory is read-only
    }
  }

  if (!targetOutDir) {
    targetOutDir = resolve(process.cwd(), 'output/water');
    mkdirSync(targetOutDir, { recursive: true });
  }
  options.outDir = targetOutDir;
  console.log(`Target output dir : ${options.outDir}`);

  let outPrefix = isInland ? `${options.terrain}-inland-water` : `${options.terrain}-water`;

  const maskCandidates = [
    join(waterDir, 'bathymetry_mask.txt'),
    join(waterDir, 'TBD_WaterExport_mask.txt'),
    join(waterDir, 'TBD_InlandWaterExport_mask.txt'),
  ];
  const maskPath = maskCandidates.find(existsSync);

  const depthCandidates = [
    join(waterDir, 'bathymetry_depth.txt'),
    join(waterDir, 'TBD_WaterExport_depth.txt'),
    join(waterDir, 'TBD_InlandWaterExport_depth.txt'),
  ];
  const depthPath = depthCandidates.find(existsSync);

  const riversCandidates = [
    join(waterDir, 'rivers.json'),
    join(waterDir, 'TBD_InlandWaterExport_vectors.json'),
    join(waterDir, 'TBD_WaterExport_vectors.json'),
  ];
  const riversPath = riversCandidates.find(existsSync);

  const lakesCandidates = [
    join(waterDir, 'lakes.json'),
    join(waterDir, 'TBD_WaterExport_vectors.json'),
  ];
  const lakesPath = lakesCandidates.find(existsSync);

  const pondsCandidates = [
    join(waterDir, 'ponds.json'),
  ];
  const pondsPath = pondsCandidates.find(existsSync);

  if (!existsSync(metaPath)) {
    console.error(`Error: Missing required metadata file (water_meta.json or inland_water_meta.json) in ${waterDir}`);
    process.exit(1);
  }

  const meta = JSON.parse(readFileSync(metaPath, 'utf8'));
  let W = meta.widthPx || 12800;
  let H = meta.heightPx || 12800;
  let worldSizeM = meta.worldSizeM || 12800;
  let minWorldX = 0;
  let minWorldZ = 0;
  let maxWorldX = worldSizeM;
  let maxWorldZ = worldSizeM;

  let mpp = meta.planarResolutionM || (worldSizeM / W);
  if (options.res > 0) {
    mpp = options.res;
  }

  // Handle ROI
  if (options.roi) {
    [minWorldX, minWorldZ, maxWorldX, maxWorldZ] = options.roi;
    W = Math.max(16, Math.round((maxWorldX - minWorldX) / mpp));
    H = Math.max(16, Math.round((maxWorldZ - minWorldZ) / mpp));
    outPrefix += `-roi-${Math.round(minWorldX)}_${Math.round(minWorldZ)}`;
    console.log(`ROI Enabled: [${minWorldX}, ${minWorldZ}] to [${maxWorldX}, ${maxWorldZ}] (${W}x${H} px @ ${mpp}m/px)`);
  } else if (options.res > 0) {
    W = Math.round(worldSizeM / mpp);
    H = W;
    console.log(`Custom Resolution: ${W}x${H} px (${mpp} m/px)`);
  }

  const totalPx = W * H;
  const sx = (maxWorldX - minWorldX) / (W > 1 ? W - 1 : 1);
  const sz = (maxWorldZ - minWorldZ) / (H > 1 ? H - 1 : 1);

  const mask = new Uint8Array(totalPx);
  const depth = new Uint16Array(totalPx);

  // 1. Parse discrete matrix files if matching full terrain
  if (!options.roi && maskPath && depthPath && W === (meta.widthPx || 12800) && H === (meta.heightPx || 12800)) {
    console.log(`\n1. Parsing Ground-Truth Ocean Bathymetry Mask (${W}x${H})...`);
    const t0 = performance.now();
    const maskBuf = readFileSync(maskPath);
    let idx = 0, cur = 0, inNum = false;
    for (let i = 0; i < maskBuf.length; i++) {
      const c = maskBuf[i];
      if (c >= 0x30 && c <= 0x39) {
        cur = cur * 10 + (c - 0x30);
        inNum = true;
      } else if (inNum) {
        if (idx < totalPx) mask[idx++] = cur;
        cur = 0;
        inNum = false;
      }
    }
    if (inNum && idx < totalPx) mask[idx++] = cur;
    console.log(`   Mask parsed in ${(performance.now() - t0).toFixed(0)} ms (${idx} pixels).`);

    console.log(`\n2. Parsing Ground-Truth Ocean Bathymetry Depth (${W}x${H})...`);
    const t1 = performance.now();
    const depthBuf = readFileSync(depthPath);
    idx = 0; cur = 0; inNum = false;
    for (let i = 0; i < depthBuf.length; i++) {
      const c = depthBuf[i];
      if (c >= 0x30 && c <= 0x39) {
        cur = cur * 10 + (c - 0x30);
        inNum = true;
      } else if (inNum) {
        if (idx < totalPx) depth[idx++] = cur;
        cur = 0;
        inNum = false;
      }
    }
    if (inNum && idx < totalPx) depth[idx++] = cur;
    console.log(`   Depth parsed in ${(performance.now() - t1).toFixed(0)} ms (${idx} pixels).`);
  }

  // 2. Vector Spline & Polygon Enhancement (Rivers, Lakes, Ponds)
  if (options.vectorEnhance && (lakesPath || pondsPath || riversPath)) {
    console.log(`\n3. Applying Continuous Vector Splines (Rivers) & Lake/Pond Polygons...`);
    const tVec = performance.now();
    let lakes = [];
    let ponds = [];
    let rivers = [];

    if (lakesPath && existsSync(lakesPath)) {
      const lData = JSON.parse(readFileSync(lakesPath, 'utf8'));
      lakes = lData.lakes || lData.waterBodies || [];
    }
    if (pondsPath && existsSync(pondsPath)) {
      const pData = JSON.parse(readFileSync(pondsPath, 'utf8'));
      ponds = pData.ponds || [];
    }
    if (riversPath && existsSync(riversPath)) {
      const rData = JSON.parse(readFileSync(riversPath, 'utf8'));
      rivers = rData.rivers || [];
    }

    const dem = loadDemHeightfield(options.demPath, waterDir);
    const lCount = rasterizePolygons({ items: lakes, mask, depth, W, H, minWorldX, minWorldZ, sx, sz, dem, maskType: 2 });
    const pCount = rasterizePolygons({ items: ponds, mask, depth, W, H, minWorldX, minWorldZ, sx, sz, dem, maskType: 2 });
    const rSegs = rasterizeSplineRivers({ rivers, mask, depth, W, H, minWorldX, minWorldZ, sx, sz });

    console.log(`   Vector rasterization complete in ${(performance.now() - tVec).toFixed(0)} ms (${lCount} lakes, ${pCount} ponds, ${rSegs} river segments).`);
  }

  // Compute stats
  let minDepthDm = 65535, maxDepthDm = 0;
  let oceanCount = 0, lakeCount = 0, riverCount = 0, landCount = 0;
  for (let i = 0; i < totalPx; i++) {
    const m = mask[i];
    if (m === 0) {
      landCount++;
    } else {
      if (m === 1) oceanCount++;
      else if (m === 2) lakeCount++;
      else if (m === 3) riverCount++;
      const d = depth[i];
      if (d < minDepthDm) minDepthDm = d;
      if (d > maxDepthDm) maxDepthDm = d;
    }
  }
  if (minDepthDm > maxDepthDm) minDepthDm = 0;

  console.log(`\nStatistics: Land=${landCount}, Ocean=${oceanCount}, Lake/Pond=${lakeCount}, River=${riverCount}`);
  console.log(`Total Water Pixels: ${oceanCount + lakeCount + riverCount}`);
  console.log(`Depth Range: ${(minDepthDm * 0.1).toFixed(1)} m to ${(maxDepthDm * 0.1).toFixed(1)} m`);

  const isWaterMode = options.mode === 'water' || options.mode === 'bathymetry';
  const runAll = options.mode === 'all';

  // --- Output: 32-bit RGBA Water Image (Transparent Land, North-Up) ---
  if (runAll || isWaterMode) {
    console.log(`\n4. Building 32-bit RGBA Pure Water PNG (Transparent Land, North-Up)...`);
    const tStart = performance.now();
    const rowBytes = 1 + W * 4;
    const raw = Buffer.allocUnsafe(rowBytes * H);

    for (let iy = 0; iy < H; iy++) {
      const exportY = H - 1 - iy; // North-up flip
      const exportRowOffset = exportY * W;
      let o = iy * rowBytes;
      raw[o++] = 0; // PNG filter: None

      for (let x = 0; x < W; x++) {
        const pIdx = exportRowOffset + x;
        const m = mask[pIdx];
        if (m === 0) {
          // Land: 100% Transparent
          raw[o++] = 0;
          raw[o++] = 0;
          raw[o++] = 0;
          raw[o++] = 0;
        } else {
          // Water: Depth colored RGB with solid Alpha 255
          const depthM = depth[pIdx] * 0.1;
          const [r, g, b] = getBathymetryColor(depthM, m);
          raw[o++] = r;
          raw[o++] = g;
          raw[o++] = b;
          raw[o++] = 255;
        }
      }
    }

    const png = encodePng({ width: W, height: H, bitDepth: 8, colorType: 6, rawScanlines: raw });
    const outBathymetry = join(options.outDir, `${outPrefix}-bathymetry.png`);
    const outWater = join(options.outDir, `${outPrefix}.png`);
    writeFileSync(outBathymetry, png);
    writeFileSync(outWater, png);
    console.log(`   Wrote ${outBathymetry} (${(png.length / 1024 / 1024).toFixed(2)} MB in ${(performance.now() - tStart).toFixed(0)} ms)`);
    console.log(`   Wrote ${outWater}`);
  }

  // --- Output 2: 24-bit RGB Bathymetry Dark Mode with Contours (North-Up) ---
  if (runAll || options.mode === 'dark') {
    console.log(`\n5. Building 24-bit RGB Dark Bathymetry with Contours PNG (North-Up)...`);
    const tStart = performance.now();
    const rowBytes = 1 + W * 3;
    const raw = Buffer.allocUnsafe(rowBytes * H);

    for (let iy = 0; iy < H; iy++) {
      const exportY = H - 1 - iy;
      const exportRowOffset = exportY * W;
      let o = iy * rowBytes;
      raw[o++] = 0;

      for (let x = 0; x < W; x++) {
        const pIdx = exportRowOffset + x;
        const m = mask[pIdx];
        if (m === 0) {
          raw[o++] = DARK_LAND_COLOR[0];
          raw[o++] = DARK_LAND_COLOR[1];
          raw[o++] = DARK_LAND_COLOR[2];
        } else {
          const depthM = depth[pIdx] * 0.1;
          const [r, g, b] = getBathymetryColor(depthM, m);
          const contour = getContourMultiplier(depthM, m);
          raw[o++] = Math.round(r * contour);
          raw[o++] = Math.round(g * contour);
          raw[o++] = Math.round(b * contour);
        }
      }
    }

    const png = encodePng({ width: W, height: H, bitDepth: 8, colorType: 2, rawScanlines: raw });
    const outPath = join(options.outDir, `${outPrefix}-bathymetry-dark.png`);
    writeFileSync(outPath, png);
    console.log(`   Wrote ${outPath} (${(png.length / 1024 / 1024).toFixed(2)} MB in ${(performance.now() - tStart).toFixed(0)} ms)`);
  }

  // --- Output 3: 16-bit Grayscale Depth Heightfield PNG (North-Up) ---
  if (runAll || options.mode === 'depth16') {
    console.log(`\n6. Building 16-bit Grayscale Depth Heightfield PNG (North-Up)...`);
    const tStart = performance.now();
    const rowBytes = 1 + W * 2;
    const raw = Buffer.allocUnsafe(rowBytes * H);

    for (let iy = 0; iy < H; iy++) {
      const exportY = H - 1 - iy;
      const exportRowOffset = exportY * W;
      let o = iy * rowBytes;
      raw[o++] = 0;

      for (let x = 0; x < W; x++) {
        const pIdx = exportRowOffset + x;
        const v = mask[pIdx] === 0 ? 0 : depth[pIdx];
        raw[o++] = (v >> 8) & 0xff;
        raw[o++] = v & 0xff;
      }
    }

    const png = encodePng({ width: W, height: H, bitDepth: 16, colorType: 0, rawScanlines: raw });
    const outPath = join(options.outDir, `${outPrefix}-depth-16bit.png`);
    writeFileSync(outPath, png);
    console.log(`   Wrote ${outPath} (${(png.length / 1024 / 1024).toFixed(2)} MB in ${(performance.now() - tStart).toFixed(0)} ms)`);
  }

  // --- Output 4: 8-bit Grayscale Classification Mask PNG (North-Up) ---
  if (runAll || options.mode === 'mask') {
    console.log(`\n7. Building 8-bit Classification Mask PNG (North-Up)...`);
    const tStart = performance.now();
    const rowBytes = 1 + W;
    const raw = Buffer.allocUnsafe(rowBytes * H);

    for (let iy = 0; iy < H; iy++) {
      const exportY = H - 1 - iy;
      const exportRowOffset = exportY * W;
      let o = iy * rowBytes;
      raw[o++] = 0;

      for (let x = 0; x < W; x++) {
        const pIdx = exportRowOffset + x;
        raw[o++] = mask[pIdx];
      }
    }

    const png = encodePng({ width: W, height: H, bitDepth: 8, colorType: 0, rawScanlines: raw });
    const outPath = join(options.outDir, `${outPrefix}-mask.png`);
    writeFileSync(outPath, png);
    console.log(`   Wrote ${outPath} (${(png.length / 1024 / 1024).toFixed(2)} MB in ${(performance.now() - tStart).toFixed(0)} ms)`);
  }

  // --- Output 5: Downsampled Preview (1600x1600 RGB, North-Up) ---
  if (runAll || options.mode === 'preview') {
    console.log(`\n8. Building Preview PNG (North-Up)...`);
    const tStart = performance.now();
    const targetDim = Math.min(1600, W);
    const scale = Math.max(1, Math.round(W / targetDim));
    const pw = Math.round(W / scale);
    const ph = Math.round(H / scale);
    const rowBytes = 1 + pw * 3;
    const raw = Buffer.allocUnsafe(rowBytes * ph);

    for (let py = 0; py < ph; py++) {
      let o = py * rowBytes;
      raw[o++] = 0;

      for (let px = 0; px < pw; px++) {
        let sumDepthDm = 0;
        let waterCount = 0;
        let riverCountInBlock = 0;
        let lakeCountInBlock = 0;
        let primaryType = 0;

        for (let dy = 0; dy < scale; dy++) {
          const iy = py * scale + dy;
          if (iy >= H) continue;
          const exportY = H - 1 - iy;
          const exportRowOffset = exportY * W;
          for (let dx = 0; dx < scale; dx++) {
            const x = px * scale + dx;
            if (x >= W) continue;
            const pIdx = exportRowOffset + x;
            const m = mask[pIdx];
            if (m !== 0) {
              waterCount++;
              sumDepthDm += depth[pIdx];
              if (m === 3) riverCountInBlock++;
              else if (m === 2) lakeCountInBlock++;
            }
          }
        }

        if (waterCount === 0) {
          raw[o++] = DARK_LAND_COLOR[0];
          raw[o++] = DARK_LAND_COLOR[1];
          raw[o++] = DARK_LAND_COLOR[2];
        } else {
          if (riverCountInBlock > 0) primaryType = 3;
          else if (lakeCountInBlock > 0) primaryType = 2;
          else primaryType = 1;

          const avgDepthM = (sumDepthDm / waterCount) * 0.1;
          const [r, g, b] = getBathymetryColor(avgDepthM, primaryType);
          const contour = getContourMultiplier(avgDepthM, primaryType);

          let finalR, finalG, finalB;
          if (primaryType === 3 || primaryType === 2) {
            finalR = Math.round(r * contour);
            finalG = Math.round(g * contour);
            finalB = Math.round(b * contour);
          } else {
            const landWeight = ((scale * scale) - waterCount) / (scale * scale);
            finalR = Math.round((r * contour) * (1 - landWeight) + DARK_LAND_COLOR[0] * landWeight);
            finalG = Math.round((g * contour) * (1 - landWeight) + DARK_LAND_COLOR[1] * landWeight);
            finalB = Math.round((b * contour) * (1 - landWeight) + DARK_LAND_COLOR[2] * landWeight);
          }

          raw[o++] = finalR;
          raw[o++] = finalG;
          raw[o++] = finalB;
        }
      }
    }

    const png = encodePng({ width: pw, height: ph, bitDepth: 8, colorType: 2, rawScanlines: raw });
    const outPath = join(options.outDir, `${outPrefix}-preview.png`);
    writeFileSync(outPath, png);
    console.log(`   Wrote ${outPath} (${(png.length / 1024 / 1024).toFixed(2)} MB in ${(performance.now() - tStart).toFixed(0)} ms)`);
  }

  console.log(`\n=== Water export complete! ===`);
}

main().catch((err) => {
  console.error('Fatal error:', err);
  process.exit(1);
});


