// T-090.1.2.8 — Build the unified satellite bundle (tbd-sat v1) from a full-extent ortho.
//
// One asset per terrain satellite: a GLB-style container holding the FULL GPU mip chain
// (base → 1×1, dims = max(1, floor(prev/2)) — the GL mip rule) as lossless WebP (VP8L)
// blocks. The frontend fetches it ONCE, decodes each mip via createImageBitmap, and uploads
// them into a single luma.gl texture — no per-viewport tile HTTP, no BitmapLayer pop-in
// (the T-090.1.2.8 "Reforger map feel" contract). Replaces the 5461-tile pyramid as the
// primary delivery; the pyramid stays on disk as the manifest-flagged fallback.
//
// Container layout (offsets absolute from file start, little-endian):
//   bytes 0..3   magic  ASCII "TBDS"
//   bytes 4..7   u32    formatVersion = 1
//   bytes 8..11  u32    jsonLength
//   bytes 12..   UTF-8 JSON index (mips[].tiles[] with offset/length per VP8L block)
//   then         concatenated WebP blocks
//
// A mip level splits into ceil(dim/tileThreshold) tiles per axis (Everon: level 0 12800² →
// 2×2 of 6400², everything else 1×1). Tiling keeps every block under the VP8L 16383 px cap
// for future terrains, parallelizes the slow lossless encodes across cores at build time,
// and lets the browser decode the base level in parallel workers. Tile x/y are pixel
// offsets in IMAGE space (row 0 = top = north — same orientation contract as the pyramid
// input: no flips anywhere on this path).
//
// Usage:
//   node scripts/map-assets/build-unified-satellite.mjs \
//     --input packages/map-assets/everon/staging/sap/everon-sap-ortho.png \
//     --out   packages/map-assets/everon/satellite/everon-sat.tbd-sat \
//     --terrain everon [--tile-threshold 8192]
//
// Requires magick (ImageMagick 7) + cwebp on PATH (same toolchain as build-tile-pyramid.sh).
import { createHash } from 'node:crypto';
import { execFileSync, execFile } from 'node:child_process';
import { mkdtempSync, rmSync, mkdirSync, readFileSync, writeFileSync, existsSync } from 'node:fs';
import { tmpdir, cpus } from 'node:os';
import { join, dirname, resolve } from 'node:path';
import { promisify } from 'node:util';

const execFileP = promisify(execFile);

function arg(name, fallback) {
  const i = process.argv.indexOf(`--${name}`);
  return i >= 0 ? process.argv[i + 1] : fallback;
}

const INPUT = arg('input');
const OUT = arg('out');
const TERRAIN = arg('terrain', 'everon');
const TILE_THRESHOLD = Number(arg('tile-threshold', '8192'));

if (!INPUT || !OUT) {
  console.error(
    'Usage: node build-unified-satellite.mjs --input <ortho.png> --out <bundle.tbd-sat> --terrain everon [--tile-threshold 8192]',
  );
  process.exit(2);
}
if (!existsSync(INPUT)) {
  console.error(`input not found: ${INPUT}`);
  process.exit(1);
}
for (const tool of ['magick', 'cwebp']) {
  try {
    execFileSync(tool, ['-version'], { stdio: 'ignore' });
  } catch {
    console.error(`${tool} required on PATH`);
    process.exit(1);
  }
}

const MAGIC = 'TBDS';
const FORMAT_VERSION = 1;

// Terrain contract mirrors packages/map-assets/<terrain>/manifest.json worldBounds.
const WORLD_BOUNDS = {
  everon: [0, 0, 12800, 12800],
  arland: [0, 0, 4096, 4096],
};
const worldBounds = WORLD_BOUNDS[TERRAIN];
if (!worldBounds) {
  console.error(`unknown terrain "${TERRAIN}" (add its worldBounds to build-unified-satellite.mjs)`);
  process.exit(1);
}

