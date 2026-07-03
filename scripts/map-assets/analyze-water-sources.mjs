// T-090.1.2.5 — P0 water-source spike: pick the hydrology mask provenance for the
// satellite water composite (ocean + inland) BEFORE any compositing happens.
// T-090.1.2.5.1 — inland refine: two-tier acceptance (compact lake/wetland class vs linear
// river/stream class), flatFrac tightened to kill graded town pavement, and a DEM
// valley-carve gate that admits dark mountain stream channels while excluding hillside
// road cuts. Ocean mask A unchanged.
// T-090.1.2.5.2 — EXACT ROAD-GEOMETRY subtraction: the .topo decode (decode-topo.mjs)
// turned out to carry the full ROAD network (4 classes + airfield lines + powerlines) and
// NO hydro layer (G1-B, verified by colour-overlay: type-1 routes cross ridges and connect
// the airfield/towns — highways, not rivers). The engine road vectors are rasterized into
// exclusion corridors that deterministically remove the residual path/ditch FP class, which
// in turn makes it safe to RELAX the wet-channel stream class (the operator wants carved
// gully watercourses blue even when dry) — closing the hill-stream FN gap. Fully offline
// (pak + DEM) → automatable. Spike output: t090_1_2_5_2_source_spike.json.
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
const { decodeTopo, TOPO_TYPES } = await import(join(HERE, "decode-topo.mjs"));

const SAP = join(REPO, "packages/map-assets/everon/staging/sap");
const ORTHO = join(SAP, "everon-sap-ortho.png");
const DEM_PATH = join(REPO, "packages/map-assets/everon/dem/everon-dem-16bit.png");
const MANIFEST = JSON.parse(
  readFileSync(join(REPO, "packages/map-assets/everon/manifest.json"), "utf8"),
);
const OUT_JSON = join(REPO, ".ai/artifacts/t090_1_2_5_2_source_spike.json");
const PREV_SPIKE = join(REPO, ".ai/artifacts/t090_1_2_5_1_refine_spike.json");
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
// T-090.1.2.5.1 — compact class (lakes/wetlands):
const FLAT_FRAC_MAX = 0.12; // was 0.5 — graded town pavement measured 0.30–0.46, lake 0.031
const SLOPE_MEAN_MAX_DEG = 8; // unchanged
// T-090.1.2.5.2 — exact ROAD corridors from the .topo vector network (metres → px at the
// 4 m/px detect grid), used as an EXCLUSION mask. Generous half-widths: over-subtracting a
// few metres of true water beside a bridge is invisible; under-subtracting recreates the FP.
const ROAD_HALF_W = { 0: 3, 1: 2, 2: 2, 3: 1, 5: 1 }; // per .topo type, px @ 4 m/px
const ROAD_SAMPLE_STEP_PX = 2; // validation sampling stride along vector segments
// Component guard (NOT pixel exclusion — rivers legitimately run beside roads and pixel
// exclusion fragmented them): a path body sits ON its road line so nearly all its px fall
// inside the corridor (frac → ~1); a river beside a road only grazes it (frac ≲ 0.3).
const ROAD_OVERLAP_MAX = 0.45;
// Linear classes (.2.5.1) reinstated ON TOP of the road subtraction — with the road
// network excluded exactly, the lowland grey-river guard loosens back and the wet-channel
// stream class relaxes (operator call: carved gully watercourses read as water even when
// seasonally dry — brownish + lighter than the .2.5.1 wet band allowed).
const RIBBON_W_MAX_PX = 5; // ribbonWidth = 2·area/perimeter <= 5 px (20 m) → linear body
const LIN_MIN_AREA_M2 = 800; // grey river segments
const LIN_SLOPE_MEAN_MAX_DEG = 16; // channels in steep terrain pick up bank slope at 4 m/px
const LIN_FLAT_FRAC_MAX = 0.2;
const GREY_RIVER_VALLEY_MIN = 0.2; // soft guard: carve OR lowland slope
const GREY_RIVER_LOWLAND_SLOPE_DEG = 8;
const WET_MIN_AREA_M2 = 1000;
const WET_VALLEY_FRAC_MIN = 0.6; // was 0.7 — moderate relax; road guard covers the rest
const WET_MEAN_SAT_MAX = 0.18; // was 0.16 — some dry-gully brown allowed
const WET_MEAN_LUM_MAX = 0.31; // was 0.28
// Dark-wet/gully stream pixel class:
const WET_LUM_MIN = 0.09;
const WET_LUM_MAX = 0.33;
const WET_SAT_MAX = 0.19;
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

