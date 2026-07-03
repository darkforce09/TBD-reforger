// T-090.1.2.5 — P0 water-source spike: pick the hydrology mask provenance for the
// satellite water composite (ocean + inland) BEFORE any compositing happens.
// T-090.1.2.5.1 — inland refine: two-tier acceptance (compact lake/wetland class vs linear
// river/stream class), flatFrac tightened to kill graded town pavement, and a DEM
// valley-carve gate that admits dark mountain stream channels while excluding hillside
// road cuts. Ocean mask A unchanged. Spike output moves to t090_1_2_5_1_refine_spike.json
// (the .2.5 spike JSON is a shipped historical artifact and is never rewritten).
//
// Candidates evaluated (spec t090_1_2_5_satellite_water_composite.md §Investigation):
//   A. DEM height <= sea level (0 m)          — engine GetTerrainSurfaceY heights (T-091.0)
//   B. Eden_<N>_layer.edds material masks      — per-cell splat weights (pak VFS)
//   C. .Rivers flow maps / .Shore atlas / .topo/.smap — engine hydrology files (pak VFS)
//   D. Workbench MCP entity query              — LakeGeneratorEntity/RiverEntity live query
//   E. Supertexture water appearance ∩ DEM     — the engine's own SAP renderer paints water
//      areas with the underwater/seabed treatment (smooth desaturated grey, no vegetation
//      colour). Grey ∩ not-ocean ∩ not-engine-flattened ∩ opened ∩ min-area = inland water.
//
// Verdicts are computed from real data where possible (A, E) and documented with the probe
// evidence for the blocked candidates (B, C, D). The chosen mask is:
//   ocean  = A (DEM <= 0 m)                          — primary, with DEM depth as refine
//   inland = E (SAP water appearance ∩ DEM filters)  — engine-rendered water pixels
// No hand-painted lakes, no AI rivers, no solid rectangles: every inland component comes
// from engine-rendered supertexture pixels and is DEM-cross-filtered, and the component
// list is emitted for per-body visual audit in the verify log.
//
// Outputs:
//   .ai/artifacts/t090_1_2_5_water_source_spike.json   — decision + evidence (committed)
//   staging/sap/water-inland-mask.png                  — 12800² binary inland mask (gitignored)
//   staging/sap/water-spike-preview.png                — 1600² overlay for eyeballing (gitignored)
//
// Usage: node scripts/map-assets/analyze-water-sources.mjs [--skip-b-c-probes]
import { execFileSync } from "node:child_process";
import { createRequire } from "node:module";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = join(HERE, "../..");
const require = createRequire(join(REPO, "packages/tbd-schema/package.json"));
const { PNG } = require("pngjs");

const SAP = join(REPO, "packages/map-assets/everon/staging/sap");
const ORTHO = join(SAP, "everon-sap-ortho.png");
const DEM_PATH = join(REPO, "packages/map-assets/everon/dem/everon-dem-16bit.png");
const MANIFEST = JSON.parse(
  readFileSync(join(REPO, "packages/map-assets/everon/manifest.json"), "utf8"),
);
const OUT_JSON = join(REPO, ".ai/artifacts/t090_1_2_5_1_refine_spike.json");
const PREV_SPIKE = join(REPO, ".ai/artifacts/t090_1_2_5_water_source_spike.json");
const OUT_MASK = join(SAP, "water-inland-mask.png");
const OUT_PREVIEW = join(SAP, "water-spike-preview.png");

