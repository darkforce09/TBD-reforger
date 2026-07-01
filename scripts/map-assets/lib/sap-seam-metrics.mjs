// T-090.1.2.2 — shared SAP-ortho seam metrics (used by analyze-sap-seams.mjs +
// verify-sap-seams.mjs). Dep-free apart from `magick` (already a hard pipeline
// prerequisite) to decode the staged 12800² ortho PNG to raw RGB once.
//
// ROOT CAUSE (proven in P0, read-only): each Eden `_supertexture.edds` cell has a baked
// constant ~3–4 px apron on all four edges (mip0). Tiled edge-to-edge (stitch-sap-ortho.mjs)
// those aprons stack into an ~8 px DEAD-FLAT band at every interior 256 px seam — a grid of
// blurry lines over otherwise-sharp terrain at max zoom. It is NOT an exposure step (cross-seam
// ΔRGB ≈ 0.1 on the broken ortho) and NOT placement (boundaries land exactly on 256·k).
//
// The gate therefore measures the FLAT BAND, not ΔRGB: within each interior seam band the
// minimum per-line gradient must be non-flat WHEN the surrounding cell interior is textured
// (a genuinely uniform region — e.g. open water — legitimately has a flat band and no visible
// seam, so it is not flagged). See docs spec t090_1_2_2_sap_cell_seam_repair.md.
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { CELL_PX, GRID } from "../decode-edds.mjs";

// ── Locked parameters (documented in the verify log) ─────────────────────────
export const HW = 4; // seam-bridge half-width; band = [c-HW, c+HW-1] (8 px, within spec "2–8 px")
export const ANCHOR = HW + 1; // bridge anchor offset: lerp between col/row c-ANCHOR and c+HW
export const FILL_FLOOR = 0.25; // absolute band-gradient reference (reported); met by high/mid-contrast seams
export const REL_FLOOR = 0.05; // primary numeric floor: band must recover to ≥ this fraction of local interior
                               // detail (a linear bridge yields gradient ∝ local contrast, so an absolute floor
                               // would unfairly fail low-contrast coastal seams; observed post-fix ratios 0.08–0.16).
export const STEP_CAP = 6.0; // guard: cross-seam mean ΔRGB (anchor strips) must stay ≤ this (no new exposure step)
export const DETAIL_MIN = 1.0; // a seam is only evaluated where the cell interior gradient exceeds this (else uniform → skip)
export const FLAT_EPS = 0.15; // per-line gradient below this counts as "flat" (apron) when measuring apron width
export const STRIP = 8; // anchor-strip width for the ΔRGB guard
export const ORTHO_PX = GRID * CELL_PX; // 12800

