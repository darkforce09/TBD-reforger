// T-091.0 — Pack the Workbench plugin's ASCII uint16 raster (TBD_TerrainExport_heightmap.txt)
// into a 16-bit grayscale PNG (bitDepth 16, colorType 0, big-endian samples) for the DEM gate.
//
// The plugin (TBD_TerrainExportPlugin.c) writes rows of space-separated uint16 values in the
// exact grid order verify-terrain-alignment expects (row py = world z, col px = world x; no flip),
// so this is a straight repack — no resample, no orientation change.
//
// Usage:
//   node scripts/raw-u16-to-dem-png.mjs --raster <heightmap.txt> --meta <meta.json> --out <png>
// Self-checks IHDR (bitDepth 16, colorType 0) + dims via pngjs before exiting 0.
import { readFileSync, writeFileSync } from 'node:fs';
import { deflateSync } from 'node:zlib';
import { PNG } from 'pngjs';

function arg(name, def) {
  const i = process.argv.indexOf(name);
  return i >= 0 ? process.argv[i + 1] : def;
}

const rasterPath = arg('--raster');
const metaPath = arg('--meta');
const outPath = arg('--out');
if (!rasterPath || !metaPath || !outPath) {
  console.error('Usage: raw-u16-to-dem-png.mjs --raster <txt> --meta <json> --out <png>');
  process.exit(2);
}

const meta = JSON.parse(readFileSync(metaPath, 'utf8'));
const W = meta.widthPx;
const H = meta.heightPx;
if (!(W > 0 && H > 0)) {
  console.error(`Bad meta dims ${W}x${H}`);
  process.exit(1);
}

// --- Parse ASCII raster -> Uint16Array (row-major), byte-scan for speed/low alloc ---
console.log(`Parsing raster ${rasterPath} (${W}x${H})...`);
const buf = readFileSync(rasterPath);
const raster = new Uint16Array(W * H);
let idx = 0;
let cur = 0;
let inNum = false;
for (let i = 0; i < buf.length; i++) {
  const c = buf[i];
  if (c >= 0x30 && c <= 0x39) {
    cur = cur * 10 + (c - 0x30);
    inNum = true;
  } else if (inNum) {
    raster[idx++] = cur;
    cur = 0;
    inNum = false;
  }
}
if (inNum) raster[idx++] = cur;
if (idx !== W * H) {
  console.error(`FAIL parsed ${idx} values, expected ${W * H}`);
  process.exit(1);
}
let uMin = 65535;
let uMax = 0;
for (let k = 0; k < raster.length; k++) {
  const v = raster[k];
  if (v < uMin) uMin = v;
  if (v > uMax) uMax = v;
}
console.log(`Parsed ${idx} samples; u16 range [${uMin}, ${uMax}]`);

// --- Build raw filtered scanlines: per row, filter byte 0 + W big-endian uint16 ---
const rowBytes = 1 + W * 2;
const rawImg = Buffer.allocUnsafe(rowBytes * H);
for (let y = 0; y < H; y++) {
  let o = y * rowBytes;
  rawImg[o++] = 0; // filter: none
  const base = y * W;
  for (let x = 0; x < W; x++) {
    const v = raster[base + x];
    rawImg[o++] = (v >> 8) & 0xff; // hi (big-endian)
    rawImg[o++] = v & 0xff; // lo
  }
}

// --- CRC32 (PNG) ---
const crcTable = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    t[n] = c >>> 0;
  }
  return t;
})();
function crc32(b) {
  let c = 0xffffffff;
  for (let i = 0; i < b.length; i++) c = crcTable[(c ^ b[i]) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}
function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const typeBuf = Buffer.from(type, 'ascii');
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])), 0);
  return Buffer.concat([len, typeBuf, data, crc]);
}

const sig = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
const ihdr = Buffer.alloc(13);
ihdr.writeUInt32BE(W, 0);
ihdr.writeUInt32BE(H, 4);
ihdr[8] = 16; // bit depth
ihdr[9] = 0; // color type: grayscale
ihdr[10] = 0; // compression
ihdr[11] = 0; // filter
ihdr[12] = 0; // interlace

console.log('Deflating IDAT...');
const idat = deflateSync(rawImg, { level: 9 });
const png = Buffer.concat([
  sig,
  chunk('IHDR', ihdr),
  chunk('IDAT', idat),
  chunk('IEND', Buffer.alloc(0)),
]);
writeFileSync(outPath, png);
console.log(`Wrote ${outPath} (${png.length} bytes)`);

// --- Self-check via pngjs (skipRescale preserves true 16-bit) ---
const check = PNG.sync.read(readFileSync(outPath), { skipRescale: true });
const depth = check.bitDepth ?? check.depth;
if (depth !== 16 || check.colorType !== 0 || check.width !== W || check.height !== H) {
  console.error(
    `FAIL IHDR check: depth=${depth} colorType=${check.colorType} ${check.width}x${check.height}`,
  );
  process.exit(1);
}
const channels = check.data.length / (W * H);
for (const [x, y] of [
  [0, 0],
  [W - 1, H - 1],
  [W >> 1, H >> 1],
]) {
  const got = check.data[(y * W + x) * channels];
  const want = raster[y * W + x];
  if (got !== want) {
    console.error(`FAIL round-trip (${x},${y}): got ${got} want ${want}`);
    process.exit(1);
  }
}
console.log('OK  IHDR bitDepth=16 colorType=0 dims match; round-trip pixels OK');
