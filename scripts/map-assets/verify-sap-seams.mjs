// T-090.1.2.2 — ship gate for the SAP cell seam repair (run AFTER stitch/blend).
//
// The operator-visible artifact is a DEAD-FLAT band (baked cell apron) at every interior 256 px
// seam, NOT an exposure step — on the broken ortho the cross-seam ΔRGB is already ~0.1, so a
// "grid-edge mean ΔRGB" gate cannot see it (see analyze-sap-seams.mjs / spec). This gate
// therefore asserts the FLAT BAND is gone where the terrain is textured, keeps ΔRGB only as a
// no-new-step guard, and holds the global-stddev floor so nothing was blurred flat.
//
// Assertions (thresholds locked in lib/sap-seam-metrics.mjs; documented in the verify log):
//   1. ortho exists 12800² (via magick identify)
//   2. FILL: every TEXTURED interior seam (interiorGrad > DETAIL_MIN) has its contiguous flat run
//      (apron) removed (≤ 1 line) AND the band recovered to ≥ REL_FLOOR of local interior detail —
//      no dead-flat band remains. (An absolute band-gradient floor is contrast-dependent: a linear
//      bridge yields gradient ∝ local contrast, so apron-removal + relative recovery is the
//      contrast-invariant gate; absolute bandMinGrad ≥ FILL_FLOOR is reported.) Uniform seams
//      (e.g. open water) are legitimately flat and are not evaluated.
//   3. STEP guard: every seam's cross-seam mean ΔRGB ≤ STEP_CAP (the bridge introduced no
//      exposure discontinuity)
//   4. ANCHOR (NIT-1): no seam is anchor-unsafe (apron never reached the bridge anchors)
//   5. CONTROL: interior non-seam lines stay textured (bandMinGrad ≥ FILL_FLOOR) — proves the
//      metric distinguishes seam (was flat) from interior (detail), i.e. the gate isn't trivial
//   6. global stddev > MIN_STDDEV — no whole-map blur/flatten
//
// Exit 1 on any failure; else prints `verify-sap-seams OK`.
//   node scripts/map-assets/verify-sap-seams.mjs TERRAIN=everon
import { execFileSync } from "node:child_process";
import { existsSync } from "node:fs";
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
const MIN_STDDEV = 0.02; // mirror verify-sap-ortho: a flat/grey placeholder is ~0, real ground >> this

const errors = [];
const ok = (m) => console.log(`  ok: ${m}`);

if (!existsSync(orthoPath)) {
  console.error(`verify-sap-seams FAIL: missing ${orthoPath} — run stitch-sap-ortho.mjs first`);
  process.exit(1);
}

// 1) dims + global stddev
let stddev = null;
try {
  const out = execFileSync(
    "magick",
    ["identify", "-format", "%w %h %[fx:standard_deviation]", orthoPath],
    { encoding: "utf8" },
  ).trim();
  const [w, h, sd] = out.split(/\s+/);
  if (+w !== 12800 || +h !== 12800) errors.push(`ortho ${w}x${h} != 12800²`);
  else ok(`ortho ${w}x${h}`);
  stddev = +sd;
} catch (e) {
  errors.push(`magick identify failed: ${e.message}`);
}

const { buf, w, h } = loadOrthoRgb(orthoPath);
const res = analyzeSeams(buf, w, h);
const sum = summarize(res);

// 2) FILL — flat band (apron) removed on textured seams
if (sum.evaluatedCount === 0) {
  errors.push(`no textured seams evaluated (interiorGrad > ${DETAIL_MIN}) — unexpected for everon`);
} else if (sum.fillFailures.length > 0) {
  const sample = sum.fillFailures
    .slice(0, 6)
    .map((s) => `${s.axis}k=${s.k}(apron ${s.apronLeft}/${s.apronRight}, band ${s.bandMinGrad})`)
    .join(", ");
  errors.push(
    `FILL: ${sum.fillFailures.length}/${sum.evaluatedCount} textured seams still flat ` +
      `(apron > 1 or recovery < ${REL_FLOOR}× interior): ${sample}`,
  );
} else {
  ok(
    `FILL: flat band removed on all ${sum.evaluatedCount} textured seams — worst apron ${sum.worstApron} (≤1), ` +
      `worst recovery ${sum.worstRatio} (≥ ${REL_FLOOR}); abs bandMinGrad ≥ ${FILL_FLOOR} on ${sum.absoluteFloorMet}/${sum.evaluatedCount}`,
  );
}

// 3) STEP guard — no new exposure step
if (sum.stepFailures.length > 0) {
  const sample = sum.stepFailures.slice(0, 6).map((s) => `${s.axis}k=${s.k}(${s.stepDeltaRgb})`).join(", ");
  errors.push(`STEP: ${sum.stepFailures.length} seams exceed STEP_CAP ${STEP_CAP}: ${sample}`);
} else {
  ok(`STEP guard: max cross-seam ΔRGB ${sum.maxStepDelta} ≤ STEP_CAP ${STEP_CAP}`);
}

// 4) ANCHOR safety (NIT-1)
if (sum.anchorUnsafe.length > 0) {
  const sample = sum.anchorUnsafe
    .slice(0, 6)
    .map((s) => `${s.axis}k=${s.k}(apron ${s.apronLeft}/${s.apronRight})`)
    .join(", ");
  errors.push(`ANCHOR: ${sum.anchorUnsafe.length} seams anchor-unsafe (apron reached anchors): ${sample}`);
} else {
  ok(`ANCHOR safety: all seams clear (apron never reached bridge anchors)`);
}

// 5) CONTROL — interior non-seam lines stay textured
const flatControls = res.controls.filter((c) => c.bandMinGrad < FILL_FLOOR);
if (flatControls.length > 0) {
  errors.push(
    `CONTROL: interior line(s) unexpectedly flat (< FILL_FLOOR ${FILL_FLOOR}): ` +
      flatControls.map((c) => `${c.axis}@${c.at}(${c.bandMinGrad})`).join(", "),
  );
} else {
  ok(`control interior lines textured (${res.controls.map((c) => c.bandMinGrad).join(", ")})`);
}

// 6) global stddev floor
if (!(stddev > MIN_STDDEV)) errors.push(`ortho stddev ${stddev} <= ${MIN_STDDEV} (whole-map blur/flatten?)`);
else ok(`global stddev ${stddev.toFixed(4)} (> ${MIN_STDDEV})`);

if (errors.length) {
  console.error(`\nverify-sap-seams FAIL (${errors.length}):`);
  for (const e of errors) console.error(`  - ${e}`);
  process.exit(1);
}
console.log("\nverify-sap-seams OK");
