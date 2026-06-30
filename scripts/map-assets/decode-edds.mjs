// T-090.1.2 — Enfusion `_supertexture.edds` decoder (Everon / "Eden" SAP cells).
//
// Pipeline, all proven on real Everon (data_worlds.pak) in the P0 spike:
//   pak (enfusion-mcp PakVirtualFS, zlib-inflated)
//     -> EDDS header   : dxgiFormat = u32LE @ 0x48 (99 = BC7_UNORM_SRGB)
//     -> chunk table    : @ 0x5c, records [4B tag][u32LE len], tag in {COPY, 'LZ4 '},
//                         contiguous until first non-tag; mip bodies follow the table.
//     -> mip geometry   : record i side = 1<<i (rec0=1px .. rec8=256px); mip0 = last/largest.
//     -> COPY chunk     : raw BC7 bytes (len bytes).
//     -> 'LZ4 ' chunk   : [u32LE decompSize][u32LE _][raw LZ4 block]  (stream starts at +8).
//     -> BC7 -> RGBA8   : vendored bcdec.wasm (vendor/bc7.mjs).
//
// Everon supertextures are `worlds/Eden/Eden/.Data/Eden_<N>_supertexture.edds`, N=0..2499,
// a 50x50 grid (row-major N = y*50 + x, y=0 north/top), 256 m world / 256 px per cell
// => 1 m/px, full ortho 12800x12800.  Library + CLI (`node decode-edds.mjs <N>` -> raw RGBA
// on stdout + meta on stderr).
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { decodeBc7 } from "./vendor/bc7.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));

// ── Constants (the decode contract) ─────────────────────────────────────────
export const EDEN_DATA_DIR = "worlds/Eden/Eden/.Data";
export const GRID = 50; // 50x50 cells
export const CELL_COUNT = GRID * GRID; // 2500
export const CELL_PX = 256; // mip0 side
export const CELL_M = 256; // world metres per cell
export const WORLD_M = GRID * CELL_M; // 12800
export const DXGI_BC7_UNORM_SRGB = 99;
export const DXGI_BC7_UNORM = 98;

const CHUNK_TABLE_OFFSET = 0x5c;
const DXGI_OFFSET = 0x48;

// ── Pak VFS (enfusion-mcp) ──────────────────────────────────────────────────
let _vfs = null;
export async function getVfs() {
  if (_vfs) return _vfs;
  const { PakVirtualFS } = await import(
    join(HERE, "../mod/node_modules/enfusion-mcp/dist/pak/vfs.js")
  );
  const gamePath =
    process.env.ENFUSION_GAME_PATH ||
    join(process.env.HOME || "", ".cache/enfusion-mcp-root");
  const vfs = PakVirtualFS.get(gamePath);
  if (!vfs) throw new Error(`No pak VFS at ${gamePath} (ENFUSION_GAME_PATH?)`);
  _vfs = vfs;
  return vfs;
}

/** Virtual path for Eden cell N. */
export function cellPath(n) {
  return `${EDEN_DATA_DIR}/Eden_${n}_supertexture.edds`;
}

/** Row-major grid coords for linear index N (x east, y=0 north/top). */
export function cellGrid(n) {
  return { gridX: n % GRID, gridY: Math.floor(n / GRID) };
}

// ── EDDS parsing ────────────────────────────────────────────────────────────
/** Parse the Enfusion EDDS header + chunk table. */
export function parseEdds(buf) {
  const dxgi = buf.readUInt32LE(DXGI_OFFSET);
  let o = CHUNK_TABLE_OFFSET;
  const recs = [];
  while (o + 8 <= buf.length) {
    const tag = buf.toString("ascii", o, o + 4);
    if (tag !== "COPY" && tag !== "LZ4 ") break;
    recs.push({ tag, len: buf.readUInt32LE(o + 4), off: 0 });
    o += 8;
  }
  let cur = o;
  for (const r of recs) {
    r.off = cur;
    cur += r.len;
  }
  return { dxgi, recs, mipCount: recs.length };
}

