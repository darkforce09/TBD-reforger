#!/usr/bin/env -S node --max-old-space-size=8192
/**
 * export-roads-to-png.mjs
 *
 * Renders exported Arma Reforger TBD road network data to high-resolution PNG images.
 * Reads:
 *   - roads_meta.json (Manifest & Topological Junctions)
 *   - highways.json
 *   - roads_paved.json
 *   - roads_dirt.json
 *   - tracks.json
 *   - paths.json
 *   - runways.json
 *
 * Generates:
 *   1. everon-roads-transparent.png (32-bit RGBA, transparent background)
 *   2. everon-roads-dark.png (24-bit RGB, dark topographic background)
 *   3. Individual layer images (highways, roads_paved, roads_dirt, tracks, paths, runways)
 *
 * Usage:
 *   node apps/mod/tbd-export/tools/export-roads-to-png.mjs [path_to_roads_dir] [options]
 */

import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { deflateSync, crc32 as zlibCrc32 } from 'node:zlib';

function parseArgs() {
  const args = process.argv.slice(2);
  const options = {
    roadsDir: '',
    outDir: '',
    terrain: 'everon',
    worldSize: 12800,
    width: 2048,
    height: 2048,
    showJunctions: false, // Clean roads by default!
  };

  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if (arg === '--roads-dir' && i + 1 < args.length) {
      options.roadsDir = args[++i];
    } else if (arg === '--out-dir' && i + 1 < args.length) {
      options.outDir = args[++i];
    } else if (arg === '--size' && i + 1 < args.length) {
      options.width = parseInt(args[++i], 10);
      options.height = options.width;
    } else if (arg === '--terrain' && i + 1 < args.length) {
      options.terrain = args[++i];
    } else if (arg === '--show-junctions') {
      options.showJunctions = true;
    } else if (!arg.startsWith('--') && !options.roadsDir) {
      options.roadsDir = arg;
    }
  }

  if (!options.roadsDir) {
    const candidates = [
      '/home/Samuel/.local/share/Steam/steamapps/compatdata/1874910/pfx/drive_c/users/steamuser/Documents/My Games/ArmaReforgerWorkbench/profile/TBD_Export/$tbd_framework:worlds/roads',
      '/home/Samuel/Games/ArmaReforger-Base/TBD_Export/$tbd_framework:worlds/roads',
      '/home/Samuel/Games/ArmaReforger-Base/TBD_Export/everon/roads',
      resolve(process.cwd(), 'output/roads'),
    ];
    for (const c of candidates) {
      if (existsSync(c)) {
        options.roadsDir = c;
        break;
      }
    }
    if (!options.roadsDir) options.roadsDir = candidates[0];
  }

  if (!options.outDir) {
    options.outDir = join(options.roadsDir, 'images');
  }

  return options;
}

// --- Low-Level PNG Encoder ---

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

// --- Color & Style Definitions ---

const ROAD_STYLES = {
  highways: {
    color: [255, 175, 45, 255],    // Bright Highway Amber/Gold
    widthM: 8.0,
    minPx: 3.5,
    name: 'Highways & Major Arterials',
  },
  roads_paved: {
    color: [240, 242, 248, 255],   // Solid Crisp White/Slate
    widthM: 6.0,
    minPx: 2.5,
    name: 'Secondary Paved Roads',
  },
  roads_dirt: {
    color: [215, 140, 75, 255],    // Warm Terracotta / Dirt Ochre
    widthM: 4.5,
    minPx: 2.0,
    name: 'Dirt & Gravel Roads',
  },
  tracks: {
    color: [145, 205, 60, 255],    // Chartreuse / Olive Track
    widthM: 3.5,
    minPx: 1.5,
    name: 'Forestry & Agricultural Tracks',
  },
  paths: {
    color: [50, 220, 240, 255],    // Bright Cyan Footpaths
    widthM: 2.0,
    minPx: 1.2,
    name: 'Footpaths & Walking Trails',
  },
  runways: {
    color: [255, 80, 120, 255],    // Distinct Magenta/Coral for Airfield Runways/Taxiways
    widthM: 25.0,
    minPx: 6.0,
    name: 'Airfield Runways & Taxiways',
  },
};