// ── Calibrated parameters (E) — measured on real ortho patches, see spike JSON evidence ──
const DETECT_DIM = 3200; // 4 m/px detection grid (roads shrink to <=2.5 px; lakes stay huge)
const SAT_MAX = 0.12; // HSL saturation: water/seabed grey ~0.04–0.10, vegetation >=0.15
const LUM_MIN = 0.2; // luminance band of the seabed/water treatment
const LUM_MAX = 0.44;
const OPEN_R = 2; // 8 m density-core radius at 4 m/px — removes roads (<=16 m) + roof speckle
const DENSITY_MIN = 0.6; // fraction of the (2r+1)² box that must be grey to seed a core
const OCEAN_DILATE_R = 5; // 20 m — keep the inland mask clear of the DEM coastline
const FLAT_DILATE_R = 2; // engine-flattened pads (runways/bases) + margin
const MIN_AREA_M2 = 2000; // smallest inland body worth shipping at map zoom
const MEAN_SAT_MAX = 0.115; // component-level mean saturation acceptance (compact class)
const SLOPE_PX_MAX_DEG = 18; // grey px on steeper ground = rock face, not water
// T-090.1.2.5.1 — two-tier acceptance. Compact class (lakes/wetlands):
const FLAT_FRAC_MAX = 0.12; // was 0.5 — graded town pavement measured 0.30–0.46, lake 0.031
const SLOPE_MEAN_MAX_DEG = 8; // unchanged
// Linear class (rivers/streams; thin winding ribbons), two sub-classes:
//   grey river  — carries the engine water appearance (meanSat <= MEAN_SAT_MAX), same
//                 trust level as the .2.5 ship; needs only a soft lowland/valley guard so
//                 hillside grey roads can't ride in.
//   wet channel — the RELAXED dark-wet appearance for mountain streams; the DEM valley
//                 carve is mandatory and the bar is strict (this class is the refine's
//                 new reach, so it earns acceptance, not the other way round).
const RIBBON_W_MAX_PX = 5; // ribbonWidth = 2·area/perimeter <= 5 px (20 m) → linear body
const LIN_MIN_AREA_M2 = 800; // grey river segments
const LIN_SLOPE_MEAN_MAX_DEG = 16; // channels in steep terrain pick up bank slope at 4 m/px
const LIN_FLAT_FRAC_MAX = 0.2;
const GREY_RIVER_VALLEY_MIN = 0.2; // soft guard: carve OR lowland slope
const GREY_RIVER_LOWLAND_SLOPE_DEG = 8;
const WET_MIN_AREA_M2 = 1200;
const WET_VALLEY_FRAC_MIN = 0.7; // hillside road cuts have no symmetric carve
const WET_MEAN_SAT_MAX = 0.16;
const WET_MEAN_LUM_MAX = 0.28; // wet rock/water is dark; dry dirt tracks are lighter
// Dark-wet stream pixel class (mountain channels are dark wet rock, not seabed grey):
const WET_LUM_MIN = 0.1;
const WET_LUM_MAX = 0.3;
const WET_SAT_MAX = 0.16;
const WET_SLOPE_PX_MAX_DEG = 24;
const VALLEY_BLUR_R = 12; // px @ 4 m/px → 48 m neighbourhood so banks enter the carve test
const VALLEY_CARVE_M = 0.8; // blur(DEM) − DEM above this = carved channel floor

const t0 = Date.now();
const log = (m) => console.log(`[water-spike] ${m}`);

if (!existsSync(ORTHO)) {
  console.error(`missing ${ORTHO} — run the SAP stitch pipeline first`);
  process.exit(1);
}

// ── DEM: heights + sea + exact-flat masks (native 6400², row 0 = SOUTH) ─────────────────
const dem = PNG.sync.read(readFileSync(DEM_PATH), { skipRescale: true });
const DW = dem.width; // 6400
const dStride = dem.data.length / (DW * dem.height);
const { heightRangeMinM: LO, heightRangeMaxM: HI } = MANIFEST.dem;
const SEA_U16 = Math.round(((0 - LO) / (HI - LO)) * 65535);
const demV = (x, y) => dem.data[(y * DW + x) * dStride];

