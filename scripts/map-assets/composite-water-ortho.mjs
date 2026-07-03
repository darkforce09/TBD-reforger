// T-090.1.2.5 — Composite engine-sourced hydrology onto the seam-fixed SAP ortho.
//
// Inputs (all north-up, [0,0,12800,12800] world contract):
//   staging/sap/everon-sap-ortho.png     — seam-fixed SAP ortho (T-090.1.2.2), 12800² @ 1 m/px
//   dem/everon-dem-16bit.png             — 6400² u16 GetTerrainSurfaceY heights (row 0 = SOUTH)
//   staging/sap/water-inland-mask.png    — 12800² binary inland-water mask from the P0 spike
//                                          (analyze-water-sources.mjs; engine supertexture water
//                                          appearance ∩ DEM filters — see the spike JSON)
//
// Treatment (locked in the slice plan):
//   ocean  = DEM <= 0 m; colour ramps oceanDark→oceanBright by depth (engine palette from
//            TBD_SatelliteExportPlugin.c SetupColors), alpha WATER_ALPHA over the SAP seabed
//            so the real ground texture still ghosts through (no solid rectangle)
//   inland = flat oceanBright-leaning water colour at the same alpha over the river/lake pixels
//   feather = alpha fades in over FEATHER_R px INSIDE the water mask only — every land pixel
//             outside the mask stays BYTE-IDENTICAL (feather never bleeds outward)
//
// In-place pipeline stage (same pattern as the T-090.1.2.2 seam repair): the canonical ortho is
// backed up to everon-sap-ortho.pre-water.png once, overwritten with the composite, and a
// `waterComposite` block is appended to TBD_SatExport_meta.json.
//
// Usage: node scripts/map-assets/composite-water-ortho.mjs
import { createRequire } from "node:module";
import { copyFileSync, existsSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = join(HERE, "../..");
const require = createRequire(join(REPO, "packages/tbd-schema/package.json"));
const { PNG } = require("pngjs");

const SAP = join(REPO, "packages/map-assets/everon/staging/sap");
const ORTHO = join(SAP, "everon-sap-ortho.png");
const BACKUP = join(SAP, "everon-sap-ortho.pre-water.png");
const META = join(SAP, "TBD_SatExport_meta.json");
const INLAND = join(SAP, "water-inland-mask.png");
const DEM_PATH = join(REPO, "packages/map-assets/everon/dem/everon-dem-16bit.png");
const MANIFEST = JSON.parse(
  readFileSync(join(REPO, "packages/map-assets/everon/manifest.json"), "utf8"),
);

// Engine-faithful water palette — TBD_SatelliteExportPlugin.c SetupColors (T-090.1 interim).
const OCEAN_BRIGHT = [58, 96, 120];
const OCEAN_DARK = [28, 52, 78];
// Inland reads best a touch brighter than the deep-ocean ramp end.
const INLAND_COLOR = [52, 88, 112];
const WATER_ALPHA = 0.8; // plan-locked 0.75–0.85
const DEPTH_FULL_M = 80; // depth (m) at which the ocean ramp reaches oceanDark
const FEATHER_R = 3; // px of inward alpha fade at every water edge

const t0 = Date.now();
const log = (m) => console.log(`[water-composite] ${m}`);

for (const p of [ORTHO, INLAND, DEM_PATH]) {
  if (!existsSync(p)) {
    console.error(`missing ${p}${p === INLAND ? " — run analyze-water-sources.mjs first" : ""}`);
    process.exit(1);
  }
}

// Refuse to double-composite: the meta block records the one allowed application.
const meta = JSON.parse(readFileSync(META, "utf8"));
if (meta.waterComposite) {
  console.error(
    "meta already has waterComposite — restore everon-sap-ortho.pre-water.png (and remove the meta block) before re-running",
  );
  process.exit(1);
}

log(`reading ortho (12800², this takes a moment)`);
const ortho = PNG.sync.read(readFileSync(ORTHO));
const W = ortho.width;
if (W !== 12800 || ortho.height !== 12800) {
  console.error(`ortho is ${W}x${ortho.height}, expected 12800²`);
  process.exit(1);
}
const dem = PNG.sync.read(readFileSync(DEM_PATH), { skipRescale: true });
const DW = dem.width; // 6400, row 0 = south
const dStride = dem.data.length / (DW * dem.height);
const { heightRangeMinM: LO, heightRangeMaxM: HI } = MANIFEST.dem;
const SEA_U16 = Math.round(((0 - LO) / (HI - LO)) * 65535);
const M_PER_U16 = (HI - LO) / 65535;
const inland = PNG.sync.read(readFileSync(INLAND));
if (inland.width !== W || inland.height !== W) {
  console.error(`inland mask is ${inland.width}x${inland.height}, expected ${W}²`);
  process.exit(1);
}

// ── Build the water alpha plane (0 = untouched land) + per-pixel class ──────────────────
// alpha starts binary (255 in water), then an inward-only feather: blurred alpha is taken
// but re-masked so pixels outside the water mask stay exactly 0 → land bytes never change.
log("building water masks");
const N = W * W;
const alpha = new Uint8Array(N); // 255 inside water, 0 outside
const isOcean = new Uint8Array(N);
let oceanPx = 0;
let inlandPx = 0;
for (let y = 0; y < W; y++) {
  const demY = Math.min(DW - 1, (W - 1 - y) >> 1); // north-up image row → south-up DEM row
  for (let x = 0; x < W; x++) {
    const i = y * W + x;
    const v = dem.data[(demY * DW + (x >> 1)) * dStride];
    if (v <= SEA_U16) {
      alpha[i] = 255;
      isOcean[i] = 1;
      oceanPx++;
    } else if (inland.data[i * 4] > 127) {
      alpha[i] = 255;
      inlandPx++;
    }
  }
}
log(
  `ocean ${(oceanPx / 1e6).toFixed(1)} Mpx, inland ${(inlandPx / 1e6).toFixed(2)} Mpx (${(inlandPx / 1e4).toFixed(0)} ha)`,
);

// Inward feather: separable box blur ×2 (≈ gaussian), then clamp to the original mask.
log(`feathering (inward, r=${FEATHER_R})`);
const blurPass = (src) => {
  const r = FEATHER_R;
  const tmp = new Uint8Array(N);
  const win = 2 * r + 1;
  for (let y = 0; y < W; y++) {
    let acc = 0;
    const row = y * W;
    for (let x = -r; x <= r; x++) acc += src[row + Math.max(0, Math.min(W - 1, x))];
    for (let x = 0; x < W; x++) {
      tmp[row + x] = (acc / win) | 0;
      acc += src[row + Math.min(W - 1, x + r + 1)] - src[row + Math.max(0, x - r)];
    }
  }
  const out = new Uint8Array(N);
  for (let x = 0; x < W; x++) {
    let acc = 0;
    for (let y = -r; y <= r; y++) acc += tmp[Math.max(0, Math.min(W - 1, y)) * W + x];
    for (let y = 0; y < W; y++) {
      out[y * W + x] = (acc / win) | 0;
      acc += tmp[Math.min(W - 1, y + r + 1) * W + x] - tmp[Math.max(0, y - r) * W + x];
    }
  }
  return out;
};
let soft = blurPass(blurPass(alpha));
for (let i = 0; i < N; i++) soft[i] = alpha[i] ? soft[i] : 0; // inward only

// ── Blend ────────────────────────────────────────────────────────────────────────────────
log("blending");
const d = ortho.data;
for (let y = 0; y < W; y++) {
  const demY = Math.min(DW - 1, (W - 1 - y) >> 1);
  for (let x = 0; x < W; x++) {
    const i = y * W + x;
    const a8 = soft[i];
    if (a8 === 0) continue; // land: byte-identical
    let cr;
    let cg;
    let cb;
    if (isOcean[i]) {
      const v = dem.data[(demY * DW + (x >> 1)) * dStride];
      const depthM = (SEA_U16 - v) * M_PER_U16;
      const t = Math.min(1, depthM / DEPTH_FULL_M);
      cr = OCEAN_BRIGHT[0] + (OCEAN_DARK[0] - OCEAN_BRIGHT[0]) * t;
      cg = OCEAN_BRIGHT[1] + (OCEAN_DARK[1] - OCEAN_BRIGHT[1]) * t;
      cb = OCEAN_BRIGHT[2] + (OCEAN_DARK[2] - OCEAN_BRIGHT[2]) * t;
    } else {
      cr = INLAND_COLOR[0];
      cg = INLAND_COLOR[1];
      cb = INLAND_COLOR[2];
    }
    const a = (a8 / 255) * WATER_ALPHA;
    const o = i * 4;
    d[o] = Math.round(d[o] * (1 - a) + cr * a);
    d[o + 1] = Math.round(d[o + 1] * (1 - a) + cg * a);
    d[o + 2] = Math.round(d[o + 2] * (1 - a) + cb * a);
  }
}

// ── Ship: backup once, overwrite canonical, append meta block ────────────────────────────
if (!existsSync(BACKUP)) {
  log("backing up pre-water ortho");
  copyFileSync(ORTHO, BACKUP);
}
log("writing composited ortho");
writeFileSync(ORTHO, PNG.sync.write(ortho));

meta.waterComposite = {
  slice: "T-090.1.2.5",
  refineSlice: "T-090.1.2.5.2",
  oceanMaskSource: "dem-below-sea-level",
  inlandMaskSource:
    "supertexture-water-appearance-dem-filtered + topo-road-subtraction (exact .topo road " +
    "network guard; relaxed wet-channel stream class)",
  spikeArtifact: ".ai/artifacts/t090_1_2_5_water_source_spike.json",
  refineSpikeArtifact: ".ai/artifacts/t090_1_2_5_2_source_spike.json",
  palette: { oceanBright: OCEAN_BRIGHT, oceanDark: OCEAN_DARK, inland: INLAND_COLOR },
  waterAlpha: WATER_ALPHA,
  depthFullM: DEPTH_FULL_M,
  featherRadiusPx: FEATHER_R,
  featherMode: "inward-only (land pixels outside the mask are byte-identical)",
  oceanPx,
  inlandPx,
  generatedAt: new Date().toISOString(),
};
writeFileSync(META, JSON.stringify(meta, null, 2) + "\n");
log(`done in ${((Date.now() - t0) / 1000).toFixed(0)}s — meta.waterComposite written`);
