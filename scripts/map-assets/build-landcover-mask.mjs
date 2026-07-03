// T-090.1.1.1 — Land-cover masks for the Map cartographic compose (L1 winner,
// .ai/artifacts/t090_1_1_1_source_spike.json).
//
// The G1-A MapDataExporter TGA is monochrome on land (all 5.28M land px sit in a single
// relief-shaded olive band R∈[68,105); the plugin's forestArea palette never rendered), so
// forest/field readability must come from the SAP satellite ortho — the only offline source
// that actually encodes land cover. This script classifies the SAP appearance into two soft
// masks the compose overlays as tint alphas:
//
//   forest : dark canopy green   — g > r, g > b+8, L ≤ 52   (sampled forest ≈ (41,46,30),
//            dark conifer/scrub bands ≈ (53,58,36), grass ≈ (62,66,41) — cut above the
//            conifer band, below grass; SAP land is unlit supertexture → island-wide stable)
//   bright : sun-tan fields / plow / urban — red-dominant (r ≥ g+4) ∧ L ≥ 58
//            (tan ≈ (105,93,66) r−g=+12, plow ≈ (72,66,46) +6, town roofs +12; grass is
//            green-dominant r−g≈−4, and grey rock/haze r≈g stays out on purpose — bare
//            mountain reads as base olive until a real land-cover export lands)
//   water px (b ≥ g) are excluded outright: ocean/lakes/rivers get their own tint pass.
//
// Classification runs at CLASS_PX (4 m/px — magick -sample, no averaging, so thresholds see
// true pixel values). Raw per-pixel classes are salt-and-pepper at village/copse scale, so
// each plane goes through a close-then-open (grow @ 0.35 → shrink @ 0.6 via box blur +
// re-threshold, ~25 m radius) to get chunky Google-Maps-scale regions, then a final soft
// blur turns the hard edge into an alpha ramp. Masks are written at CLASS_PX and upscaled
// by the compose's magick pass; alpha is capped there (TINT strength), not here.
//
// Usage:  node scripts/map-assets/build-landcover-mask.mjs            (TERRAIN=everon)
// Output: packages/map-assets/<terrain>/staging/map/landcover-forest-mask.png
//         packages/map-assets/<terrain>/staging/map/landcover-bright-mask.png
//         packages/map-assets/<terrain>/staging/map/landcover-mask-meta.json
import { execFileSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = join(HERE, '../..');
const require = createRequire(join(REPO, 'packages/tbd-schema/package.json'));
const { PNG } = require('pngjs');

export const CLASS_PX = 3200; // 4 m/px classification grid (12800 m world)

const SOURCES = {
  everon: {
    sap: 'packages/map-assets/everon/staging/sap/everon-sap-ortho.png',
    outDir: 'packages/map-assets/everon/staging/map',
  },
};

// Thresholds from the P0 sample boxes (spike JSON §heuristic).
const FOREST_LUM_MAX = 52;
const FOREST_GREEN_OVER_BLUE = 8;
const BRIGHT_RED_OVER_GREEN = 4;
const BRIGHT_LUM_MIN = 58;

function boxBlur(src, w, h, radius) {
  // Two-pass separable box blur on a Float32 plane (same shape the water spike used).
  const tmp = new Float32Array(src.length);
  const dst = new Float32Array(src.length);
  const win = 2 * radius + 1;
  for (let y = 0; y < h; y++) {
    let acc = 0;
    const row = y * w;
    for (let x = -radius; x <= radius; x++) acc += src[row + Math.min(w - 1, Math.max(0, x))];
    for (let x = 0; x < w; x++) {
      tmp[row + x] = acc / win;
      const add = Math.min(w - 1, x + radius + 1);
      const sub = Math.max(0, x - radius);
      acc += src[row + add] - src[row + sub];
    }
  }
  for (let x = 0; x < w; x++) {
    let acc = 0;
    for (let y = -radius; y <= radius; y++) acc += tmp[Math.min(h - 1, Math.max(0, y)) * w + x];
    for (let y = 0; y < h; y++) {
      dst[y * w + x] = acc / win;
      const add = Math.min(h - 1, y + radius + 1);
      const sub = Math.max(0, y - radius);
      acc += tmp[add * w + x] - tmp[sub * w + x];
    }
  }
  return dst;
}

function writeMask(plane, w, h, outPath) {
  const png = new PNG({ width: w, height: h, colorType: 0 });
  for (let i = 0; i < plane.length; i++) {
    const v = Math.round(Math.max(0, Math.min(1, plane[i])) * 255);
    const o = i * 4;
    png.data[o] = v;
    png.data[o + 1] = v;
    png.data[o + 2] = v;
    png.data[o + 3] = 255;
  }
  writeFileSync(outPath, PNG.sync.write(png, { colorType: 0 }));
}

export function buildLandcoverMasks(terrain) {
  const cfg = SOURCES[terrain];
  if (!cfg) throw new Error(`build-landcover-mask: no SAP source registered for terrain "${terrain}"`);
  const sap = join(REPO, cfg.sap);
  if (!existsSync(sap)) {
    throw new Error(
      `build-landcover-mask: SAP ortho missing: ${cfg.sap}\n` +
        `staging/ is gitignored — restore it (make map-water-everon rebuilds the water composite).`,
    );
  }
  const outDir = join(REPO, cfg.outDir);
  mkdirSync(outDir, { recursive: true });

  // -sample (nearest), not -resize: classification thresholds were tuned on true pixel
  // values; averaging would smear forest/field boundaries into unclassifiable midtones.
  const started = Date.now();
  const tmpPng = join(tmpdir(), `landcover-sample-${process.pid}.png`);
  execFileSync('magick', [sap, '-sample', `${CLASS_PX}x${CLASS_PX}`, tmpPng], { stdio: 'inherit' });
  const img = PNG.sync.read(readFileSync(tmpPng));
  rmSync(tmpPng);
  if (img.width !== CLASS_PX || img.height !== CLASS_PX)
    throw new Error(`build-landcover-mask: sample is ${img.width}x${img.height}, want ${CLASS_PX}`);

  const n = CLASS_PX * CLASS_PX;
  let forest = new Float32Array(n);
  let bright = new Float32Array(n);
  const counts = { forest: 0, bright: 0, water: 0, grass: 0 };
  for (let i = 0; i < n; i++) {
    const o = i * 4;
    const r = img.data[o];
    const g = img.data[o + 1];
    const b = img.data[o + 2];
    if (b >= g) {
      counts.water++;
      continue; // ocean / lakes / rivers — the water pass owns these
    }
    const L = (r + g + b) / 3;
    if (g > r && g > b + FOREST_GREEN_OVER_BLUE && L <= FOREST_LUM_MAX) {
      forest[i] = 1;
      counts.forest++;
    } else if (r >= g + BRIGHT_RED_OVER_GREEN && L >= BRIGHT_LUM_MIN) {
      bright[i] = 1;
      counts.bright++;
    } else {
      counts.grass++;
    }
  }

  // Close-then-open per plane: grow (low threshold) bridges in-patch texture gaps, shrink
  // (high threshold) drops isolated speckle — chunky regions, no salt-and-pepper. Radius 6
  // @ 4 m/px ≈ 25 m structuring element. Final r=2 blur = soft alpha ramp at region edges.
  const closeOpen = (plane) => {
    let p = boxBlur(plane, CLASS_PX, CLASS_PX, 6);
    for (let i = 0; i < n; i++) p[i] = p[i] >= 0.35 ? 1 : 0;
    p = boxBlur(p, CLASS_PX, CLASS_PX, 6);
    for (let i = 0; i < n; i++) p[i] = p[i] >= 0.6 ? 1 : 0;
    return boxBlur(p, CLASS_PX, CLASS_PX, 2);
  };
  forest = closeOpen(forest);
  bright = closeOpen(bright);

  const forestOut = join(outDir, 'landcover-forest-mask.png');
  const brightOut = join(outDir, 'landcover-bright-mask.png');
  writeMask(forest, CLASS_PX, CLASS_PX, forestOut);
  writeMask(bright, CLASS_PX, CLASS_PX, brightOut);

  const meta = {
    slice: 'T-090.1.1.1',
    source: cfg.sap,
    classPx: CLASS_PX,
    thresholds: {
      water: 'b >= g (excluded)',
      forest: `g > r && g > b+${FOREST_GREEN_OVER_BLUE} && L <= ${FOREST_LUM_MAX}`,
      bright: `r >= g+${BRIGHT_RED_OVER_GREEN} && L >= ${BRIGHT_LUM_MIN}`,
    },
    fractions: {
      forest: +(counts.forest / n).toFixed(4),
      bright: +(counts.bright / n).toFixed(4),
      grass: +(counts.grass / n).toFixed(4),
      water: +(counts.water / n).toFixed(4),
    },
    buildSeconds: Math.round((Date.now() - started) / 1000),
    generatedAt: new Date().toISOString(),
  };
  writeFileSync(join(outDir, 'landcover-mask-meta.json'), `${JSON.stringify(meta, null, 2)}\n`);
  return { forestMask: forestOut, brightMask: brightOut, meta };
}

// CLI entry (compose imports buildLandcoverMasks directly, same pattern as decode-topo.mjs).
if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const terrain = process.env.TERRAIN || 'everon';
  const { meta } = buildLandcoverMasks(terrain);
  console.log(
    `build-landcover-mask: OK ${terrain} @ ${CLASS_PX}² — fractions ` +
      `forest=${meta.fractions.forest} bright=${meta.fractions.bright} ` +
      `grass=${meta.fractions.grass} water=${meta.fractions.water} (${meta.buildSeconds}s)`,
  );
}