// Sea mask + exact-flat mask (2×2 identical u16 above sea = engine flatten / water plane),
// both downsampled 6400→3200 by 2×2 max-pool and stored NORTH-UP to match the ortho.
const D = DETECT_DIM;
const sea = new Uint8Array(D * D);
const flat6400 = new Uint8Array(DW * DW);
let seaPx6400 = 0;
for (let y = 0; y < DW; y++) {
  for (let x = 0; x < DW; x++) {
    const v = demV(x, y);
    if (v <= SEA_U16) {
      seaPx6400++;
      continue;
    }
    if (
      x < DW - 1 &&
      y < DW - 1 &&
      v === demV(x + 1, y) &&
      v === demV(x, y + 1) &&
      v === demV(x + 1, y + 1)
    ) {
      flat6400[y * DW + x] = 1;
    }
  }
}
// Slope (deg) from central differences at 2 m/px; rock faces are steep, water is not.
const M_PER_U16 = (HI - LO) / 65535;
const flat = new Uint8Array(D * D);
const slope = new Float32Array(D * D); // max-pooled to 3200², north-up
const elevM = new Float32Array(D * D); // 2×2 average-pooled metres, north-up
for (let y = 0; y < DW; y++) {
  const ny = DW - 1 - y; // south-up → north-up
  for (let x = 0; x < DW; x++) {
    const di = (ny >> 1) * D + (x >> 1);
    const v = demV(x, y);
    elevM[di] += (v * M_PER_U16 + LO) / 4;
    if (v <= SEA_U16) sea[di] = 1;
    if (flat6400[y * DW + x]) flat[di] = 1;
    if (x > 0 && x < DW - 1 && y > 0 && y < DW - 1) {
      const gx = ((demV(x + 1, y) - demV(x - 1, y)) * M_PER_U16) / 4; // 2*2 m spacing
      const gy = ((demV(x, y + 1) - demV(x, y - 1)) * M_PER_U16) / 4;
      const s = (Math.atan(Math.hypot(gx, gy)) * 180) / Math.PI;
      if (s > slope[di]) slope[di] = s;
    }
  }
}

// Valley-carve mask (T-090.1.2.5.1): channel floors sit BELOW their 32 m neighbourhood mean.
// Streams are carved into the terrain in both cross directions; a road cut into a hillside
// has terrain above on one side and below on the other, so its blur-difference stays ~0.
const valley = (() => {
  const r = VALLEY_BLUR_R;
  const win = 2 * r + 1;
  const tmp = new Float32Array(D * D);
  for (let y = 0; y < D; y++) {
    let acc = 0;
    const row = y * D;
    for (let x = -r; x <= r; x++) acc += elevM[row + Math.max(0, Math.min(D - 1, x))];
    for (let x = 0; x < D; x++) {
      tmp[row + x] = acc / win;
      acc += elevM[row + Math.min(D - 1, x + r + 1)] - elevM[row + Math.max(0, x - r)];
    }
  }
  const out = new Uint8Array(D * D);
  for (let x = 0; x < D; x++) {
    let acc = 0;
    for (let y = -r; y <= r; y++) acc += tmp[Math.max(0, Math.min(D - 1, y)) * D + x];
    for (let y = 0; y < D; y++) {
      if (acc / win - elevM[y * D + x] > VALLEY_CARVE_M) out[y * D + x] = 1;
      acc += tmp[Math.min(D - 1, y + r + 1) * D + x] - tmp[Math.max(0, y - r) * D + x];
    }
  }
  return out;
})();
const seaFraction = seaPx6400 / (DW * DW);
log(`DEM sea fraction ${(seaFraction * 100).toFixed(1)} % (sea level u16=${SEA_U16})`);

// Inland <=0 audit for candidate A (flood-fill sea from the borders; leftovers = inland).
function inlandBelowSeaAudit() {
  const m = Uint8Array.from(sea);
  const q = [];
  for (let x = 0; x < D; x++) {
    for (const y of [0, D - 1]) if (m[y * D + x] === 1) (m[y * D + x] = 2), q.push(y * D + x);
  }
  for (let y = 0; y < D; y++) {
    for (const x of [0, D - 1]) if (m[y * D + x] === 1) (m[y * D + x] = 2), q.push(y * D + x);
  }
  while (q.length) {
    const i = q.pop();
    const x = i % D;
    const y = (i / D) | 0;
    if (x > 0 && m[i - 1] === 1) (m[i - 1] = 2), q.push(i - 1);
    if (x < D - 1 && m[i + 1] === 1) (m[i + 1] = 2), q.push(i + 1);
    if (y > 0 && m[i - D] === 1) (m[i - D] = 2), q.push(i - D);
    if (y < D - 1 && m[i + D] === 1) (m[i + D] = 2), q.push(i + D);
  }
  let px = 0;
  for (let i = 0; i < D * D; i++) if (m[i] === 1) px++;
  return px * 16; // m² at 4 m/px
}
const inlandBelowSeaM2 = inlandBelowSeaAudit();