// ── T-090.1.2.5.2: exact ROAD corridors from the .topo vector network ────────────────────
// .topo verts are (x east, y north-up image) in metres — same frame as this grid at /4.
// Vertices encode a SEGMENT LIST (consecutive pairs share endpoints), so draw pairwise.
// ALL five .topo classes are road/airfield line work (colour-overlay verified: type-1
// routes cross ridges and connect airfield/towns; PWLN trailing group = 2-vertex powerline
// spans) → the whole set becomes the exclusion corridor.
const topo = await decodeTopo("everon");
const roadCorridor = new Uint8Array(D * D);
const stampDisc = (mask2, cx, cy, r) => {
  const x0 = Math.max(0, Math.round(cx) - r);
  const x1 = Math.min(D - 1, Math.round(cx) + r);
  const y0 = Math.max(0, Math.round(cy) - r);
  const y1 = Math.min(D - 1, Math.round(cy) + r);
  for (let y = y0; y <= y1; y++) {
    for (let x = x0; x <= x1; x++) {
      if ((x - cx) * (x - cx) + (y - cy) * (y - cy) <= r * r + 0.5) mask2[y * D + x] = 1;
    }
  }
};
const drawRecord = (mask2, rec, halfW) => {
  const v = rec.verts;
  for (let s = 0; s + 3 < v.length; s += 4) {
    const ax = v[s] / 4;
    const ay = v[s + 1] / 4;
    const bx = v[s + 2] / 4;
    const by = v[s + 3] / 4;
    const steps = Math.max(1, Math.ceil(Math.hypot(bx - ax, by - ay)));
    for (let t = 0; t <= steps; t++) {
      stampDisc(mask2, ax + ((bx - ax) * t) / steps, ay + ((by - ay) * t) / steps, halfW);
    }
  }
};
const sampleFrac = (records, pred) => {
  let hit = 0;
  let n = 0;
  for (const rec of records) {
    const v = rec.verts;
    for (let s = 0; s + 3 < v.length; s += 4) {
      const ax = v[s] / 4;
      const ay = v[s + 1] / 4;
      const bx = v[s + 2] / 4;
      const by = v[s + 3] / 4;
      const steps = Math.max(1, Math.ceil(Math.hypot(bx - ax, by - ay) / ROAD_SAMPLE_STEP_PX));
      for (let t = 0; t <= steps; t++) {
        const x = Math.round(ax + ((bx - ax) * t) / steps);
        const y = Math.round(ay + ((by - ay) * t) / steps);
        if (x < 0 || x >= D || y < 0 || y >= D) continue;
        n++;
        if (pred(y * D + x)) hit++;
      }
    }
  }
  return n ? hit / n : 0;
};
let roadRecordCount = 0;
for (const rec of topo.records) {
  const halfW = ROAD_HALF_W[rec.type];
  if (halfW === undefined) continue;
  drawRecord(roadCorridor, rec, halfW);
  roadRecordCount++;
}
const airfieldRecs = topo.records.filter((r) => r.type === TOPO_TYPES.AIRFIELD);
const topoValidation = {
  airfieldOnEngineFlatFrac: +sampleFrac(airfieldRecs, (i) => flatWide[i] === 1).toFixed(3),
  // The FP body class must sit on the corridor: measured on the operator sites below.
  note:
    "type-1 colour-overlay crosses ridges + connects airfield/towns → highways (no hydro " +
    "layer in .topo); all 5 classes rasterized as exclusion corridors",
};
let roadPx = 0;
for (let i = 0; i < D * D; i++) if (roadCorridor[i]) roadPx++;
log(`topo road corridors: ${roadRecordCount} records → ${roadPx} px (${topoValidation.airfieldOnEngineFlatFrac} airfield-flat check)`);