const DARK_BG_COLOR = [18, 22, 28]; // Dark Navy Charcoal
const JUNCTION_COLOR = [255, 225, 100, 255]; // Yellow-Gold Junction Highlights

// --- Rasterization / Drawing Engine ---

class Canvas2D {
  constructor(width, height) {
    this.width = width;
    this.height = height;
    this.rgba = new Uint8Array(width * height * 4);
  }

  fill(r, g, b, a = 255) {
    const len = this.width * this.height * 4;
    for (let i = 0; i < len; i += 4) {
      this.rgba[i] = r;
      this.rgba[i + 1] = g;
      this.rgba[i + 2] = b;
      this.rgba[i + 3] = a;
    }
  }

  setPixel(x, y, r, g, b, a = 255) {
    if (x < 0 || x >= this.width || y < 0 || y >= this.height) return;
    const idx = (y * this.width + x) * 4;
    if (a >= 255) {
      this.rgba[idx] = r;
      this.rgba[idx + 1] = g;
      this.rgba[idx + 2] = b;
      this.rgba[idx + 3] = 255;
    } else {
      const srcA = a / 255;
      const dstA = this.rgba[idx + 3] / 255;
      const outA = srcA + dstA * (1 - srcA);
      if (outA > 0) {
        this.rgba[idx] = Math.round((r * srcA + this.rgba[idx] * dstA * (1 - srcA)) / outA);
        this.rgba[idx + 1] = Math.round((g * srcA + this.rgba[idx + 1] * dstA * (1 - srcA)) / outA);
        this.rgba[idx + 2] = Math.round((b * srcA + this.rgba[idx + 2] * dstA * (1 - srcA)) / outA);
        this.rgba[idx + 3] = Math.round(outA * 255);
      }
    }
  }

  drawCircle(cx, cy, radius, r, g, b, a = 255) {
    const rInt = Math.ceil(radius);
    const r2 = radius * radius;
    const minX = Math.max(0, Math.floor(cx - rInt));
    const maxX = Math.min(this.width - 1, Math.ceil(cx + rInt));
    const minY = Math.max(0, Math.floor(cy - rInt));
    const maxY = Math.min(this.height - 1, Math.ceil(cy + rInt));

    for (let y = minY; y <= maxY; y++) {
      const dy = y - cy;
      const dy2 = dy * dy;
      for (let x = minX; x <= maxX; x++) {
        const dx = x - cx;
        const d2 = dx * dx + dy2;
        if (d2 <= r2) {
          const edgeDist = radius - Math.sqrt(d2);
          let alpha = a;
          if (edgeDist < 0.75) {
            alpha = Math.round(a * (edgeDist / 0.75));
          }
          if (alpha > 0) this.setPixel(x, y, r, g, b, alpha);
        }
      }
    }
  }

  drawLineSegment(x0, y0, x1, y1, radius, r, g, b, a = 255) {
    const dx = x1 - x0;
    const dy = y1 - y0;
    const len = Math.hypot(dx, dy);
    if (len < 0.5) {
      this.drawCircle(x0, y0, radius, r, g, b, a);
      return;
    }

    const steps = Math.max(2, Math.ceil(len * 2.0));
    const invSteps = 1.0 / steps;

    for (let s = 0; s <= steps; s++) {
      const t = s * invSteps;
      const cx = x0 + dx * t;
      const cy = y0 + dy * t;
      this.drawCircle(cx, cy, radius, r, g, b, a);
    }
  }

  toRgbaScanlines() {
    const rowBytes = 1 + this.width * 4;
    const raw = Buffer.allocUnsafe(rowBytes * this.height);
    for (let y = 0; y < this.height; y++) {
      let o = y * rowBytes;
      raw[o++] = 0; // PNG filter None
      const srcOffset = y * this.width * 4;
      for (let x = 0; x < this.width * 4; x++) {
        raw[o++] = this.rgba[srcOffset + x];
      }
    }
    return raw;
  }

