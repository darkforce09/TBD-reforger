#!/usr/bin/env node
// T-152.7 — export height-labels.json from Everon DEM via wasm peak detect.
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const terrain = process.env.TERRAIN ?? "everon";
const manifestPath = join(repoRoot, "packages", "map-assets", terrain, "manifest.json");
const demPath = join(repoRoot, "packages", "map-assets", terrain, "dem", "everon-dem-16bit.png");
const outPath = join(repoRoot, "packages", "map-assets", terrain, "height-labels.json");

if (!existsSync(manifestPath) || !existsSync(demPath)) {
  console.error("export-height-labels: missing manifest or DEM — run git lfs pull && make map-assets-link");
  process.exit(1);
}

const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
const dem = manifest.dem;
const flip = dem.axisFlip ?? {};
const minM = dem.heightRangeMinM;
const maxM = dem.heightRangeMaxM;

const wasmPkg = join(repoRoot, "apps", "website", "frontend", "src", "wasm", "pkg", "map_engine_wasm.js");
if (!existsSync(wasmPkg)) {
  console.error("export-height-labels: wasm pkg missing — run make wasm");
  process.exit(1);
}

const wasm = await import(wasmPkg);
const pngBytes = readFileSync(demPath);
const decoded = wasm.dem_decode_png_to_meters(pngBytes, minM, maxM);
const meters = decoded.meters;
const width = decoded.width;
const height = decoded.height;

const peaksJson = wasm.find_peaks_from_meters(
  meters,
  width,
  height,
  0,
  0,
  manifest.worldBounds[2],
  manifest.worldBounds[3],
  Boolean(flip.x),
  Boolean(flip.z),
);
const peaks = JSON.parse(peaksJson);
const drawnJson = wasm.declutter_height_labels_json(peaksJson, 0);
const drawn = JSON.parse(drawnJson);

writeFileSync(outPath, JSON.stringify(peaks, null, 2) + "\n");
console.log(`export-height-labels: ${peaks.length} peaks (${drawn.length} @ z=0) → ${outPath}`);
