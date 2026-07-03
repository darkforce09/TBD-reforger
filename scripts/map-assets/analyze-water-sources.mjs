// T-090.1.2.5 — P0 water-source spike: pick the hydrology mask provenance for the
// satellite water composite (ocean + inland) BEFORE any compositing happens.
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
const OUT_JSON = join(REPO, ".ai/artifacts/t090_1_2_5_water_source_spike.json");
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
const MEAN_SAT_MAX = 0.115; // component-level mean saturation acceptance
const FLAT_FRAC_MAX = 0.5; // component rejected if >50 % engine-exact-flat (pavement)
const SLOPE_PX_MAX_DEG = 18; // grey px on steeper ground = rock face, not water
const SLOPE_MEAN_MAX_DEG = 8; // component-level mean slope acceptance (water sits in valleys)

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
for (let y = 0; y < DW; y++) {
  const ny = DW - 1 - y; // south-up → north-up
  for (let x = 0; x < DW; x++) {
    const di = (ny >> 1) * D + (x >> 1);
    if (demV(x, y) <= SEA_U16) sea[di] = 1;
    if (flat6400[y * DW + x]) flat[di] = 1;
    if (x > 0 && x < DW - 1 && y > 0 && y < DW - 1) {
      const gx = ((demV(x + 1, y) - demV(x - 1, y)) * M_PER_U16) / 4; // 2*2 m spacing
      const gy = ((demV(x, y + 1) - demV(x, y - 1)) * M_PER_U16) / 4;
      const s = (Math.atan(Math.hypot(gx, gy)) * 180) / Math.PI;
      if (s > slope[di]) slope[di] = s;
    }
  }
}
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
let greyPx = 0;
let greyOnSeaPx = 0;
let seaPx = 0;
for (let i = 0; i < D * D; i++) {
  const isGrey =
    sat[i] < SAT_MAX && lum[i] > LUM_MIN && lum[i] < LUM_MAX && slope[i] <= SLOPE_PX_MAX_DEG;
  if (sea[i]) {
    seaPx++;
    if (isGrey) greyOnSeaPx++;
  }
  if (isGrey && !seaWide[i]) {
    grey[i] = 1;
    greyPx++;
  }
}
const greyOceanRecall = greyOnSeaPx / seaPx; // engine water rendering ↔ grey correlation
log(
  `grey px inland (pre-open): ${greyPx}; ocean grey recall ${(greyOceanRecall * 100).toFixed(1)} %`,
);