// ── Pixel classes (road-corridor px excluded up front) ──────────────────────────────────
let grey = new Uint8Array(D * D);
let wet = new Uint8Array(D * D); // dark/gully valley channels (mountain + seasonal streams)
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
  let nRoad = 0;
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
    if (roadCorridor[k]) nRoad++;
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
  const roadFrac = nRoad / px.length; // path bodies ride their road line → frac ~1
  // T-090.1.2.5.2 acceptance: same two-tier shape as .2.5.1 but on the road-subtracted
  // pixel field — the .topo corridors already removed every path/ditch, so the wet-channel
  // stream class runs at the relaxed thresholds (carved gully watercourses count as water).
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
  if (accepted && roadFrac > ROAD_OVERLAP_MAX) accepted = false; // the .topo road guard
  comps.push({
    px,
    accepted,
    class: klass,
    roadFrac: +roadFrac.toFixed(3),
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
// Debug: WATER_DEBUG_COORDS="x,z;x,z" logs every component near the given world coords.
if (process.env.WATER_DEBUG_COORDS) {
  for (const pair of process.env.WATER_DEBUG_COORDS.split(";")) {
    const [qx, qz] = pair.split(",").map(Number);
    log(`debug components near (${qx}, ${qz}):`);
    for (const c of comps) {
      if (Math.hypot(c.centreWorldM[0] - qx, c.centreWorldM[1] - qz) > 300) continue;
      const { px, ...rest } = c;
      log(`  ${JSON.stringify(rest)}`);
    }
  }
}
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

// ── Source spike JSON (T-090.1.2.5.2) — vs the .2.5.1 ship ──────────────────────────────
// A .2.5.1 body is "retained" when a new accepted body centre lies within 250 m; the
// dropped list is expected to be exactly the road-riding FP bodies.
let comparison = null;
if (existsSync(PREV_SPIKE)) {
  const prev = JSON.parse(readFileSync(PREV_SPIKE, "utf8"));
  const prevBodies = prev.results?.acceptedBodies ?? [];
  const near = (a, b) => Math.hypot(a[0] - b[0], a[1] - b[1]) <= 250;
  const retained = [];
  const dropped = [];
  for (const p of prevBodies) {
    (accepted.some((c) => near(c.centreWorldM, p.centreWorldM)) ? retained : dropped).push({
      centreWorldM: p.centreWorldM,
      areaM2: p.areaM2,
      class: p.class,
    });
  }
  const isNew = (c) => !prevBodies.some((p) => near(c.centreWorldM, p.centreWorldM));
  comparison = {
    prevAccepted: prevBodies.length,
    retained: retained.length,
    dropped,
    newBodies: accepted.filter(isNew).length,
  };
}

const spike = {
  slice: "T-090.1.2.5.2",
  parent:
    "T-090.1.2.5 + .2.5.1 spikes: .ai/artifacts/t090_1_2_5_water_source_spike.json / " +
    "t090_1_2_5_1_refine_spike.json (shipped history, unchanged)",
  generatedAt: new Date().toISOString(),
  decision: {
    verdict:
      "G1-B — Eden.topo carries the full ROAD network but NO hydro layer; exact road-" +
      "corridor SUBTRACTION removes the path/ditch FP class deterministically, enabling a " +
      "safe wet-channel relaxation that closes the hill-stream/gully FN gap",
    oceanMask: "A-dem-below-sea-level (UNCHANGED)",
    inlandMask:
      "appearance classes (compact + grey-river + wet-channel) computed on the ROAD-" +
      "SUBTRACTED pixel field; wet-channel relaxed (operator call: carved gully " +
      "watercourses read as water even when seasonally dry)",
    automation:
      "fully offline: pak (.topo + supertextures) + committed DEM → make map-water-everon; " +
      "terrain-parameterized via decode-topo.mjs TOPO_TERRAINS (operator one-button requirement)",
    forbiddenMethodsAttestation:
      "No hand-painted lakes, no AI-generated rivers, no solid rectangles. The subtraction " +
      "layer is the engine's own map-geometry road network decoded from Eden.topo; water " +
      "acceptance remains engine-rendered supertexture appearance + engine DEM filters.",
  },
  topoFormat: {
    file: "worlds/Eden/Eden.topo (pak VFS)",
    framing:
      "header 0x18 B (u32 @0x10 = sectionCount 6, @0x14 = recordsPerSection 888); " +
      "record = [u8 type][u32 vertexCount][vertexCount × (f32LE x, f32LE y)][u32 K][K × u32 " +
      "attrs]; sections 2..6 prefixed by u32 recordCount (LOD levels of the same set); " +
      "y axis = NORTH-UP image metres (worldZ = 12800 − y)",
    types: {
      "0": "airfield/runway line work ×5 — sits exactly on the NW-airfield engine-flattened runways (0.833 flat-overlap; also proves the y-axis orientation)",
      "1": "primary routes/highways ×12 — colour-overlay crosses ridges and connects airfield/towns (NOT rivers — earlier bbox-chain reading corrected)",
      "2": "secondary roads ×110",
      "3": "minor roads/tracks ×367",
      "5": "paths/trails ×394",
    },
    trailingGroups:
      "'PWLN' tagged group after the 6 sections = 2-vertex span records (powerlines); no " +
      "hydro anywhere in .topo — exact water GEOMETRY needs Eden.ent entities (locked pak " +
      "codec) or Workbench entity export → T-090.8",
  },
  params: {
    roadSubtraction: { halfWidthPxByType: ROAD_HALF_W, gridMetersPerPx: 4, roadOverlapMax: ROAD_OVERLAP_MAX },
    compactClass: {
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
      flatFracMax: FLAT_FRAC_MAX,
      slopePxMaxDeg: SLOPE_PX_MAX_DEG,
      slopeMeanMaxDeg: SLOPE_MEAN_MAX_DEG,
      ribbonWidthMaxPx: RIBBON_W_MAX_PX,
    },
    greyRiver: {
      linMinAreaM2: LIN_MIN_AREA_M2,
      linSlopeMeanMaxDeg: LIN_SLOPE_MEAN_MAX_DEG,
      linFlatFracMax: LIN_FLAT_FRAC_MAX,
      greyRiverValleyMin: GREY_RIVER_VALLEY_MIN,
      greyRiverLowlandSlopeDeg: GREY_RIVER_LOWLAND_SLOPE_DEG,
    },
    wetChannelRelaxed: {
      wetMinAreaM2: { old251: 1200, new: WET_MIN_AREA_M2 },
      wetValleyFracMin: { old251: 0.7, new: WET_VALLEY_FRAC_MIN },
      wetMeanSatMax: { old251: 0.16, new: WET_MEAN_SAT_MAX },
      wetMeanLumMax: { old251: 0.28, new: WET_MEAN_LUM_MAX },
      wetPxBand: { lum: [WET_LUM_MIN, WET_LUM_MAX], satMax: WET_SAT_MAX, slopeMaxDeg: WET_SLOPE_PX_MAX_DEG },
      valleyBlurRadiusPx: VALLEY_BLUR_R,
      valleyCarveM: VALLEY_CARVE_M,
    },
  },
  results: {
    topoValidation,
    roadCorridorPx: roadPx,
    greyOceanRecall: +greyOceanRecall.toFixed(3),
    seaFraction: +seaFraction.toFixed(4),
    inlandBelowSeaM2,
    acceptedBodies: accepted.map(({ px, ...rest }) => rest),
    rejectedComponentCount: comps.length - accepted.length,
    comparisonVsShip251: comparison,
  },
  outputs: {
    inlandMaskPng: "packages/map-assets/everon/staging/sap/water-inland-mask.png (gitignored)",
    previewPng: "packages/map-assets/everon/staging/sap/water-spike-preview.png (gitignored)",
  },
};
writeFileSync(OUT_JSON, JSON.stringify(spike, null, 2) + "\n");
log(`wrote ${OUT_JSON} in ${((Date.now() - t0) / 1000).toFixed(0)}s`);