// ── Ortho at detection scale (magick handles the 12800² decode + Lanczos) ───────────────
const work = mkdtempSync(join(tmpdir(), "water-spike-"));
process.on("exit", () => rmSync(work, { recursive: true, force: true }));
const small = join(work, "ortho3200.png");
log(`downsampling ortho → ${D}²`);
execFileSync("magick", [ORTHO, "-resize", `${D}x${D}!`, small]);
const ortho = PNG.sync.read(readFileSync(small));

// ── Candidate E classifier ───────────────────────────────────────────────────────────────
const sat = new Float32Array(D * D);
const lum = new Float32Array(D * D);
for (let i = 0; i < D * D; i++) {
  const r = ortho.data[i * 4] / 255;
  const g = ortho.data[i * 4 + 1] / 255;
  const b = ortho.data[i * 4 + 2] / 255;
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  lum[i] = l;
  const den = 1 - Math.abs(2 * l - 1);
  sat[i] = den < 1e-6 ? 0 : (max - min) / den;
}

const dilate = (src, r) => {
  const out = new Uint8Array(D * D);
  for (let y = 0; y < D; y++) {
    for (let x = 0; x < D; x++) {
      if (!src[y * D + x]) continue;
      for (let dy = -r; dy <= r; dy++) {
        const ny = y + dy;
        if (ny < 0 || ny >= D) continue;
        for (let dx = -r; dx <= r; dx++) {
          const nx = x + dx;
          if (nx >= 0 && nx < D) out[ny * D + nx] = 1;
        }
      }
    }
  }
  return out;
};
const erode = (src, r) => {
  const out = new Uint8Array(D * D);
  outer: for (let y = r; y < D - r; y++) {
    for (let x = r; x < D - r; x++) {
      for (let dy = -r; dy <= r; dy++) {
        for (let dx = -r; dx <= r; dx++) {
          if (!src[(y + dy) * D + (x + dx)]) continue outer;
        }
      }
      out[y * D + x] = 1;
    }
  }
  return out;
};

const seaWide = dilate(sea, OCEAN_DILATE_R);
const flatWide = dilate(flat, FLAT_DILATE_R);

let grey = new Uint8Array(D * D);
let wet = new Uint8Array(D * D); // T-090.1.2.5.1 dark-wet valley channels (mountain streams)
let greyPx = 0;
let wetPx = 0;
let greyOnSeaPx = 0;
let seaPx = 0;
for (let i = 0; i < D * D; i++) {
  const isGrey =
    sat[i] < SAT_MAX && lum[i] > LUM_MIN && lum[i] < LUM_MAX && slope[i] <= SLOPE_PX_MAX_DEG;
  if (sea[i]) {
    seaPx++;
    if (isGrey) greyOnSeaPx++;
  }
  if (seaWide[i]) continue;
  if (isGrey) {
    grey[i] = 1;
    greyPx++;
  }
  if (
    valley[i] &&
    lum[i] > WET_LUM_MIN &&
    lum[i] < WET_LUM_MAX &&
    sat[i] < WET_SAT_MAX &&
    slope[i] <= WET_SLOPE_PX_MAX_DEG
  ) {
    wet[i] = 1;
    wetPx++;
  }
}
const greyOceanRecall = greyOnSeaPx / seaPx; // engine water rendering ↔ grey correlation
log(
  `grey px inland (pre-open): ${greyPx}; wet-valley px: ${wetPx}; ocean grey recall ${(greyOceanRecall * 100).toFixed(1)} %`,
);