  toRgbScanlines(bgR = 18, bgG = 22, bgB = 28) {
    const rowBytes = 1 + this.width * 3;
    const raw = Buffer.allocUnsafe(rowBytes * this.height);
    for (let y = 0; y < this.height; y++) {
      let o = y * rowBytes;
      raw[o++] = 0; // PNG filter None
      const srcOffset = y * this.width * 4;
      for (let x = 0; x < this.width; x++) {
        const pIdx = srcOffset + x * 4;
        const alpha = this.rgba[pIdx + 3] / 255;
        if (alpha <= 0) {
          raw[o++] = bgR;
          raw[o++] = bgG;
          raw[o++] = bgB;
        } else if (alpha >= 1.0) {
          raw[o++] = this.rgba[pIdx];
          raw[o++] = this.rgba[pIdx + 1];
          raw[o++] = this.rgba[pIdx + 2];
        } else {
          raw[o++] = Math.round(this.rgba[pIdx] * alpha + bgR * (1 - alpha));
          raw[o++] = Math.round(this.rgba[pIdx + 1] * alpha + bgG * (1 - alpha));
          raw[o++] = Math.round(this.rgba[pIdx + 2] * alpha + bgB * (1 - alpha));
        }
      }
    }
    return raw;
  }
}

// --- Main Execution ---

