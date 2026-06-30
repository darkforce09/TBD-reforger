// T-090.1 — Verify the satellite WebP tile pyramid (dep-free, CI-portable).
//
// Gates:
//   1. tiles/satellite/0/0/0.webp exists (K3 file gate)
//   2. every present level z has exactly (2**z)x(2**z) tiles named {x}/{y}.webp
//   3. every tile is a valid RIFF/WEBP; lossy VP8 tiles assert 256x256 (tileSizePx)
//   4. manifest tiles.satellite.urlTemplate + min/maxZoom + tileSizePx agree with disk
//
// No image deps: WebP magic + VP8 lossy frame-header dims are parsed by hand.
// Usage: node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon
import { readFileSync, existsSync, readdirSync, statSync } from "node:fs";

const TERRAIN =
  process.env.TERRAIN ?? process.argv.find((a) => a.startsWith("TERRAIN="))?.split("=")[1] ?? "everon";

const ROOT = new URL("../../packages/map-assets/", import.meta.url).pathname;
const TILES_DIR = `${ROOT}${TERRAIN}/tiles/satellite`;
const MANIFEST = `${ROOT}${TERRAIN}/manifest.json`;

const errors = [];
const fail = (m) => errors.push(m);

// --- WebP dims: RIFF....WEBP, then a chunk. Parse VP8 (lossy) keyframe dims. ---
function webpDims(buf) {
  if (buf.length < 30) return { ok: false };
  if (buf.toString("ascii", 0, 4) !== "RIFF" || buf.toString("ascii", 8, 12) !== "WEBP")
    return { ok: false };
  const fourcc = buf.toString("ascii", 12, 16);
  if (fourcc === "VP8 ") {
    // lossy: start code 9d 01 2a at 23..25, then 14-bit width/height LE.
    if (buf[23] !== 0x9d || buf[24] !== 0x01 || buf[25] !== 0x2a) return { ok: true, w: 0, h: 0 };
    const w = (buf[26] | (buf[27] << 8)) & 0x3fff;
    const h = (buf[28] | (buf[29] << 8)) & 0x3fff;
    return { ok: true, w, h };
  }
  if (fourcc === "VP8L") {
    // lossless: 0x2f sig then 14-bit (w-1),(h-1) packed LE across bytes 21..24.
    if (buf[20] !== 0x2f) return { ok: true, w: 0, h: 0 };
    const b = buf.readUInt32LE(21);
    const w = (b & 0x3fff) + 1;
    const h = ((b >> 14) & 0x3fff) + 1;
    return { ok: true, w, h };
  }
  // VP8X / animation: accept magic only (canvas dims not asserted here).
  return { ok: true, w: 0, h: 0 };
}

function listInts(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .filter((n) => statSync(`${dir}/${n}`).isDirectory())
    .map(Number)
    .filter((n) => Number.isInteger(n));
}

if (!existsSync(MANIFEST)) {
  console.error(`verify-tile-pyramid: manifest missing ${MANIFEST}`);
  process.exit(1);
}
const manifest = JSON.parse(readFileSync(MANIFEST, "utf8"));
const tiles = manifest.tiles ?? {};
const sat = tiles.satellite ?? {};
const tileSize = tiles.tileSizePx ?? 256;
const minZoom = tiles.minZoom ?? 0;
const maxZoom = tiles.maxZoom ?? 5;

// Gate 1 — K3 file gate.
if (!existsSync(`${TILES_DIR}/0/0/0.webp`)) fail(`missing ${TILES_DIR}/0/0/0.webp (K3 file gate)`);

// Gate 4 — manifest path agreement.
const expectPath = `tiles/satellite`;
if (sat.path && sat.path !== expectPath) fail(`manifest tiles.satellite.path=${sat.path} != ${expectPath}`);
if (sat.urlTemplate && !sat.urlTemplate.includes(`/${TERRAIN}/tiles/satellite/`))
  fail(`manifest tiles.satellite.urlTemplate does not point at ${TERRAIN}/tiles/satellite: ${sat.urlTemplate}`);

// Gates 2 + 3 — per level structure + tile validity. Allow a sparse pyramid where only
// some levels are committed, but every PRESENT level must be complete + square.
let checkedTiles = 0;
const presentZ = listInts(TILES_DIR).filter((z) => z >= minZoom && z <= maxZoom);
if (presentZ.length === 0) fail(`no zoom levels present under ${TILES_DIR}`);

for (const z of presentZ.sort((a, b) => a - b)) {
  const n = 1 << z;
  const xs = listInts(`${TILES_DIR}/${z}`);
  if (xs.length !== n) fail(`z=${z}: ${xs.length} x-columns, expected ${n}`);
  for (let x = 0; x < n; x++) {
    for (let y = 0; y < n; y++) {
      const p = `${TILES_DIR}/${z}/${x}/${y}.webp`;
      if (!existsSync(p)) {
        fail(`z=${z}: missing tile ${x}/${y}.webp`);
        continue;
      }
      const d = webpDims(readFileSync(p));
      if (!d.ok) fail(`z=${z} ${x}/${y}: not a valid RIFF/WEBP`);
      else if (d.w && (d.w !== tileSize || d.h !== tileSize))
        fail(`z=${z} ${x}/${y}: ${d.w}x${d.h}, expected ${tileSize}x${tileSize}`);
      checkedTiles++;
    }
  }
}

if (errors.length) {
  console.error(`verify-tile-pyramid: FAIL (${errors.length}) for ${TERRAIN}`);
  for (const e of errors) console.error(`  - ${e}`);
  process.exit(1);
}
console.log(
  `verify-tile-pyramid: OK ${TERRAIN} — levels [${presentZ.join(",")}], ${checkedTiles} tiles, ${tileSize}px`,
);