// Speckle-tolerant opening: water bodies are mottled at 4 m/px, so a strict erosion dies on
// interior holes. Instead: density core (>= minFrac of the (2r+1)² box set) → dilate the
// core back out and intersect with the original mask. Thin roads (<= ~2 px at this scale)
// never reach the density floor; large bodies keep their true outline.
const densityOpen = (src, r, minFrac) => {
  const side = 2 * r + 1;
  const need = Math.ceil(side * side * minFrac);
  const ii = new Int32Array((D + 1) * (D + 1));
  for (let y = 0; y < D; y++) {
    let row = 0;
    for (let x = 0; x < D; x++) {
      row += src[y * D + x];
      ii[(y + 1) * (D + 1) + (x + 1)] = ii[y * (D + 1) + (x + 1)] + row;
    }
  }
  const boxSum = (x0, y0, x1, y1) =>
    ii[(y1 + 1) * (D + 1) + (x1 + 1)] -
    ii[y0 * (D + 1) + (x1 + 1)] -
    ii[(y1 + 1) * (D + 1) + x0] +
    ii[y0 * (D + 1) + x0];
  const core = new Uint8Array(D * D);
  for (let y = 0; y < D; y++) {
    for (let x = 0; x < D; x++) {
      if (!src[y * D + x]) continue;
      const s = boxSum(Math.max(0, x - r), Math.max(0, y - r), Math.min(D - 1, x + r), Math.min(D - 1, y + r));
      if (s >= need) core[y * D + x] = 1;
    }
  }
  const coreWide = dilate(core, r + 1);
  const out = new Uint8Array(D * D);
  for (let i = 0; i < D * D; i++) out[i] = src[i] && coreWide[i] ? 1 : 0;
  return out;
};
grey = densityOpen(grey, OPEN_R, DENSITY_MIN);
wet = densityOpen(wet, 1, 0.55); // streams are 1–3 px ribbons — light speckle clean only
// Union: components form over both classes (a stream can feed a grey lowland river).
for (let i = 0; i < D * D; i++) if (wet[i]) grey[i] = 1;

