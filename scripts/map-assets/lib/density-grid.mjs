#!/usr/bin/env node
// T-090.3.2 — TBDD corner-density grid (plan §3.3). Pure + deterministic; shared by
// build-world-objects.mjs (encode), verify-phase.mjs (recompute + compare) and the
// schema-package golden gate S13 (encode/decode round-trip on a synthetic fixture).
//
// Format (locked, little-endian):
//   header 16 B: u32 magic 'TBDD', u16 version=1, u16 cellM=32, u16 cols=17, u16 rows=17,
//                u8 channelCount=2, 3 B pad (zero)
//   body:        per channel (0=tree, 1=rock) u16[17*17] corner counts, row-major (j*17+i)
//
// Corner definition: corner (i,j) of chunk (cx,cy) sits at world (cx*512 + i*32, cy*512 + j*32);
// its count = instances whose ROUNDED-2dp (x,y) falls in [X-16, X+16) x [Y-16, Y+16).
// Implementation = ONE global (worldSizeM/32 + 1)^2 corner grid per channel
// (gx = floor((x+16)/32)), sliced per chunk file — shared border corners are byte-equal across
// neighbouring files by construction, and every instance lands in exactly one global corner:
//   sum(global corners) == instance count (exact — the export-side PH-P2-5 identity).

export const DENSITY_CELL_M = 32;
export const DENSITY_COLS = 17;
export const DENSITY_ROWS = 17;
export const DENSITY_CHANNELS = ["tree", "rock"];
export const TBDD_VERSION = 1;
export const TBDD_HEADER_BYTES = 16;
export const TBDD_FILE_BYTES =
  TBDD_HEADER_BYTES + DENSITY_CHANNELS.length * DENSITY_COLS * DENSITY_ROWS * 2; // 1172

/** Global corner-grid side length for a square world (401 for Everon 12800). */
export function cornerGridSize(worldSizeM) {
  return Math.floor(worldSizeM / DENSITY_CELL_M) + 1;
}

/** Global corner index of a coordinate (half-open window [corner-16, corner+16)). */
export function cornerOf(coord, worldSizeM) {
  const n = cornerGridSize(worldSizeM);
  const g = Math.floor((coord + DENSITY_CELL_M / 2) / DENSITY_CELL_M);
  return Math.max(0, Math.min(n - 1, g));
}

/**
 * Accumulate a global corner grid from instance positions.
 * @param {Iterable<{x: number, y: number}>} rows rounded, in-bounds positions
 * @param {number} worldSizeM
 * @returns {{ grid: Uint32Array, size: number, count: number }} count = rows consumed
 */
export function accumulateCorners(rows, worldSizeM) {
  const size = cornerGridSize(worldSizeM);
  const grid = new Uint32Array(size * size);
  let count = 0;
  for (const r of rows) {
    grid[cornerOf(r.y, worldSizeM) * size + cornerOf(r.x, worldSizeM)]++;
    count++;
  }
  return { grid, size, count };
}

export function sumGrid(grid) {
  let s = 0;
  for (const v of grid) s += v;
  return s;
}

/**
 * Slice one chunk's 17x17 corner window out of a global grid (chunk = 16 cells => corners
 * cx*16 .. cx*16+16 inclusive). Values clamp to u16.
 * @returns {Uint16Array} length 289, row-major (j*17+i)
 */
export function sliceChunkCorners(grid, size, cx, cy) {
  const out = new Uint16Array(DENSITY_COLS * DENSITY_ROWS);
  for (let j = 0; j < DENSITY_ROWS; j++) {
    const gy = cy * (DENSITY_ROWS - 1) + j;
    for (let i = 0; i < DENSITY_COLS; i++) {
      const gx = cx * (DENSITY_COLS - 1) + i;
      const v = gy < size && gx < size ? grid[gy * size + gx] : 0;
      out[j * DENSITY_COLS + i] = Math.min(65535, v);
    }
  }
  return out;
}

/**
 * Encode one chunk's channels into a TBDD buffer.
 * @param {Uint16Array[]} channels length DENSITY_CHANNELS.length, each 289 values
 * @returns {Buffer}
 */
export function encodeTBDD(channels) {
  if (channels.length !== DENSITY_CHANNELS.length) {
    throw new Error(`encodeTBDD: expected ${DENSITY_CHANNELS.length} channels, got ${channels.length}`);
  }
  const buf = Buffer.alloc(TBDD_FILE_BYTES);
  buf.write("TBDD", 0, "ascii");
  buf.writeUInt16LE(TBDD_VERSION, 4);
  buf.writeUInt16LE(DENSITY_CELL_M, 6);
  buf.writeUInt16LE(DENSITY_COLS, 8);
  buf.writeUInt16LE(DENSITY_ROWS, 10);
  buf.writeUInt8(DENSITY_CHANNELS.length, 12);
  for (const [c, ch] of channels.entries()) {
    if (ch.length !== DENSITY_COLS * DENSITY_ROWS) {
      throw new Error(`encodeTBDD: channel ${c} has ${ch.length} values, want ${DENSITY_COLS * DENSITY_ROWS}`);
    }
    const base = TBDD_HEADER_BYTES + c * DENSITY_COLS * DENSITY_ROWS * 2;
    for (let k = 0; k < ch.length; k++) buf.writeUInt16LE(ch[k], base + k * 2);
  }
  return buf;
}

/**
 * Decode a TBDD buffer (endian-safe: explicit readUInt16LE, no typed-array aliasing).
 * @returns {{ magic: string, version: number, cellM: number, cols: number, rows: number,
 *            channelCount: number, channels: Uint16Array[] }}
 */
export function decodeTBDD(buf) {
  const magic = buf.toString("ascii", 0, 4);
  if (magic !== "TBDD") throw new Error(`decodeTBDD: bad magic '${magic}'`);
  const version = buf.readUInt16LE(4);
  const cellM = buf.readUInt16LE(6);
  const cols = buf.readUInt16LE(8);
  const rows = buf.readUInt16LE(10);
  const channelCount = buf.readUInt8(12);
  const channels = [];
  for (let c = 0; c < channelCount; c++) {
    const base = TBDD_HEADER_BYTES + c * cols * rows * 2;
    const ch = new Uint16Array(cols * rows);
    for (let k = 0; k < ch.length; k++) ch[k] = buf.readUInt16LE(base + k * 2);
    channels.push(ch);
  }
  return { magic, version, cellM, cols, rows, channelCount, channels };
}
