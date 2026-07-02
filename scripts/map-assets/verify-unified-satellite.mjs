// T-090.1.2.8 — Verify the unified satellite bundle (tbd-sat v1; dep-free, CI-portable).
//
// Gates:
//   1. bundle exists AND is a real binary (a git-lfs pointer file fails with a
//      "run `git lfs pull`" hint)
//   2. container parses: "TBDS" magic, formatVersion 1, JSON index within bounds
//   3. index contract: terrainId + worldBounds match manifest.json; base dims match the
//      terrain; mip chain halves with the GL rule (max(1, floor(d/2))) down to 1×1 and
//      mipCount === floor(log2(base))+1 (the luma.gl getMipLevelCount alignment)
//   4. every tile block: offset/length inside the file, RIFF/WEBP magic, VP8L fourcc
//      (lossless gate — a VP8 lossy block fails), VP8L frame-header dims === declared
//      dims, and each level's tile grid covers the level exactly (no gaps/overlaps)
//   5. manifest agreement: tiles.satellite.delivery === "unified" and
//      unified.{path,url,encoding,baseWidthPx,baseHeightPx,mipCount,bytes} match the file
//
// No image deps: the WebP magic + VP8L frame-header parse is the same hand-rolled reader
// as verify-tile-pyramid.mjs. Usage: node scripts/map-assets/verify-unified-satellite.mjs TERRAIN=everon
import { readFileSync, existsSync, statSync } from "node:fs";

const TERRAIN =
  process.env.TERRAIN ?? process.argv.find((a) => a.startsWith("TERRAIN="))?.split("=")[1] ?? "everon";

const ROOT = new URL("../../packages/map-assets/", import.meta.url).pathname;
const MANIFEST = `${ROOT}${TERRAIN}/manifest.json`;

const errors = [];
const fail = (m) => errors.push(m);
const die = (m) => {
  console.error(`verify-unified-satellite: FAIL — ${m}`);
  process.exit(1);
};

if (!existsSync(MANIFEST)) die(`manifest missing ${MANIFEST}`);
const manifest = JSON.parse(readFileSync(MANIFEST, "utf8"));
const sat = manifest.tiles?.satellite ?? {};
const unified = sat.unified ?? {};

const BUNDLE = `${ROOT}${TERRAIN}/${unified.path ?? `satellite/${TERRAIN}-sat.tbd-sat`}`;

// Gate 1 — real binary present (LFS pointers start with "version https://git-lfs...").
if (!existsSync(BUNDLE)) die(`bundle missing ${BUNDLE} (build it, then check .gitattributes LFS rule)`);
const buf = readFileSync(BUNDLE);
if (buf.length < 64 || buf.toString("ascii", 0, 12).startsWith("version http"))
  die(`${BUNDLE} is a git-lfs pointer, not the bundle — run \`git lfs pull\``);

// Gate 2 — container header.
if (buf.toString("ascii", 0, 4) !== "TBDS") die(`bad magic (expected "TBDS")`);
const version = buf.readUInt32LE(4);
if (version !== 1) die(`unsupported formatVersion ${version} (expected 1)`);
const jsonLen = buf.readUInt32LE(8);
if (12 + jsonLen > buf.length) die(`jsonLength ${jsonLen} overruns file (${buf.length} bytes)`);
let index;
try {
  index = JSON.parse(buf.toString("utf8", 12, 12 + jsonLen));
} catch (e) {
  die(`JSON index unparseable: ${e.message}`);
}

// Gate 3 — index contract vs manifest + GL mip rule.
if (index.formatVersion !== 1) fail(`index.formatVersion ${index.formatVersion} !== 1`);
if (index.terrainId !== TERRAIN) fail(`index.terrainId ${index.terrainId} !== ${TERRAIN}`);
const wb = manifest.worldBounds;
if (JSON.stringify(index.worldBounds) !== JSON.stringify(wb))
  fail(`index.worldBounds ${JSON.stringify(index.worldBounds)} !== manifest ${JSON.stringify(wb)}`);
if (index.encoding !== "webp-lossless") fail(`index.encoding ${index.encoding} !== webp-lossless`);
const expectedMipCount = Math.floor(Math.log2(Math.max(index.baseWidthPx, index.baseHeightPx))) + 1;
if (index.mipCount !== expectedMipCount)
  fail(`mipCount ${index.mipCount} !== floor(log2(base))+1 = ${expectedMipCount}`);
if (index.mips?.length !== index.mipCount)
  fail(`mips[] length ${index.mips?.length} !== mipCount ${index.mipCount}`);

let w = index.baseWidthPx;
let h = index.baseHeightPx;
for (const [i, mip] of (index.mips ?? []).entries()) {
  if (mip.level !== i) fail(`mips[${i}].level = ${mip.level} (must be ${i})`);
  if (mip.width !== w || mip.height !== h)
    fail(`level ${i}: ${mip.width}x${mip.height}, GL rule expects ${w}x${h}`);
  w = Math.max(1, Math.floor(w / 2));
  h = Math.max(1, Math.floor(h / 2));
}
const last = index.mips?.[index.mips.length - 1];
if (last && (last.width !== 1 || last.height !== 1)) fail(`chain must end at 1x1 (got ${last.width}x${last.height})`);