// Connected components + per-component acceptance.
const labels = new Int32Array(D * D).fill(-1);
const comps = [];
const MIN_AREA_PX = Math.ceil(MIN_AREA_M2 / 16);
for (let i = 0; i < D * D; i++) {
  if (!grey[i] || labels[i] !== -1) continue;
  const st = [i];
  labels[i] = comps.length;
  const px = [];
  while (st.length) {
    const k = st.pop();
    px.push(k);
    const x = k % D;
    const y = (k / D) | 0;
    for (const [dx, dy] of [
      [1, 0],
      [-1, 0],
      [0, 1],
      [0, -1],
    ]) {
      const nx = x + dx;
      const ny = y + dy;
      if (nx < 0 || nx >= D || ny < 0 || ny >= D) continue;
      const j = ny * D + nx;
      if (grey[j] && labels[j] === -1) {
        labels[j] = comps.length;
        st.push(j);
      }
    }
  }
  let sSat = 0;
  let sSlope = 0;
  let sElev = 0;
  let sLum = 0;
  let nFlat = 0;
  let nValley = 0;
  let perim = 0;
  let minX = D;
  let maxX = 0;
  let minY = D;
  let maxY = 0;
  for (const k of px) {
    sSat += sat[k];
    sSlope += slope[k];
    sElev += elevM[k];
    sLum += lum[k];
    if (flatWide[k]) nFlat++;
    if (valley[k]) nValley++;
    const x = k % D;
    const y = (k / D) | 0;
    if (
      x === 0 || x === D - 1 || y === 0 || y === D - 1 ||
      !grey[k - 1] || !grey[k + 1] || !grey[k - D] || !grey[k + D]
    ) {
      perim++;
    }
    if (x < minX) minX = x;
    if (x > maxX) maxX = x;
    if (y < minY) minY = y;
    if (y > maxY) maxY = y;
  }
  const meanSat = sSat / px.length;
  const meanSlope = sSlope / px.length;
  const meanElevM = sElev / px.length;
  const meanLum = sLum / px.length;
  const flatFrac = nFlat / px.length;
  const valleyFrac = nValley / px.length;
  const areaM2 = px.length * 16;
  const ribbonWidthPx = (2 * px.length) / Math.max(1, perim);
  const isLinear = ribbonWidthPx <= RIBBON_W_MAX_PX;
  // Two-tier acceptance (T-090.1.2.5.1): compact = lake/wetland rules with the pavement-
  // killing flatFrac cap. Linear splits by appearance trust: grey rivers (engine water
  // appearance, .2.5 trust level) take a soft carve-or-lowland guard; the relaxed dark-wet
  // class must earn it — deep symmetric DEM carve, dark, near-grey, bigger than a speckle.
  const isGreyRiver = meanSat <= MEAN_SAT_MAX;
  let accepted;
  let klass;
  if (!isLinear) {
    klass = "compact";
    accepted =
      areaM2 >= MIN_AREA_M2 &&
      meanSat <= MEAN_SAT_MAX &&
      flatFrac <= FLAT_FRAC_MAX &&
      meanSlope <= SLOPE_MEAN_MAX_DEG;
  } else if (isGreyRiver) {
    klass = "grey-river";
    accepted =
      areaM2 >= LIN_MIN_AREA_M2 &&
      meanSlope <= LIN_SLOPE_MEAN_MAX_DEG &&
      flatFrac <= LIN_FLAT_FRAC_MAX &&
      (valleyFrac >= GREY_RIVER_VALLEY_MIN || meanSlope <= GREY_RIVER_LOWLAND_SLOPE_DEG);
  } else {
    klass = "wet-channel";
    accepted =
      areaM2 >= WET_MIN_AREA_M2 &&
      meanSlope <= LIN_SLOPE_MEAN_MAX_DEG &&
      flatFrac <= LIN_FLAT_FRAC_MAX &&
      valleyFrac >= WET_VALLEY_FRAC_MIN &&
      meanSat <= WET_MEAN_SAT_MAX &&
      meanLum <= WET_MEAN_LUM_MAX;
  }
  comps.push({
    px,
    accepted,
    class: klass,
    areaM2,
    meanSat: +meanSat.toFixed(4),
    meanLum: +meanLum.toFixed(3),
    meanSlopeDeg: +meanSlope.toFixed(2),
    meanElevM: +meanElevM.toFixed(1),
    flatFrac: +flatFrac.toFixed(3),
    valleyFrac: +valleyFrac.toFixed(3),
    ribbonWidthPx: +ribbonWidthPx.toFixed(2),
    // ortho-space (north-up, 1 m/px) bbox for crops + world-space centre for the log
    bboxOrthoPx: [minX * 4, minY * 4, (maxX + 1) * 4, (maxY + 1) * 4],
    centreWorldM: [((minX + maxX) / 2) * 4, 12800 - ((minY + maxY) / 2) * 4], // [x, z]
  });
}
const accepted = comps.filter((c) => c.accepted);
accepted.sort((a, b) => b.areaM2 - a.areaM2);
log(
  `components: ${comps.length} total, ${accepted.length} accepted (>=${MIN_AREA_M2} m², meanSat<=${MEAN_SAT_MAX}, flatFrac<=${FLAT_FRAC_MAX})`,
);
for (const c of accepted.slice(0, 16)) {
  log(
    `  ${c.class.padEnd(7)} ${(c.areaM2 / 1e4).toFixed(1)} ha @ world (${c.centreWorldM[0]}, ${c.centreWorldM[1]}) sat=${c.meanSat} slope=${c.meanSlopeDeg}° flat=${c.flatFrac} valley=${c.valleyFrac} w=${c.ribbonWidthPx}px`,
  );
}