const work = mkdtempSync(join(tmpdir(), 'tbd-sat-'));
process.on('exit', () => rmSync(work, { recursive: true, force: true }));

const t0 = Date.now();
const log = (m) => console.log(`[tbd-sat] ${m}`);

// --- 1. Normalize: strip alpha + metadata, force sRGB (same normalize as the pyramid). ---
const norm = join(work, 'mip0.png');
log(`normalizing ${INPUT}`);
execFileSync('magick', [INPUT, '-alpha', 'off', '-colorspace', 'sRGB', norm]);
const [srcW, srcH] = execFileSync('magick', ['identify', '-format', '%w %h', norm])
  .toString()
  .trim()
  .split(' ')
  .map(Number);
log(`source ${srcW}x${srcH}; tileThreshold=${TILE_THRESHOLD}`);

// --- 2. Mip chain dims: base → 1×1 with the GL rule so mipCount always equals
//        floor(log2(max(w,h)))+1 and lines up with luma.gl getMipLevelCount. ---
const levels = [];
for (let w = srcW, h = srcH; ; w = Math.max(1, Math.floor(w / 2)), h = Math.max(1, Math.floor(h / 2))) {
  levels.push({ width: w, height: h });
  if (w === 1 && h === 1) break;
}
log(`mip chain: ${levels.length} levels (${srcW} → 1)`);

// --- 3. Per level: cascade-halve from the previous level PNG (standard mip practice — one
//        full-res Lanczos pass then successive halvings), crop over-threshold levels into a
//        tile grid, and encode every tile as lossless WebP. Encodes are fanned out across
//        cores; level-0 quadrants are what parallelize the dominant cost. ---
const encodeJobs = []; // {level, gridX, gridY, png, webp}
for (let level = 0; level < levels.length; level++) {
  const { width, height } = levels[level];
  const lvPng = join(work, `mip${level}.png`);
  if (level > 0) {
    execFileSync('magick', [join(work, `mip${level - 1}.png`), '-resize', `${width}x${height}!`, lvPng]);
  }
  const cols = Math.ceil(width / TILE_THRESHOLD);
  const rows = Math.ceil(height / TILE_THRESHOLD);
  const tileW = Math.ceil(width / cols);
  const tileH = Math.ceil(height / rows);
  const lvDir = join(work, `level-${level}`);
  mkdirSync(lvDir);
  if (cols === 1 && rows === 1) {
    encodeJobs.push({ level, gridX: 0, gridY: 0, w: width, h: height, png: lvPng, webp: join(lvDir, 't0-0.webp') });
  } else {
    // magick -crop WxH +adjoin slices row-major: index i → col i%cols, row i/cols.
    execFileSync('magick', [lvPng, '-crop', `${tileW}x${tileH}`, '+repage', '+adjoin', join(lvDir, 'tile_%d.png')]);
    for (let gy = 0; gy < rows; gy++) {
      for (let gx = 0; gx < cols; gx++) {
        const w = Math.min(tileW, width - gx * tileW);
        const h = Math.min(tileH, height - gy * tileH);
        encodeJobs.push({
          level,
          gridX: gx,
          gridY: gy,
          w,
          h,
          png: join(lvDir, `tile_${gy * cols + gx}.png`),
          webp: join(lvDir, `t${gx}-${gy}.webp`),
        });
      }
    }
  }
  levels[level].cols = cols;
  levels[level].rows = rows;
  levels[level].tileW = tileW;
  levels[level].tileH = tileH;
}

const parallel = Math.max(1, cpus().length - 1);
log(`encoding ${encodeJobs.length} VP8L blocks (parallel=${parallel})`);
{
  const queue = [...encodeJobs];
  const worker = async () => {
    for (let job = queue.shift(); job; job = queue.shift()) {
      await execFileP('cwebp', ['-quiet', '-lossless', '-mt', job.png, '-o', job.webp]);
    }
  };
  await Promise.all(Array.from({ length: parallel }, worker));
}