/** Decode the ortho PNG to a raw RGB buffer (row-major, 3 B/px) via magick. Caller-owned. */
export function loadOrthoRgb(pngPath) {
  const tmp = mkdtempSync(join(tmpdir(), "sap-seams-"));
  const raw = join(tmp, "ortho.rgb");
  try {
    execFileSync("magick", [pngPath, "-depth", "8", `RGB:${raw}`], { stdio: "inherit" });
    const buf = readFileSync(raw);
    const px = ORTHO_PX * ORTHO_PX * 3;
    if (buf.length !== px) {
      throw new Error(`ortho raw ${buf.length} B != ${px} (expected ${ORTHO_PX}² RGB)`);
    }
    return { buf, w: ORTHO_PX, h: ORTHO_PX };
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
}

const STRIDE = ORTHO_PX * 3;
const lumSum3 = (buf, o) => buf[o] + buf[o + 1] + buf[o + 2]; // (r+g+b), 0..765

/** Mean over rows of |lum(x) - lum(x+1)|, lum=(r+g+b)/3 → 0..255 per-channel scale. */
function colGrad(buf, w, h, x) {
  let s = 0;
  for (let y = 0; y < h; y++) {
    const o = y * STRIDE + x * 3;
    s += Math.abs(lumSum3(buf, o) - lumSum3(buf, o + 3));
  }
  return s / (3 * h);
}

/** Mean over cols of |lum(y) - lum(y+1)|. */
function rowGrad(buf, w, h, y) {
  let s = 0;
  const base = y * STRIDE;
  for (let x = 0; x < w; x++) {
    const o = base + x * 3;
    s += Math.abs(lumSum3(buf, o) - lumSum3(buf, o + STRIDE));
  }
  return s / (3 * w);
}

/** Mean [r,g,b] over a column strip [x0,x1) × all rows. */
function colStripMeanRgb(buf, w, h, x0, x1) {
  let r = 0, g = 0, b = 0;
  let n = 0;
  for (let y = 0; y < h; y++) {
    const base = y * STRIDE;
    for (let x = x0; x < x1; x++) {
      const o = base + x * 3;
      r += buf[o];
      g += buf[o + 1];
      b += buf[o + 2];
      n++;
    }
  }
  return [r / n, g / n, b / n];
}

/** Mean [r,g,b] over a row strip [y0,y1) × all cols. */
function rowStripMeanRgb(buf, w, h, y0, y1) {
  let r = 0, g = 0, b = 0;
  let n = 0;
  for (let y = y0; y < y1; y++) {
    const base = y * STRIDE;
    for (let x = 0; x < w; x++) {
      const o = base + x * 3;
      r += buf[o];
      g += buf[o + 1];
      b += buf[o + 2];
      n++;
    }
  }
  return [r / n, g / n, b / n];
}

const meanAbsDeltaRgb = (a, b) => (Math.abs(a[0] - b[0]) + Math.abs(a[1] - b[1]) + Math.abs(a[2] - b[2])) / 3;
const mean = (arr) => arr.reduce((s, v) => s + v, 0) / arr.length;
const r2 = (v) => Math.round(v * 100) / 100;

// Count contiguous flat lines (grad < FLAT_EPS) outward from the seam, on each side.
// gAt(i) = gradient between line i and i+1.
function apronWidths(gAt, c, maxScan = 8) {
  let left = 0;
  for (let i = c - 1; i >= c - maxScan; i--) {
    if (gAt(i) < FLAT_EPS) left++;
    else break;
  }
  let right = 0;
  for (let i = c; i <= c + maxScan - 1; i++) {
    if (gAt(i) < FLAT_EPS) right++;
    else break;
  }
  return { left, right };
}

/** Metric for one interior seam. axis: 'v' (x=c) or 'h' (y=c). */
function seamMetric(buf, w, h, c, axis) {
  const grad = axis === "v" ? (x) => colGrad(buf, w, h, x) : (y) => rowGrad(buf, w, h, y);
  // Precompute the gradient window once (band + interior refs + apron scan).
  const g = new Map();
  const gAt = (i) => {
    let v = g.get(i);
    if (v === undefined) {
      v = grad(i);
      g.set(i, v);
    }
    return v;
  };
  // Band = [c-HW, c+HW-1]; bandMinGrad = flattest line inside it.
  const bandGrads = [];
  for (let i = c - HW; i <= c + HW - 1; i++) bandGrads.push(gAt(i));
  const bandMinGrad = Math.min(...bandGrads);
  // Interior reference (both cells, clear of apron and next seam): [c-20,c-13] ∪ [c+12,c+19].
  const refs = [];
  for (let i = c - 20; i <= c - 13; i++) refs.push(gAt(i));
  for (let i = c + 12; i <= c + 19; i++) refs.push(gAt(i));
  const interiorGrad = mean(refs);
  const apron = apronWidths(gAt, c);
  // Anchors used by the bridge sit at c-ANCHOR and c+HW; unsafe if a flat run reaches them
  // while the neighbourhood is textured (apron wider than expected).
  const anchorSafe = !(interiorGrad > DETAIL_MIN && (apron.left >= ANCHOR || apron.right >= ANCHOR));
  // Cross-seam DC step (anchor-region strips, outside the band): guards against a new exposure step.
  const stripMean = axis === "v" ? colStripMeanRgb : rowStripMeanRgb;
  const left = stripMean(buf, w, h, c - 12, c - 4);
  const right = stripMean(buf, w, h, c + 4, c + 12);
  const stepDeltaRgb = meanAbsDeltaRgb(left, right);
  const evaluated = interiorGrad > DETAIL_MIN;
  return {
    axis,
    k: c / CELL_PX,
    c,
    bandMinGrad: r2(bandMinGrad),
    interiorGrad: r2(interiorGrad),
    apronLeft: apron.left,
    apronRight: apron.right,
    anchorSafe,
    stepDeltaRgb: r2(stepDeltaRgb),
    evaluated,
  };
}

/** Interior non-seam control line (cell centre) — should read as textured (not flat). */
function controlMetric(buf, w, h, c, axis) {
  const grad = axis === "v" ? (x) => colGrad(buf, w, h, x) : (y) => rowGrad(buf, w, h, y);
  const gs = [];
  for (let i = c - HW; i <= c + HW - 1; i++) gs.push(grad(i));
  return { axis, at: c, bandMinGrad: r2(Math.min(...gs)) };
}

/** Full seam sweep: all interior vertical + horizontal seams + a few interior controls. */
export function analyzeSeams(buf, w, h) {
  const vertical = [];
  const horizontal = [];
  for (let k = 1; k < GRID; k++) {
    vertical.push(seamMetric(buf, w, h, k * CELL_PX, "v"));
    horizontal.push(seamMetric(buf, w, h, k * CELL_PX, "h"));
  }
  // Controls: mid-cell lines (guaranteed off any seam), a couple of each axis.
  const controls = [
    controlMetric(buf, w, h, 25 * CELL_PX + 128, "v"),
    controlMetric(buf, w, h, 30 * CELL_PX + 128, "v"),
    controlMetric(buf, w, h, 25 * CELL_PX + 128, "h"),
  ];
  return { vertical, horizontal, controls };
}

/** Reduce a sweep to pass/fail facts against the thresholds. */
export function summarize(res) {
  const all = [...res.vertical, ...res.horizontal];
  const evaluated = all.filter((s) => s.evaluated);
  const worst = evaluated.reduce(
    (w, s) => (w === null || s.bandMinGrad < w.bandMinGrad ? s : w),
    null,
  );
  // FILL pass = the flat strip is gone. Primary (contrast-invariant): the contiguous flat run
  // (apron) is removed (≤1 line). Secondary: the band recovered to ≥ REL_FLOOR of local interior
  // detail (a linear bridge yields gradient ∝ contrast). A dead-flat baseline fails both.
  const fillFailures = evaluated.filter(
    (s) => s.apronLeft > 1 || s.apronRight > 1 || s.bandMinGrad < REL_FLOOR * s.interiorGrad,
  );
  const stepFailures = all.filter((s) => s.stepDeltaRgb > STEP_CAP);
  const anchorUnsafe = all.filter((s) => !s.anchorSafe);
  const meanBandMinGradEval = evaluated.length ? r2(mean(evaluated.map((s) => s.bandMinGrad))) : null;
  const maxStepDelta = r2(Math.max(...all.map((s) => s.stepDeltaRgb)));
  const absoluteFloorMet = evaluated.filter((s) => s.bandMinGrad >= FILL_FLOOR).length;
  const worstApron = evaluated.length ? Math.max(...evaluated.map((s) => Math.max(s.apronLeft, s.apronRight))) : 0;
  const worstRatio = evaluated.length
    ? r2(Math.min(...evaluated.map((s) => (s.interiorGrad > 0 ? s.bandMinGrad / s.interiorGrad : 1))))
    : null;
  return {
    seamCount: all.length,
    evaluatedCount: evaluated.length,
    worstEvaluated: worst,
    worstBandMinGrad: worst ? worst.bandMinGrad : null,
    meanBandMinGradEval,
    maxStepDelta,
    absoluteFloorMet,
    worstApron,
    worstRatio,
    fillFailures,
    stepFailures,
    anchorUnsafe,
  };
}