// --- WebP block reader (same parse as verify-tile-pyramid.mjs) ---
function webpDims(b) {
  if (b.length < 30) return { ok: false };
  if (b.toString("ascii", 0, 4) !== "RIFF" || b.toString("ascii", 8, 12) !== "WEBP") return { ok: false };
  const fourcc = b.toString("ascii", 12, 16);
  if (fourcc === "VP8L") {
    if (b[20] !== 0x2f) return { ok: true, fourcc, w: 0, h: 0 };
    const v = b.readUInt32LE(21);
    return { ok: true, fourcc, w: (v & 0x3fff) + 1, h: ((v >> 14) & 0x3fff) + 1 };
  }
  if (fourcc === "VP8 ") {
    if (b[23] !== 0x9d || b[24] !== 0x01 || b[25] !== 0x2a) return { ok: true, fourcc, w: 0, h: 0 };
    return { ok: true, fourcc, w: (b[26] | (b[27] << 8)) & 0x3fff, h: (b[28] | (b[29] << 8)) & 0x3fff };
  }
  return { ok: true, fourcc, w: 0, h: 0 };
}

// Gate 4 — every block valid VP8L with declared dims; grids cover each level exactly.
let blockCount = 0;
let payloadBytes = 0;
for (const mip of index.mips ?? []) {
  const seen = new Set();
  let covered = 0;
  for (const t of mip.tiles ?? []) {
    blockCount++;
    payloadBytes += t.length;
    if (t.offset < 12 + jsonLen || t.offset + t.length > buf.length) {
      fail(`level ${mip.level} tile @(${t.x},${t.y}): offset ${t.offset}+${t.length} out of range`);
      continue;
    }
    const d = webpDims(buf.subarray(t.offset, t.offset + Math.min(t.length, 64)));
    if (!d.ok) fail(`level ${mip.level} tile @(${t.x},${t.y}): not a RIFF/WEBP block`);
    else if (d.fourcc !== "VP8L") fail(`level ${mip.level} tile @(${t.x},${t.y}): ${d.fourcc}, expected VP8L (lossless)`);
    else if (d.w !== t.width || d.h !== t.height)
      fail(`level ${mip.level} tile @(${t.x},${t.y}): VP8L says ${d.w}x${d.h}, index says ${t.width}x${t.height}`);
    const key = `${t.x},${t.y}`;
    if (seen.has(key)) fail(`level ${mip.level}: duplicate tile @(${key})`);
    seen.add(key);
    if (t.x < 0 || t.y < 0 || t.x + t.width > mip.width || t.y + t.height > mip.height)
      fail(`level ${mip.level} tile @(${t.x},${t.y}) ${t.width}x${t.height} exceeds level ${mip.width}x${mip.height}`);
    covered += t.width * t.height;
  }
  if (covered !== mip.width * mip.height)
    fail(`level ${mip.level}: tiles cover ${covered}px², level is ${mip.width * mip.height}px² (gap/overlap)`);
}
if (12 + jsonLen + payloadBytes !== buf.length)
  fail(`payload bytes ${payloadBytes} + header ${12 + jsonLen} !== file size ${buf.length}`);

// Gate 5 — manifest agreement.
if (sat.delivery !== "unified") fail(`manifest tiles.satellite.delivery "${sat.delivery}" !== "unified"`);
if (unified.encoding !== "tbd-sat-v1") fail(`manifest unified.encoding "${unified.encoding}" !== "tbd-sat-v1"`);
if (unified.url && !unified.url.includes(`/${TERRAIN}/${unified.path}`))
  fail(`manifest unified.url ${unified.url} does not point at ${TERRAIN}/${unified.path}`);
if (unified.baseWidthPx !== index.baseWidthPx || unified.baseHeightPx !== index.baseHeightPx)
  fail(`manifest unified base ${unified.baseWidthPx}x${unified.baseHeightPx} !== bundle ${index.baseWidthPx}x${index.baseHeightPx}`);
if (unified.mipCount !== index.mipCount) fail(`manifest unified.mipCount ${unified.mipCount} !== bundle ${index.mipCount}`);
const size = statSync(BUNDLE).size;
if (unified.bytes !== size) fail(`manifest unified.bytes ${unified.bytes} !== file size ${size}`);

if (errors.length) {
  console.error(`verify-unified-satellite: FAIL (${errors.length}) for ${TERRAIN}`);
  for (const e of errors) console.error(`  - ${e}`);
  process.exit(1);
}
console.log(
  `verify-unified-satellite: OK ${TERRAIN} — ${index.baseWidthPx}x${index.baseHeightPx}, ${index.mipCount} mips, ${blockCount} VP8L blocks, ${(size / 1e6).toFixed(1)} MB`,
);
