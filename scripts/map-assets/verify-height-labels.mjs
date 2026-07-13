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

let labels = labelsRaw;
const wasmPkg = join(repoRoot, "apps", "website", "frontend", "src", "wasm", "pkg", "map_engine_wasm.js");
if (existsSync(wasmPkg)) {
  const wasm = await import(wasmPkg);
  labels = JSON.parse(wasm.declutter_height_labels_json(JSON.stringify(labelsRaw), 0));
}

const gateErrors = verifyHeightLabelGates(labels, 0);
if (gateErrors.length === 0) {
  pass(`G4 declutter @ z=0 (sep 80 m)`);
  pass(`G5 count ${labels.length} ≤ ${PEAK_LABEL_MAX}`);
  const maxV = Math.max(...labels.map((l) => l.value_m));
  pass(`G6 max(value_m)=${maxV} ≥ 350`);
} else {
  for (const e of gateErrors) fail(e);
}

// G2/G3 wasm oracle when DEM present
if (existsSync(manifestPath) && existsSync(demPath) && existsSync(wasmPkg)) {
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  const dem = manifest.dem;
  const flip = dem.axisFlip ?? {};
  const minM = dem.heightRangeMinM;
  const maxM = dem.heightRangeMaxM;
  const buf = readFileSync(demPath);
  const wasm = await import(wasmPkg);
  const decoded = wasm.dem_decode_png_to_meters(buf, minM, maxM);
  const meters = decoded.meters;
  const width = decoded.width;
  const height = decoded.height;
  const errJson = wasm.verify_height_labels_json(
    JSON.stringify(labels),
    meters,
    width,
    height,
    0,
    0,
    manifest.worldBounds[2],
    manifest.worldBounds[3],
    Boolean(flip.x),
    Boolean(flip.z),
    minM,
    maxM,
  );
  const errs = JSON.parse(errJson);
  if (errs.length === 0) {
    pass("G2 ASL ±0.5 m (wasm oracle)");
    pass("G3 sample_elevation > 0 (wasm oracle)");
  } else {
    for (const e of errs.slice(0, 5)) fail(e);
    if (errs.length > 5) fail(`… +${errs.length - 5} more G2/G3 errors`);
  }
  if (wasm.height_contour_labels_waived()) {
    pass("G-contour operator waived (contour index labels optional)");
  }
} else if (!existsSync(wasmPkg)) {
  fail("G2/G3 wasm pkg missing — run make wasm");
} else {
  fail("G2/G3 DEM missing for oracle");
}

if (failures) {
  console.error(`\nverify-height-labels: FAIL (${failures})`);
  process.exit(1);
}
console.log("\nverify-height-labels: OK");