async function main() {
  const options = parseArgs();
  console.log(`=== TBD Continuous Road Network Visualizer ===`);
  console.log(`Source roads dir: ${options.roadsDir}`);
  console.log(`Target out dir  : ${options.outDir}`);
  console.log(`Resolution      : ${options.width} x ${options.height} px`);
  console.log(`Show Junctions  : ${options.showJunctions}`);

  if (!existsSync(options.roadsDir)) {
    console.error(`Error: Directory not found: ${options.roadsDir}`);
    process.exit(1);
  }

  mkdirSync(options.outDir, { recursive: true });

  const metaFile = join(options.roadsDir, 'roads_meta.json');
  let worldSize = options.worldSize;
  let junctions = [];
  if (existsSync(metaFile)) {
    try {
      const meta = JSON.parse(readFileSync(metaFile, 'utf8'));
      if (meta.worldSizeM) worldSize = meta.worldSizeM;
      if (meta.junctions) junctions = meta.junctions;
      console.log(`Loaded metadata: mapName='${meta.mapName}', worldSize=${worldSize}m, totalSegments=${meta.totalSegments}, junctions=${junctions.length}`);
    } catch (e) {
      console.warn(`Could not parse roads_meta.json:`, e.message);
    }
  }

  const mpp = worldSize / options.width;
  console.log(`Scale: ${mpp.toFixed(2)} meters/pixel`);

  function worldToScreen(wx, wz) {
    const sx = (wx / worldSize) * (options.width - 1);
    const sy = ((worldSize - wz) / worldSize) * (options.height - 1);
    return [sx, sy];
  }

  const layers = [
    'highways',
    'roads_paved',
    'roads_dirt',
    'tracks',
    'paths',
    'runways',
  ];

  const loadedData = {};
  let totalSegmentsAll = 0;

  for (const layerKey of layers) {
    const filePath = join(options.roadsDir, `${layerKey}.json`);
    if (existsSync(filePath)) {
      try {
        const json = JSON.parse(readFileSync(filePath, 'utf8'));
        loadedData[layerKey] = json.segments || [];
        totalSegmentsAll += loadedData[layerKey].length;
        console.log(`Layer [${layerKey}]: ${loadedData[layerKey].length} continuous routes (${(json.totalLengthM || 0).toFixed(0)} m)`);
      } catch (err) {
        console.warn(`Warning: Could not parse ${filePath}:`, err.message);
        loadedData[layerKey] = [];
      }
    } else {
      loadedData[layerKey] = [];
    }
  }

  console.log(`\nTotal continuous routes to render: ${totalSegmentsAll}`);

  const masterCanvas = new Canvas2D(options.width, options.height);
  const renderOrder = ['runways', 'roads_paved', 'roads_dirt', 'tracks', 'paths', 'highways'];

  for (const layerKey of renderOrder) {
    const segments = loadedData[layerKey];
    if (!segments || segments.length === 0) continue;

    const style = ROAD_STYLES[layerKey];
    const [r, g, b, a] = style.color;

    const layerCanvas = new Canvas2D(options.width, options.height);

    for (const seg of segments) {
      const points = seg.points;
      if (!points || points.length < 2) continue;

      const segWidthM = seg.widthM || style.widthM;
      const radius = Math.max(style.minPx * 0.5, (segWidthM * 0.5) / mpp);

      for (let p = 0; p < points.length - 1; p++) {
        const p0 = points[p];
        const p1 = points[p + 1];

        const wx0 = p0[0];
        const wz0 = (p0.length >= 3 ? p0[2] : p0[1]);
        const wx1 = p1[0];
        const wz1 = (p1.length >= 3 ? p1[2] : p1[1]);

        const [sx0, sy0] = worldToScreen(wx0, wz0);
        const [sx1, sy1] = worldToScreen(wx1, wz1);

        masterCanvas.drawLineSegment(sx0, sy0, sx1, sy1, radius, r, g, b, a);
        layerCanvas.drawLineSegment(sx0, sy0, sx1, sy1, radius, r, g, b, a);
      }
    }

    // Write isolated layer PNG
    const layerRaw = layerCanvas.toRgbaScanlines();
    const layerPng = encodePng({
      width: options.width,
      height: options.height,
      bitDepth: 8,
      colorType: 6,
      rawScanlines: layerRaw,
    });
    const layerOutPath = join(options.outDir, `layer-${layerKey}.png`);
    writeFileSync(layerOutPath, layerPng);
    console.log(`Saved isolated layer: ${layerOutPath}`);
  }

  // Draw Subtle Junction Markers only if requested
  if (options.showJunctions && junctions.length > 0) {
    console.log(`Rendering ${junctions.length} topological road junctions...`);
    for (const junc of junctions) {
      if (!junc.pos || junc.pos.length < 2) continue;
      const jx = junc.pos[0];
      const jz = (junc.pos.length >= 3 ? junc.pos[2] : junc.pos[1]);
      const [jsx, jsy] = worldToScreen(jx, jz);
      const degree = junc.degree || (junc.connectedSegments ? junc.connectedSegments.length : 2);
      if (degree >= 3) {
        masterCanvas.drawCircle(jsx, jsy, 2.0, JUNCTION_COLOR[0], JUNCTION_COLOR[1], JUNCTION_COLOR[2], 220);
      }
    }
  }

  // Write Master Transparent PNG
  const transRaw = masterCanvas.toRgbaScanlines();
  const transPng = encodePng({
    width: options.width,
    height: options.height,
    bitDepth: 8,
    colorType: 6,
    rawScanlines: transRaw,
  });
  const outTrans = join(options.outDir, `${options.terrain}-roads-transparent.png`);
  writeFileSync(outTrans, transPng);
  console.log(`\nSaved Transparent Road Map: ${outTrans} (${(transPng.length / 1024).toFixed(1)} KB)`);

  // Write Master Dark Mode PNG
  const darkRaw = masterCanvas.toRgbScanlines(DARK_BG_COLOR[0], DARK_BG_COLOR[1], DARK_BG_COLOR[2]);
  const darkPng = encodePng({
    width: options.width,
    height: options.height,
    bitDepth: 8,
    colorType: 2,
    rawScanlines: darkRaw,
  });
  const outDark = join(options.outDir, `${options.terrain}-roads-dark.png`);
  writeFileSync(outDark, darkPng);
  console.log(`Saved Dark Mode Road Map   : ${outDark} (${(darkPng.length / 1024).toFixed(1)} KB)`);

  // Copy to artifacts directory
  const artifactDirs = [
    '/home/Samuel/.gemini/antigravity/brain/7cbded65-f6e5-4597-822f-92ad9287889b',
    '/home/Samuel/.gemini/antigravity/brain/bf48cdd6-37f7-43e8-bef3-eb6f0a0703d4',
  ];
  for (const artifactDir of artifactDirs) {
    if (existsSync(artifactDir)) {
      const artDark = join(artifactDir, `${options.terrain}-roads-dark.png`);
      const artTrans = join(artifactDir, `${options.terrain}-roads-transparent.png`);
      const artHw = join(artifactDir, `layer-highways.png`);
      writeFileSync(artDark, darkPng);
      writeFileSync(artTrans, transPng);
      const hwPath = join(options.outDir, `layer-highways.png`);
      if (existsSync(hwPath)) {
        writeFileSync(artHw, readFileSync(hwPath));
      }
    }
  }

  console.log(`=== Road Visualization Complete ===`);
}

main().catch(console.error);