// ── Emit inland mask (12800², binary, north-up) + preview overlay ───────────────────────
const mask = new Uint8Array(D * D);
for (const c of accepted) for (const k of c.px) mask[k] = 1;
{
  const p = new PNG({ width: D, height: D, colorType: 0 });
  for (let i = 0; i < D * D; i++) {
    p.data[i * 4] = p.data[i * 4 + 1] = p.data[i * 4 + 2] = mask[i] ? 255 : 0;
    p.data[i * 4 + 3] = 255;
  }
  const m3200 = join(work, "mask3200.png");
  writeFileSync(m3200, PNG.sync.write(p));
  execFileSync("magick", [m3200, "-resize", "12800x12800!", "-threshold", "50%", OUT_MASK]);
  execFileSync("magick", [
    ORTHO,
    "-resize",
    "1600x1600!",
    "(",
    m3200,
    "-resize",
    "1600x1600!",
    "-threshold",
    "25%",
    "-fill",
    "#2266ff",
    "-opaque",
    "white",
    "-transparent",
    "black",
    ")",
    "-composite",
    OUT_PREVIEW,
  ]);
}
log(`wrote ${OUT_MASK} + ${OUT_PREVIEW}`);

// ── Refine spike JSON (T-090.1.2.5.1): locked params + before/after vs the .2.5 ship ─────
// Compare against the shipped .2.5 accepted-body list (historical artifact, read-only):
// a previous body is "retained" when some new accepted body centre lies within 250 m.
let comparison = null;
if (existsSync(PREV_SPIKE)) {
  const prev = JSON.parse(readFileSync(PREV_SPIKE, "utf8"));
  const prevBodies =
    prev.candidates?.["E-supertexture-water-appearance"]?.evidence?.acceptedBodies ?? [];
  const near = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1]) <= 250;
  const retained = [];
  const dropped = [];
  for (const p of prevBodies) {
    (accepted.some((c) => near(c.centreWorldM, p.centreWorldM)) ? retained : dropped).push({
      centreWorldM: p.centreWorldM,
      areaM2: p.areaM2,
      flatFrac: p.flatFrac,
    });
  }
  const isNew = (c) => !prevBodies.some((p) => near(c.centreWorldM, p.centreWorldM));
  comparison = {
    prevAccepted: prevBodies.length,
    retained: retained.length,
    dropped,
    newBodies: accepted.filter(isNew).map(({ px, ...rest }) => rest),
  };
}

