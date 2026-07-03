// T-090.1 — Verify the satellite WebP tile pyramid (dep-free, CI-portable).
//
// Gates:
//   1. tiles/satellite/0/0/0.webp exists (K3 file gate)
//   2. the pyramid is COMPLETE: every level z in [minZoom, maxZoom] exists with exactly
//      (2**z)x(2**z) tiles named {x}/{y}.webp (no sparse levels — T-090.1.2.1)
//   3. every tile is a valid RIFF/WEBP @ tileSizePx; when lossless is expected
//      (manifest tiles.satellite.encoding === "webp-lossless" or EXPECT_LOSSLESS=1) every
//      tile must be a VP8L chunk — a VP8 lossy tile fails
//   4. manifest tiles.satellite.urlTemplate + min/maxZoom + tileSizePx agree with disk
//
// No image deps: WebP magic + VP8/VP8L frame-header dims + codec fourcc are parsed by hand.
// Usage: node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon
//        VIEW=map node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon
//        EXPECT_LOSSLESS=1 node scripts/map-assets/verify-tile-pyramid.mjs TERRAIN=everon
import { readFileSync, existsSync, readdirSync, statSync } from "node:fs";

const TERRAIN =
  process.env.TERRAIN ?? process.argv.find((a) => a.startsWith("TERRAIN="))?.split("=")[1] ?? "everon";
const VIEW = process.env.VIEW === "map" ? "map" : "satellite";

const ROOT = new URL("../../packages/map-assets/", import.meta.url).pathname;
const TILES_DIR = `${ROOT}${TERRAIN}/tiles/${VIEW}`;
const MANIFEST = `${ROOT}${TERRAIN}/manifest.json`;

const errors = [];
const fail = (m) => errors.push(m);

// Tile pyramids are local build output (gitignored). CI / fresh clones skip — unified bundle is primary.
if (!existsSync(TILES_DIR)) {
  console.log(
    `verify-tile-pyramid: SKIP ${TERRAIN}/${VIEW} — no pyramid on disk (local rebuild: make map-water-everon or build-tile-pyramid.sh)`,
  );
  process.exit(0);
}

// --- WebP dims + codec: RIFF....WEBP, then a chunk. fourcc distinguishes lossy (VP8 ) from
// lossless (VP8L); the EXPECT_LOSSLESS gate below fails on VP8 . Parse keyframe dims too. ---
function webpDims(buf) {
  if (buf.length < 30) return { ok: false };
  if (buf.toString("ascii", 0, 4) !== "RIFF" || buf.toString("ascii", 8, 12) !== "WEBP")
    return { ok: false };
  const fourcc = buf.toString("ascii", 12, 16);
  if (fourcc === "VP8 ") {
    // lossy: start code 9d 01 2a at 23..25, then 14-bit width/height LE.
    if (buf[23] !== 0x9d || buf[24] !== 0x01 || buf[25] !== 0x2a) return { ok: true, fourcc, w: 0, h: 0 };
    const w = (buf[26] | (buf[27] << 8)) & 0x3fff;
    const h = (buf[28] | (buf[29] << 8)) & 0x3fff;
    return { ok: true, fourcc, w, h };
  }
  if (fourcc === "VP8L") {
    // lossless: 0x2f sig then 14-bit (w-1),(h-1) packed LE across bytes 21..24.
    if (buf[20] !== 0x2f) return { ok: true, fourcc, w: 0, h: 0 };
    const b = buf.readUInt32LE(21);
    const w = (b & 0x3fff) + 1;
    const h = ((b >> 14) & 0x3fff) + 1;
    return { ok: true, fourcc, w, h };
  }
  // VP8X / animation: accept magic only (canvas dims not asserted here).
  return { ok: true, fourcc, w: 0, h: 0 };
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
const viewBlock = VIEW === "map" ? tiles.map : tiles.satellite;
const sat = tiles.satellite ?? {};
const tileSize = tiles.tileSizePx ?? 256;
const minZoom = tiles.minZoom ?? 0;
const maxZoom = tiles.maxZoom ?? 5;
// T-090.1.2.1 — when the satellite pyramid is advertised lossless (manifest encoding or the
// EXPECT_LOSSLESS env), every tile must be VP8L; a VP8 (lossy) tile fails the gate.
const expectLossless =
  VIEW === "satellite" &&
  (process.env.EXPECT_LOSSLESS === "1" || sat.encoding === "webp-lossless");

// Gate 1 — K3 file gate.
if (!existsSync(`${TILES_DIR}/0/0/0.webp`)) fail(`missing ${TILES_DIR}/0/0/0.webp (K3 file gate)`);

// Gate 4 — manifest path agreement.
const expectPath = `tiles/${VIEW}`;
if (viewBlock?.path && viewBlock.path !== expectPath)
  fail(`manifest tiles.${VIEW}.path=${viewBlock.path} != ${expectPath}`);
if (viewBlock?.urlTemplate && !viewBlock.urlTemplate.includes(`/${TERRAIN}/tiles/${VIEW}/`))
  fail(
    `manifest tiles.${VIEW}.urlTemplate does not point at ${TERRAIN}/tiles/${VIEW}: ${viewBlock.urlTemplate}`,
  );

// Gates 2 + 3 — COMPLETE pyramid + tile validity (T-090.1.2.1). Every level in
// [minZoom, maxZoom] must exist and be exactly (2**z)x(2**z) — no sparse levels — so the
// manifest can never advertise a maxZoom whose tiles aren't all on disk. When lossless is
// expected, every tile must be a VP8L chunk (a VP8 lossy tile fails).
let checkedTiles = 0;
let losslessChecked = 0;
const levels = [];
for (let z = minZoom; z <= maxZoom; z++) {
  const n = 1 << z;
  if (!existsSync(`${TILES_DIR}/${z}`)) {
    fail(`z=${z}: level missing (pyramid must be complete [${minZoom}..${maxZoom}])`);
    continue;
  }
  levels.push(z);
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
      if (!d.ok) {
        fail(`z=${z} ${x}/${y}: not a valid RIFF/WEBP`);
      } else {
        if (d.w && (d.w !== tileSize || d.h !== tileSize))
          fail(`z=${z} ${x}/${y}: ${d.w}x${d.h}, expected ${tileSize}x${tileSize}`);
        if (expectLossless) {
          if (d.fourcc === "VP8 ") fail(`z=${z} ${x}/${y}: VP8 lossy chunk, expected VP8L (lossless)`);
          else if (d.fourcc === "VP8L") losslessChecked++;
        }
      }
      checkedTiles++;
    }
  }
}

if (errors.length) {
  console.error(`verify-tile-pyramid: FAIL (${errors.length}) for ${TERRAIN}`);
  for (const e of errors) console.error(`  - ${e}`);
  process.exit(1);
}
const losslessNote = expectLossless ? `, ${losslessChecked} VP8L lossless` : "";
console.log(
  `verify-tile-pyramid: OK ${TERRAIN} — levels [${levels.join(",")}], ${checkedTiles} tiles, ${tileSize}px${losslessNote}`,
);
