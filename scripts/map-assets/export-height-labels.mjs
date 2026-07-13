#!/usr/bin/env node
// T-152.7 — export height-labels.json from Everon DEM via wasm peak detect.
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const argv = process.argv.slice(2);
const terrainArgIdx = argv.indexOf("--terrain");
const terrain =
  terrainArgIdx >= 0 && argv[terrainArgIdx + 1] ? argv[terrainArgIdx + 1] : (process.env.TERRAIN ?? "everon");
const manifestPath = join(repoRoot, "packages", "map-assets", terrain, "manifest.json");
const demPath = join(repoRoot, "packages", "map-assets", terrain, "dem", "everon-dem-16bit.png");
const locationsPath = join(repoRoot, "packages", "map-assets", terrain, "locations.json");
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
const peaks = JSON.parse(peaksJson); // unnamed DEM peaks — already floored (≥80 m) in Rust find_peaks

// T-152.16 — named merge: pull peak/hill toponyms from locations.json and sample their DEM
// elevation (Rust owns the sampling via wasm.sample_dem_elevations). Named rows are kept
// unconditionally (Q2: the 80 m floor applies only to unnamed DEM peaks); a DEM peak within
// 200 m of a named row is dropped so the named toponym wins.
const NAMED_KINDS = new Set(["peak", "hill"]);
const DEDUPE_RADIUS_M = 200;
const FLOOR_M = wasm.peak_min_value_m(); // single-sourced from Rust PEAK_MIN_VALUE_M

let named = [];
let namedDropped = [];
if (existsSync(locationsPath)) {
  const locations = JSON.parse(readFileSync(locationsPath, "utf8"));
  const namedSrc = locations.filter((l) => NAMED_KINDS.has(l.kind));
  const xs = Float64Array.from(namedSrc.map((l) => l.x));
  const ys = Float64Array.from(namedSrc.map((l) => l.y));
  const elevs = wasm.sample_dem_elevations(
    meters,
    width,
    height,
    0,
    0,
    manifest.worldBounds[2],
    manifest.worldBounds[3],
    Boolean(flip.x),
    Boolean(flip.z),
    xs,
    ys,
  );
  const namedAll = namedSrc
    .map((l, i) => ({ l, elev: elevs[i] }))
    .filter(({ l, elev }) => {
      if (!Number.isFinite(elev) || elev <= 0) {
        console.warn(`export-height-labels: skip named "${l.name}" — no DEM sample (elev=${elev})`);
        return false;
      }
      return true;
    })
    .map(({ l, elev }) => ({
      x: l.x,
      y: l.y,
      value_m: Math.round(elev),
      kind: "peak", // HeightLabelKind is peak|contour only; hill→peak here (kind fixes are T-152.17/.19)
      name: l.name,
    }));
  // T-152.16 (operator decision): one floor rule — named peaks/hills below the value floor are
  // dropped just like anonymous DEM peaks. locations.json tags several coastal features (beach,
  // headland, lighthouse, coast) as peak/hill; those sample < floor and are not credible height
  // markers. Kind fixes remain T-152.17/.19; here the floor alone removes them.
  named = namedAll.filter((r) => r.value_m >= FLOOR_M);
  namedDropped = namedAll.filter((r) => r.value_m < FLOOR_M);
} else {
  console.warn(`export-height-labels: no locations.json at ${locationsPath} — named merge skipped`);
}

// Dedupe: drop an unnamed DEM peak sitting within 200 m of any named row (named wins).
const demDeduped = peaks.filter((p) => !named.some((n) => Math.hypot(p.x - n.x, p.y - n.y) < DEDUPE_RADIUS_M));

const out = [...named, ...demDeduped];
const drawnJson = wasm.declutter_height_labels_json(JSON.stringify(out), 0);
const drawn = JSON.parse(drawnJson);

writeFileSync(outPath, JSON.stringify(out, null, 2) + "\n");

// Census (T-152.16 RETURN): count · named/DEM split · min value · drawn@z0 · dropped sub-floor named.
const minValue = out.reduce((mn, r) => Math.min(mn, r.value_m), Infinity);
const namedFrac = out.length ? ((named.length / out.length) * 100).toFixed(0) : "0";
console.log(
  `export-height-labels: ${out.length} labels (${named.length} named + ${demDeduped.length} DEM = ${namedFrac}% named; ` +
    `${drawn.length} @ z=0), min=${Number.isFinite(minValue) ? minValue : "-"} m → ${outPath}`,
);
if (namedDropped.length > 0) {
  console.warn(
    `export-height-labels: dropped ${namedDropped.length} named row(s) < ${FLOOR_M} m floor (coastal mis-tags; kind fixes T-152.17/.19): ` +
      namedDropped
        .sort((a, b) => a.value_m - b.value_m)
        .map((n) => `${n.name}=${n.value_m}`)
        .join(", "),
  );
}