// ── LZ4 raw block decompressor (byte-identical to liblz4; verified in P0) ────
export function lz4Block(src, dstSize) {
  const out = Buffer.alloc(dstSize);
  let s = 0;
  let d = 0;
  while (s < src.length) {
    const tok = src[s++];
    let ll = tok >> 4;
    if (ll === 15) {
      let x;
      do {
        x = src[s++];
        ll += x;
      } while (x === 255);
    }
    src.copy(out, d, s, s + ll);
    s += ll;
    d += ll;
    if (s >= src.length) break;
    const off = src[s] | (src[s + 1] << 8);
    s += 2;
    let ml = (tok & 15) + 4;
    if ((tok & 15) === 15) {
      let x;
      do {
        x = src[s++];
        ml += x;
      } while (x === 255);
    }
    let m = d - off;
    for (let i = 0; i < ml; i++) out[d++] = out[m++];
  }
  if (d !== dstSize) {
    throw new Error(`LZ4 size mismatch: got ${d}, expected ${dstSize}`);
  }
  return out;
}

/** Raw BC7 bytes for a mip record (COPY = stored, 'LZ4 ' = [u32 size][u32 _][block]). */
export function mipBc7(buf, rec, side) {
  const expected = side * side; // BC7 = 1 byte/px
  if (rec.tag === "COPY") {
    const bc7 = buf.subarray(rec.off, rec.off + rec.len);
    if (bc7.length < expected) {
      throw new Error(`COPY mip short: ${bc7.length} < ${expected}`);
    }
    return bc7;
  }
  const body = buf.subarray(rec.off, rec.off + rec.len);
  const decompSize = body.readUInt32LE(0);
  if (decompSize !== expected) {
    throw new Error(`LZ4 decompSize ${decompSize} != expected ${expected} (side ${side})`);
  }
  return lz4Block(body.subarray(8), decompSize);
}

/** mip0 side from mip count (smallest mip = 1px => largest = 2^(n-1)). */
export function mip0Side(mipCount) {
  return 1 << (mipCount - 1);
}

// ── Cell decode ─────────────────────────────────────────────────────────────
/**
 * Decode cell N's mip0 to RGBA8. Returns { rgba, side, dxgi, mipCount }.
 * Throws on missing/corrupt cells (callers must NOT paper over — no grey fill).
 */
export async function decodeCellRgba(n) {
  const vfs = await getVfs();
  const path = cellPath(n);
  if (!vfs.exists(path)) throw new Error(`cell missing in pak: ${path}`);
  const buf = vfs.readFile(path);
  const { dxgi, recs, mipCount } = parseEdds(buf);
  if (mipCount < 1) throw new Error(`no mip chunks in ${path}`);
  if (dxgi !== DXGI_BC7_UNORM_SRGB && dxgi !== DXGI_BC7_UNORM) {
    throw new Error(`unexpected dxgiFormat ${dxgi} in ${path} (expected BC7 98/99)`);
  }
  const side = mip0Side(mipCount);
  const bc7 = mipBc7(buf, recs[recs.length - 1], side);
  const rgba = decodeBc7(bc7, side, side);
  return { rgba, side, dxgi, mipCount };
}

/** List the 2500 Eden cells present in the pak, sorted by index. */
export async function listEdenCells() {
  const vfs = await getVfs();
  const re = /\/Eden_(\d+)_supertexture\.edds$/;
  const out = [];
  for (const p of vfs.allFilePaths()) {
    const m = p.match(re);
    if (m && p.startsWith(EDEN_DATA_DIR)) out.push({ n: +m[1], path: p });
  }
  out.sort((a, b) => a.n - b.n);
  return out;
}

// ── CLI: `node decode-edds.mjs <N>` -> raw RGBA on stdout, meta on stderr ────
if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const n = Number(process.argv[2] ?? 0);
  const { rgba, side, dxgi, mipCount } = await decodeCellRgba(n);
  process.stderr.write(
    JSON.stringify({ cell: n, side, dxgi, mipCount, rgbaBytes: rgba.length }) + "\n"
  );
  process.stdout.write(rgba);
}