// --- 4. Assemble container. Compute the JSON with offset placeholders sized by a two-pass
//        layout: payload offsets depend on jsonLength, so lay blocks out relative to the
//        payload start first, then rewrite as absolute once the JSON byte length is fixed
//        (offsets grow the JSON, so iterate until stable — bounded, tiny). ---
const inputSha256 = createHash('sha256').update(readFileSync(INPUT)).digest('hex');
let sourceMeta;
const metaPath = join(dirname(INPUT), 'TBD_SatExport_meta.json');
if (existsSync(metaPath)) {
  const m = JSON.parse(readFileSync(metaPath, 'utf8'));
  sourceMeta = { source: m.source, seamRepair: m.seamRepair, generatedAt: m.generatedAt };
}

const blocks = encodeJobs.map((j) => ({ job: j, buf: readFileSync(j.webp) }));
const mips = levels.map((lv, level) => ({
  level,
  width: lv.width,
  height: lv.height,
  tiles: blocks
    .filter((b) => b.job.level === level)
    .sort((a, b) => a.job.gridY - b.job.gridY || a.job.gridX - b.job.gridX)
    .map((b) => ({
      x: b.job.gridX * lv.tileW,
      y: b.job.gridY * lv.tileH,
      width: b.job.w,
      height: b.job.h,
      offset: 0, // patched below
      length: b.buf.length,
    })),
}));

const index = {
  formatVersion: FORMAT_VERSION,
  terrainId: TERRAIN,
  worldBounds,
  metersPerPixel: worldBounds[2] / srcW,
  source: sourceMeta?.source ?? 'unknown',
  sourceMeta,
  encoding: 'webp-lossless',
  createdAt: new Date().toISOString(),
  inputSha256,
  baseWidthPx: srcW,
  baseHeightPx: srcH,
  mipCount: levels.length,
  mips,
};

// Iterate offset patching until the JSON length stops moving (number widths stabilize).
const orderedTiles = mips.flatMap((m) => m.tiles);
let jsonBuf;
for (let jsonLen = 0; ; ) {
  let offset = 12 + jsonLen;
  for (const t of orderedTiles) {
    t.offset = offset;
    offset += t.length;
  }
  jsonBuf = Buffer.from(JSON.stringify(index), 'utf8');
  if (jsonBuf.length === jsonLen) break;
  jsonLen = jsonBuf.length;
}

const header = Buffer.alloc(12);
header.write(MAGIC, 0, 'ascii');
header.writeUInt32LE(FORMAT_VERSION, 4);
header.writeUInt32LE(jsonBuf.length, 8);

mkdirSync(dirname(resolve(OUT)), { recursive: true });
writeFileSync(OUT, Buffer.concat([header, jsonBuf, ...blocks.map((b) => b.buf)]));

const total = 12 + jsonBuf.length + blocks.reduce((s, b) => s + b.buf.length, 0);
for (const m of mips) {
  const bytes = m.tiles.reduce((s, t) => s + t.length, 0);
  log(`  level ${String(m.level).padStart(2)}  ${String(m.width).padStart(5)}px  ${m.tiles.length} block(s)  ${(bytes / 1e6).toFixed(2)} MB`);
}
log(`wrote ${OUT}  ${(total / 1e6).toFixed(1)} MB in ${((Date.now() - t0) / 1000).toFixed(0)}s`);
log(`manifest block:`);
console.log(
  JSON.stringify(
    {
      delivery: 'unified',
      unified: {
        path: `satellite/${OUT.split('/').pop()}`,
        url: `/map-assets/${TERRAIN}/satellite/${OUT.split('/').pop()}`,
        encoding: 'tbd-sat-v1',
        baseWidthPx: srcW,
        baseHeightPx: srcH,
        mipCount: levels.length,
        bytes: total,
      },
    },
    null,
    2,
  ),
);