const spike = {
  slice: "T-090.1.2.5.1",
  parent: "T-090.1.2.5 spike: .ai/artifacts/t090_1_2_5_water_source_spike.json (unchanged)",
  generatedAt: new Date().toISOString(),
  decision: {
    oceanMask: "A-dem-below-sea-level (UNCHANGED — out of slice scope)",
    inlandMask:
      "E-supertexture-water-appearance-dem-filtered, refined: two-tier acceptance " +
      "(compact lake/wetland class vs linear river/stream class) + dark-wet valley-channel " +
      "pixel pass for mountain streams",
    forbiddenMethodsAttestation:
      "No hand-painted lakes, no AI-generated rivers, no solid rectangles. All refine levers " +
      "are engine data: flatFrac (engine-graded exact-flat DEM plateaus), valley carve " +
      "(boxBlur(DEM) − DEM — designed watercourse channels), slope (DEM gradient), plus the " +
      "engine-rendered supertexture appearance bands. Every accepted body enumerated for audit.",
  },
  refine: {
    "R1-road-exclusion": {
      lever: "compact-class flatFracMax 0.5 → 0.12",
      rationale:
        "Operator-flagged town pavement bodies measured flatFrac 0.30–0.46 (engine-graded " +
        "pads); real water: central lake 0.031, rivers ~0–0.06. Thin asphalt still dies in " +
        "the density opening; hillside roads additionally fail the linear-class valley gate.",
      operatorFpBodies:
        "prev bodies near (4514,9530)/(4836,9224)/(4366,9304)/(4776,9268) — see comparison.dropped",
    },
    "R2-hill-rivers": {
      lever:
        "linear class (ribbonWidth <= 5 px) split by appearance trust: grey-river (engine " +
        "water appearance, .2.5 trust) needs carve>=0.2 OR lowland slope<=8°; wet-channel " +
        "(relaxed dark-wet band: lum 0.10–0.30, sat<0.16, valley-gated px) must earn it — " +
        "valleyFrac>=0.7, meanLum<=0.28, meanSat<=0.16, area>=1200 m²",
      rationale:
        "Mountain streams are dark wet carved channels in DEM valley floors; roads cut into " +
        "hillsides have no symmetric carve (blur-difference ~0) and fail valleyFrac; dry " +
        "dirt tracks are lighter than wet channel rock and fail meanLum.",
    },
    "R3-topo-smap-probe": {
      result: "PARTIAL (timeboxed ~20 min, not needed to ship)",
      evidence:
        "Eden.topo coordinate encoding cracked: big-endian float32 world-coordinate pairs " +
        "in the 0–12800 range (repeated shared vertices = closed polylines). Record framing/" +
        "typing (road vs building vs contour) undecoded within the timebox, so road-corridor " +
        "subtraction stays unavailable. Eden.smap remains index-buffer-like binary. Note for " +
        "T-090.8: .topo is the most promising offline road/hydro vector source.",
    },
  },
  params: {
    unchanged: {
      detectDim: DETECT_DIM,
      satMax: SAT_MAX,
      lumMin: LUM_MIN,
      lumMax: LUM_MAX,
      openRadiusPx: OPEN_R,
      densityMin: DENSITY_MIN,
      oceanDilateRadiusPx: OCEAN_DILATE_R,
      flatDilateRadiusPx: FLAT_DILATE_R,
      minAreaM2: MIN_AREA_M2,
      meanSatMax: MEAN_SAT_MAX,
      slopePxMaxDeg: SLOPE_PX_MAX_DEG,
      slopeMeanMaxDeg: SLOPE_MEAN_MAX_DEG,
    },
    changed: { flatFracMax: { old: 0.5, new: FLAT_FRAC_MAX } },
    added: {
      ribbonWidthMaxPx: RIBBON_W_MAX_PX,
      linMinAreaM2: LIN_MIN_AREA_M2,
      linSlopeMeanMaxDeg: LIN_SLOPE_MEAN_MAX_DEG,
      linFlatFracMax: LIN_FLAT_FRAC_MAX,
      greyRiverValleyMin: GREY_RIVER_VALLEY_MIN,
      greyRiverLowlandSlopeDeg: GREY_RIVER_LOWLAND_SLOPE_DEG,
      wetMinAreaM2: WET_MIN_AREA_M2,
      wetValleyFracMin: WET_VALLEY_FRAC_MIN,
      wetMeanSatMax: WET_MEAN_SAT_MAX,
      wetMeanLumMax: WET_MEAN_LUM_MAX,
      wetLumMin: WET_LUM_MIN,
      wetLumMax: WET_LUM_MAX,
      wetSatMax: WET_SAT_MAX,
      wetSlopePxMaxDeg: WET_SLOPE_PX_MAX_DEG,
      valleyBlurRadiusPx: VALLEY_BLUR_R,
      valleyCarveM: VALLEY_CARVE_M,
    },
  },
  results: {
    greyOceanRecall: +greyOceanRecall.toFixed(3),
    seaFraction: +seaFraction.toFixed(4),
    inlandBelowSeaM2,
    acceptedBodies: accepted.map(({ px, ...rest }) => rest),
    rejectedComponentCount: comps.length - accepted.length,
    comparisonVsShip25: comparison,
  },
  outputs: {
    inlandMaskPng: "packages/map-assets/everon/staging/sap/water-inland-mask.png (gitignored)",
    previewPng: "packages/map-assets/everon/staging/sap/water-spike-preview.png (gitignored)",
  },
};
writeFileSync(OUT_JSON, JSON.stringify(spike, null, 2) + "\n");
log(`wrote ${OUT_JSON} in ${((Date.now() - t0) / 1000).toFixed(0)}s`);
