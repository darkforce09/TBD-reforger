#!/usr/bin/env node
// T-152.7 — mathematical gates G2–G6 for committed height-labels.json.
import { readFileSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { PEAK_LABEL_MAX, verifyHeightLabelGates } from "./lib/height-labels-export.mjs";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const terrain = process.env.TERRAIN ?? process.argv.find((a) => a.startsWith("TERRAIN="))?.split("=")[1] ?? "everon";
const labelPath = join(repoRoot, "packages", "map-assets", terrain, "height-labels.json");
const manifestPath = join(repoRoot, "packages", "map-assets", terrain, "manifest.json");
const demPath = join(repoRoot, "packages", "map-assets", terrain, "dem", "everon-dem-16bit.png");

if (!existsSync(labelPath)) {
  console.error(`verify-height-labels: missing ${labelPath}`);
  process.exit(1);
}

const labelsRaw = JSON.parse(readFileSync(labelPath, "utf8"));
let failures = 0;
const pass = (msg) => console.log(`  PASS  ${msg}`);
const fail = (msg) => {
  failures++;
  console.log(`  FAIL  ${msg}`);
};

console.log(`verify-height-labels (${terrain}):`);

const skip = (msg) => console.log(`  SKIP  ${msg}`);
const note = (msg) => console.log(`  NOTE  ${msg}`);
const isNamed = (l) => typeof l.name === "string" && l.name.length > 0;
const named = labelsRaw.filter(isNamed);
const unnamed = labelsRaw.filter((l) => !isNamed(l));

// ── T-152.16 pure gates (operate on the RAW sidecar; no wasm/DEM needed) ──

// G2 floor: 0 rows below the value floor — ONE rule for named + unnamed (T-152.16 operator
// decision: coastal features mis-tagged peak/hill sample sub-floor and are dropped, not shipped).
const FLOOR_M = 80; // mirrors Rust PEAK_MIN_VALUE_M
const belowFloor = labelsRaw.filter((l) => l.value_m < FLOOR_M);
if (belowFloor.length === 0) {
  pass(`G2 floor: 0 rows < ${FLOOR_M} m (${named.length} named + ${unnamed.length} DEM)`);
} else {
  fail(`G2 floor: ${belowFloor.length} rows < ${FLOOR_M} m [${belowFloor.map((l) => `${l.name ?? "?"}=${l.value_m}`).join(", ")}]`);
}

// G4 dedupe: no UNNAMED DEM peak within 200 m of a named row (named wins). Named-vs-named
// proximity is allowed — real toponyms cluster (e.g. Highstone / Center North Hill 01 ~99 m).
const collisions = unnamed.filter((u) => named.some((n) => Math.hypot(u.x - n.x, u.y - n.y) < 200));
if (collisions.length === 0) {
  pass(`G4 dedupe: no DEM peak within 200 m of a named row`);
} else {
  fail(`G4 dedupe: ${collisions.length} DEM peaks within 200 m of a named row`);
}

// G3 named merge: every named sidecar row traces to a real locations.json peak/hill (no orphans /
// invented names). The exact "which sample ≥ floor" completeness is checked in the DEM branch.
const locationsPath = join(repoRoot, "packages", "map-assets", terrain, "locations.json");
let locPeakHill = null;
if (existsSync(locationsPath)) {
  locPeakHill = JSON.parse(readFileSync(locationsPath, "utf8")).filter((l) => l.kind === "peak" || l.kind === "hill");
  const validNames = new Set(locPeakHill.map((l) => l.name));
  const orphans = named.filter((l) => !validNames.has(l.name));
  if (orphans.length === 0) {
    pass(`G3 named merge: ${named.length} named rows all trace to locations.json peak/hill`);
  } else {
    fail(`G3 named merge: ${orphans.length} named rows not in locations.json [${orphans.slice(0, 6).map((o) => o.name).join(", ")}]`);
  }
} else {
  skip("G3 named merge — no locations.json");
}

// ── wasm-dependent gates (declutter math + ASL oracle). The wasm-bindgen pkg died with the
//    React app (T-159.29.3) — the declutter math is pinned by map-engine-core's own cargo tests
//    (dem::peaks) — so this permanently skips unless a pkg is hand-built to the old path. ──
const wasmPkg = join(repoRoot, "apps", "website", "frontend", "src", "wasm", "pkg", "map_engine_wasm.js");
if (!existsSync(wasmPkg)) {
  skip("declutter + ASL oracle — retired with the React wasm pkg (core cargo tests own the math)");
} else {
  const wasm = await import(wasmPkg);
  const drawn = JSON.parse(wasm.declutter_height_labels_json(JSON.stringify(labelsRaw), 0));
  const gateErrors = verifyHeightLabelGates(drawn, 0);
  if (gateErrors.length === 0) {
    pass(`G5 declutter @ z=0 (sep 80 m): ${drawn.length} ≤ ${PEAK_LABEL_MAX} drawn`);
    const maxV = Math.max(...drawn.map((l) => l.value_m));
    pass(`G6 max(value_m)=${maxV} ≥ 350`);
  } else {
    for (const e of gateErrors) fail(e);
  }

  if (existsSync(manifestPath) && existsSync(demPath)) {
    const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
    const dem = manifest.dem;
    const flip = dem.axisFlip ?? {};
    const minM = dem.heightRangeMinM;
    const maxM = dem.heightRangeMaxM;
    const decoded = wasm.dem_decode_png_to_meters(readFileSync(demPath), minM, maxM);
    const errs = JSON.parse(
      wasm.verify_height_labels_json(
        JSON.stringify(labelsRaw),
        decoded.meters,
        decoded.width,
        decoded.height,
        0,
        0,
        manifest.worldBounds[2],
        manifest.worldBounds[3],
        Boolean(flip.x),
        Boolean(flip.z),
        minM,
        maxM,
      ),
    );
    if (errs.length === 0) {
      pass("ASL ±0.5 m + sample > 0 (wasm oracle, all rows incl. named)");
    } else {
      for (const e of errs.slice(0, 5)) fail(e);
      if (errs.length > 5) fail(`… +${errs.length - 5} more ASL errors`);
    }

    // G3 completeness: the sidecar's named set == exactly the locations peak/hill rows that
    // sample ≥ floor (one floor rule). Catches a stale sidecar / drifted merge.
    if (locPeakHill) {
      const xs = Float64Array.from(locPeakHill.map((l) => l.x));
      const ys = Float64Array.from(locPeakHill.map((l) => l.y));
      const elevs = wasm.sample_dem_elevations(
        decoded.meters,
        decoded.width,
        decoded.height,
        0,
        0,
        manifest.worldBounds[2],
        manifest.worldBounds[3],
        Boolean(flip.x),
        Boolean(flip.z),
        xs,
        ys,
      );
      const floorM = wasm.peak_min_value_m();
      const expected = new Set(
        locPeakHill.filter((_, i) => Number.isFinite(elevs[i]) && elevs[i] >= floorM).map((l) => l.name),
      );
      const have = new Set(named.map((l) => l.name));
      const missing = [...expected].filter((n) => !have.has(n));
      const extra = [...have].filter((n) => !expected.has(n));
      if (missing.length === 0 && extra.length === 0) {
        pass(`G3 completeness: named set == ${expected.size} locations peak/hill ≥ ${floorM} m`);
      } else {
        if (missing.length) fail(`G3 completeness: ${missing.length} expected names missing [${missing.slice(0, 6).join(", ")}]`);
        if (extra.length) fail(`G3 completeness: ${extra.length} unexpected named [${extra.slice(0, 6).join(", ")}]`);
      }
    }
  } else {
    skip("ASL oracle — DEM absent (run git lfs pull)");
  }

  if (wasm.height_contour_labels_waived()) {
    note("contour index labels: T-152.16 FRESH operator waiver (see .ai/artifacts/t152_16_verify_log.md)");
  }
}

if (failures) {
  console.error(`\nverify-height-labels: FAIL (${failures})`);
  process.exit(1);
}
console.log("\nverify-height-labels: OK");
