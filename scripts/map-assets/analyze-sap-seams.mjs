// T-090.1.2.2 — P0 seam analysis for the stitched SAP ortho (measure BEFORE blending).
//
// Samples every interior 256 px seam (vertical x=256·k, horizontal y=256·k, k=1..49) and
// interior control lines, writing .ai/artifacts/t090_1_2_2_seam_analysis.json + a human
// summary. Diagnosis is driven by the apron/step measurements:
//   - baked_apron_flat_band : textured seams carry a dead-flat band (apron) — the T-090.1.2.2 case
//   - exposure_mismatch     : large cross-seam ΔRGB with no flat band
//   - placement             : flat/step offset by ±1 line (off-by-one / internal mirror) → STOP, strategy D
//
// NIT-1 anchor safety: reports apron width + anchor safety at k=1, k=49 and the worst textured
// seam, so the bridge anchors (c-5 / c+4) can be confirmed to sit in real cell detail before
// locking HW=4. Analysis only — always exits 0.
//
//   node scripts/map-assets/analyze-sap-seams.mjs [TERRAIN=everon]
import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  DETAIL_MIN,
  FILL_FLOOR,
  REL_FLOOR,
  STEP_CAP,
  analyzeSeams,
  loadOrthoRgb,
  summarize,
} from "./lib/sap-seam-metrics.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const REPO = join(HERE, "../..");

const terrain =
  (process.argv.find((a) => a.startsWith("TERRAIN=")) || "TERRAIN=everon").split("=")[1];
if (terrain !== "everon") {
  console.error(`only everon supported this slice (got ${terrain})`);
  process.exit(1);
}

const orthoPath = join(REPO, "packages/map-assets/everon/staging/sap/everon-sap-ortho.png");
const outPath = join(REPO, ".ai/artifacts/t090_1_2_2_seam_analysis.json");

// Global stddev (normalized 0..1) — a whole-map-blur / flatten guard, same source as verify-sap-ortho.
function globalStddev() {
  try {
    const out = execFileSync("magick", ["identify", "-format", "%[fx:standard_deviation]", orthoPath], {
      encoding: "utf8",
    }).trim();
    return Number(out);
  } catch {
    return null;
  }
}

console.error(`analyze-sap-seams: decoding ${orthoPath} …`);
const { buf, w, h } = loadOrthoRgb(orthoPath);
const res = analyzeSeams(buf, w, h);
const sum = summarize(res);
const stddev = globalStddev();

// Diagnosis: textured seams with a dead-flat band (apron ≥1 while interior textured) and small
// cross-seam ΔRGB ⇒ baked apron. A large ΔRGB would be exposure; an off-by-one flat/step would
// be placement (not observed — boundaries land exactly on 256·k).
const texturedWithApron = [...res.vertical, ...res.horizontal].filter(
  (s) => s.evaluated && (s.apronLeft + s.apronRight) >= 2 && s.bandMinGrad < FILL_FLOOR,
);
const anyBigStep = sum.maxStepDelta > STEP_CAP;
const diagnosis = anyBigStep
  ? "exposure_mismatch"
  : texturedWithApron.length > 0
    ? "baked_apron_flat_band"
    : "clean";

// NIT-1 anchor spot-check at the extreme seams + the worst textured seam.
const pick = (arr, k) => arr.find((s) => s.k === k);
const spot = [
  pick(res.vertical, 1),
  pick(res.vertical, 49),
  pick(res.horizontal, 1),
  pick(res.horizontal, 49),
  sum.worstEvaluated,
].filter(Boolean);
const anchorSpot = spot.map((s) => ({
  axis: s.axis,
  k: s.k,
  interiorGrad: s.interiorGrad,
  apronLeft: s.apronLeft,
  apronRight: s.apronRight,
  bandMinGrad: s.bandMinGrad,
  anchorSafe: s.anchorSafe,
}));

const report = {
  slice: "T-090.1.2.2",
  terrain,
  orthoPath: "packages/map-assets/everon/staging/sap/everon-sap-ortho.png",
  gridPx: 256,
  bandPx: 8,
  thresholds: { FILL_FLOOR, REL_FLOOR, STEP_CAP, DETAIL_MIN },
  diagnosis,
  globalStddev: stddev,
  summary: {
    seamCount: sum.seamCount,
    evaluatedCount: sum.evaluatedCount,
    worstApron: sum.worstApron,
    worstRatio: sum.worstRatio,
    worstBandMinGrad: sum.worstBandMinGrad,
    meanBandMinGradEval: sum.meanBandMinGradEval,
    absoluteFloorMet: sum.absoluteFloorMet,
    maxStepDelta: sum.maxStepDelta,
    fillFailureCount: sum.fillFailures.length,
    stepFailureCount: sum.stepFailures.length,
    anchorUnsafeCount: sum.anchorUnsafe.length,
  },
  worstEvaluated: sum.worstEvaluated,
  anchorSpotCheck: anchorSpot,
  vertical: res.vertical,
  horizontal: res.horizontal,
  controls: res.controls,
  generatedAt: new Date().toISOString().replace(/\.\d+Z$/, "Z"),
};

mkdirSync(dirname(outPath), { recursive: true });
writeFileSync(outPath, JSON.stringify(report, null, 2) + "\n");

// ── Human summary ────────────────────────────────────────────────────────────
console.log(`\nanalyze-sap-seams — ${terrain}`);
console.log(`  diagnosis:            ${diagnosis}`);
console.log(`  seams:                ${sum.seamCount} (evaluated/textured: ${sum.evaluatedCount})`);
console.log(`  worst apron (flat run): ${sum.worstApron}  (primary FILL: apron ≤ 1)`);
console.log(`  worst recovery ratio: ${sum.worstRatio}  (REL_FLOOR ${REL_FLOOR})`);
console.log(`  worst bandMinGrad:    ${sum.worstBandMinGrad}  (abs FILL_FLOOR ${FILL_FLOOR}: ${sum.absoluteFloorMet}/${sum.evaluatedCount} met)`);
console.log(`  mean bandMinGrad:     ${sum.meanBandMinGradEval}`);
console.log(`  max stepΔRGB:         ${sum.maxStepDelta}  (STEP_CAP ${STEP_CAP})`);
console.log(`  global stddev:        ${stddev}`);
console.log(`  fill failures:        ${sum.fillFailures.length}`);
console.log(`  step failures:        ${sum.stepFailures.length}`);
console.log(`  anchor-unsafe seams:  ${sum.anchorUnsafe.length}`);
console.log(`  controls (interior bandMinGrad): ${res.controls.map((c) => c.bandMinGrad).join(", ")}`);
console.log(`\n  NIT-1 anchor spot-check (apronL/apronR — anchors at c-5/c+4 must clear the apron):`);
for (const a of anchorSpot) {
  console.log(
    `    ${a.axis} k=${a.k}: interior=${a.interiorGrad} apron ${a.apronLeft}/${a.apronRight} ` +
      `bandMin=${a.bandMinGrad} anchorSafe=${a.anchorSafe}`,
  );
}
if (sum.anchorUnsafe.length > 0) {
  console.log(`\n  WARN: ${sum.anchorUnsafe.length} seam(s) anchor-unsafe (apron ≥ ${5}) — widen anchors or reduce HW.`);
}
console.log(`\n  wrote ${outPath}`);