// Speckle-tolerant opening: water bodies are mottled at 4 m/px, so a strict erosion dies on
// interior holes. Instead: density core (>= DENSITY_MIN of the (2r+1)² box grey) → dilate the
// core back out and intersect with the original grey. Thin roads (<= ~2 px at this scale)
// never reach the density floor; large bodies keep their true outline.
{
  const r = OPEN_R;
  const side = 2 * r + 1;
  const need = Math.ceil(side * side * DENSITY_MIN);
  // integral image for box sums
  const ii = new Int32Array((D + 1) * (D + 1));
  for (let y = 0; y < D; y++) {
    let row = 0;
    for (let x = 0; x < D; x++) {
      row += grey[y * D + x];
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
      if (!grey[y * D + x]) continue;
      const s = boxSum(Math.max(0, x - r), Math.max(0, y - r), Math.min(D - 1, x + r), Math.min(D - 1, y + r));
      if (s >= need) core[y * D + x] = 1;
    }
  }
  const coreWide = dilate(core, r + 1);
  for (let i = 0; i < D * D; i++) grey[i] = grey[i] && coreWide[i] ? 1 : 0;
}

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
  let nFlat = 0;
  let minX = D;
  let maxX = 0;
  let minY = D;
  let maxY = 0;
  for (const k of px) {
    sSat += sat[k];
    sSlope += slope[k];
    if (flatWide[k]) nFlat++;
    const x = k % D;
    const y = (k / D) | 0;
    if (x < minX) minX = x;
    if (x > maxX) maxX = x;
    if (y < minY) minY = y;
    if (y > maxY) maxY = y;
  }
  const meanSat = sSat / px.length;
  const meanSlope = sSlope / px.length;
  const flatFrac = nFlat / px.length;
  const areaM2 = px.length * 16;
  const accepted =
    areaM2 >= MIN_AREA_M2 &&
    meanSat <= MEAN_SAT_MAX &&
    flatFrac <= FLAT_FRAC_MAX &&
    meanSlope <= SLOPE_MEAN_MAX_DEG;
  comps.push({
    px,
    accepted,
    areaM2,
    meanSat: +meanSat.toFixed(4),
    meanSlopeDeg: +meanSlope.toFixed(2),
    flatFrac: +flatFrac.toFixed(3),
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
for (const c of accepted.slice(0, 12)) {
  log(
    `  body ${(c.areaM2 / 1e4).toFixed(1)} ha @ world (${c.centreWorldM[0]}, ${c.centreWorldM[1]}) sat=${c.meanSat} slope=${c.meanSlopeDeg}° flat=${c.flatFrac}`,
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

// ── Spike JSON: decision + full evidence trail ───────────────────────────────────────────
const spike = {
  slice: "T-090.1.2.5",
  generatedAt: new Date().toISOString(),
  decision: {
    oceanMask: "A-dem-below-sea-level",
    inlandMask: "E-supertexture-water-appearance-dem-filtered",
    refine: "DEM depth drives the ocean colour ramp (oceanDark→oceanBright)",
    forbiddenMethodsAttestation:
      "No hand-painted lakes, no AI-generated rivers, no solid rectangles: ocean = engine GetTerrainSurfaceY heights <= 0 m; inland = pixels the engine's own supertexture renderer drew with the underwater/seabed treatment, cross-filtered by engine DEM (coast exclusion + engine-flattened-pad rejection) and a morphological opening. Every accepted body is enumerated below for visual audit.",
  },
  candidates: {
    "A-dem-below-sea-level": {
      result: "PASS (ocean only)",
      evidence: {
        demSource: MANIFEST.dem.source,
        seaFraction: +seaFraction.toFixed(4),
        inlandBelowSeaM2,
        note: "68.5 % of the frame is <= 0 m (ocean). Inland water does NOT appear: only ~" +
          `${inlandBelowSeaM2} m² of inland pixels sit below sea level — Everon lakes/rivers ` +
          "are above sea level, so the DEM alone cannot see them (GetTerrainSurfaceY sampled " +
          "the bed, e.g. the central lake bed reads 34–55 m).",
      },
    },
    "B-terrain-layer-material-masks": {
      result: "FAIL (timeboxed)",
      timeSpentMin: 50,
      evidence: {
        decoded:
          "Eden_<N>_layer.edds format cracked: headerless-DDS variant (pixel-format block " +
          "at 0x14, flags 0x41 = RGB|ALPHAPIXELS, 32 bpp BGRA8 masks), chunk table at 0x48 " +
          "(vs 0x5C for the BC7 supertextures), 9 mips 1→256 px; COPY mip byte counts match " +
          "4 B/px exactly; mip0 (256², 262144 B) is multi-block LZ4: [u32 total][u32 blockLen]" +
          "[block]… in 64 KB blocks (decoded bit-exact, 262144/262144).",
        blocker:
          "Channels are per-cell splat WEIGHTS whose material palette lives in Eden_<N>.ttile " +
          "/ Eden.ent — both stored with a non-zlib pak codec (zlib/zstd/brotli/raw-deflate/" +
          "raw-LZ4 all fail; enfusion-mcp PakVirtualFS only inflates zlib). Without the palette " +
          "a 'water/seabed' channel cannot be labelled; channel meaning demonstrably varies " +
          "per cell (deep-ocean cell N=49 weight signature ≠ coastal cell N=0).",
      },
    },
    "C-engine-hydrology-files": {
      result: "BLOCKED (placement data unreadable)",
      evidence: {
        found:
          "worlds/Eden/.Rivers/<hash>_flow.edds ×30 (river flow maps) + worlds/Eden/.Shore/" +
          "shoreMaskAtlas.edds (BC4) — real engine hydrology textures, readable via pak VFS.",
        blocker:
          "River/lake world-space placement lives in Eden.ent (70 MB) — same non-zlib pak " +
          "codec as the .ttile files. Eden.topo (10.3 MB) and Eden.smap (16.4 MB) are readable " +
          "but are unlabelled binary (index-buffer-like streams, no strings); decoding them is " +
          "out of slice scope. Revisit for T-090.8 waterBody regions.",
      },
    },
    "D-workbench-entity-query": {
      result: "NOT NEEDED (and currently unavailable)",
      evidence:
        "GameLib exposes LakeGeneratorEntity/RiverEntity/RiverPartEntity, but wb_state reports " +
        "Workbench in GAME mode (WorldEditorAPI unavailable) at spike time, and slice rules " +
        "forbid new mod plugins. Candidate E removed the need. Ground-truth entity export " +
        "remains the T-090.8 refinement path.",
    },
    "E-supertexture-water-appearance": {
      result: "PASS (inland)",
      evidence: {
        principle:
          "The engine's supertexture renderer paints water areas with the underwater/seabed " +
          "treatment — smooth desaturated grey with no vegetation colour (verified: the " +
          "central lake at world ~(4550, 6100) renders as a uniform grey body while its DEM " +
          "bed reads 34–55 m; the existing verify-sap-ortho orientation guard already relies " +
          "on this same grey↔water correlation and matches the DEM coast at AE ratio ~0.08).",
        calibration: {
          patches: {
            lakeInterior: { sat: 0.084, lum: 0.328 },
            runwayAsphalt: { sat: 0.052, lum: 0.318, note: "rejected by exact-flat DEM filter" },
            deepOceanSmooth: { sat: 0.044, lum: 0.297 },
            grass: { sat: 0.198, lum: 0.259 },
            farmland: { sat: 0.239, lum: 0.279 },
          },
          params: {
            detectDim: DETECT_DIM,
            satMax: SAT_MAX,
            lumMin: LUM_MIN,
            lumMax: LUM_MAX,
            openRadiusPx: OPEN_R,
            oceanDilateRadiusPx: OCEAN_DILATE_R,
            flatDilateRadiusPx: FLAT_DILATE_R,
            minAreaM2: MIN_AREA_M2,
            meanSatMax: MEAN_SAT_MAX,
            flatFracMax: FLAT_FRAC_MAX,
            slopePxMaxDeg: SLOPE_PX_MAX_DEG,
            slopeMeanMaxDeg: SLOPE_MEAN_MAX_DEG,
          },
          dem_filters:
            "ocean exclusion = DEM<=0 dilated; pavement rejection = engine-exact-flat DEM " +
            "plateaus (2×2 identical u16 above sea — runways/graded pads are engine-flattened; " +
            "the NW airfield runways were the two largest flat plateaus and are rejected by " +
            "this filter); roads/roofs removed by the 8 m morphological opening.",
        },
        greyOceanRecall: +greyOceanRecall.toFixed(3),
        acceptedBodies: accepted.map(({ px, ...rest }) => rest),
        rejectedComponentCount: comps.length - accepted.length,
      },
    },
  },
  outputs: {
    inlandMaskPng: "packages/map-assets/everon/staging/sap/water-inland-mask.png (gitignored)",
    previewPng: "packages/map-assets/everon/staging/sap/water-spike-preview.png (gitignored)",
  },
};
writeFileSync(OUT_JSON, JSON.stringify(spike, null, 2) + "\n");
log(`wrote ${OUT_JSON} in ${((Date.now() - t0) / 1000).toFixed(0)}s`);
