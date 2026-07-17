#!/usr/bin/env node
// T-152.8 — mathematical gates G2–G5 for town labels @ committed locations.json.
import { readFileSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { REQUIRED_EVERON_TOWNS } from "./lib/locations-export.mjs";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const terrain = process.env.TERRAIN ?? "everon";
const zoomMatch = process.argv.find((a) => a.startsWith("--zoom"));
const deckZoom = zoomMatch
  ? Number(zoomMatch.includes("=") ? zoomMatch.split("=")[1] : process.argv[process.argv.indexOf(zoomMatch) + 1])
  : -2;

const locPath = join(repoRoot, "packages", "map-assets", terrain, "locations.json");
const wasmPkg = join(repoRoot, "apps", "website", "frontend", "src", "wasm", "pkg", "map_engine_wasm.js");

if (!existsSync(locPath)) {
  console.error(`verify-town-labels: missing ${locPath} (run T-152.6)`);
  process.exit(1);
}
if (!existsSync(wasmPkg)) {
  // T-164: the wasm-bindgen pkg died with the React app (T-159.29.3); the label-declutter math is
  // pinned by map-engine-core's cargo tests (`make wasm-ci`). Skip, never fail.
  console.log("  SKIP  verify-town-labels — retired with the React wasm pkg (make wasm-ci owns the math)");
  process.exit(0);
}

const sourceRaw = JSON.parse(readFileSync(locPath, "utf8"));
const wasm = await import(wasmPkg);

let failures = 0;
const pass = (msg) => console.log(`  PASS  ${msg}`);
const fail = (msg) => {
  failures++;
  console.log(`  FAIL  ${msg}`);
};

console.log(`verify-town-labels (${terrain} @ z=${deckZoom}):`);

const drawn = JSON.parse(wasm.declutter_town_labels_json(JSON.stringify(sourceRaw), deckZoom));

const errJson = wasm.verify_town_labels_json(
  JSON.stringify(sourceRaw),
  JSON.stringify(drawn),
  deckZoom,
  JSON.stringify(REQUIRED_EVERON_TOWNS),
);
const errs = JSON.parse(errJson);
if (errs.length === 0) {
  pass(`G2 REQUIRED_EVERON_TOWNS (${REQUIRED_EVERON_TOWNS.length}) ⊆ drawn @ z=${deckZoom}`);
  pass("G3 declutter invariant (A3 predicate)");
  pass("G4 name provenance = locations.json[id]");
} else {
  for (const e of errs) fail(e);
}

if (wasm.town_declutter_invariant_holds_json(JSON.stringify(sourceRaw), deckZoom)) {
  pass("G3 wasm oracle (redundant check)");
} else {
  fail("G3 town_declutter_invariant_holds_json");
}

const emptyDrawn = JSON.parse(wasm.declutter_town_labels_json("[]", deckZoom));
if (emptyDrawn.length === 0) {
  pass("G5 empty source → |drawn|=0");
} else {
  fail(`G5 empty source drew ${emptyDrawn.length}`);
}

// G1 (T-152.17) kind hygiene: the town lane draws settlements only.
const ALLOWED_KINDS = new Set(["town", "village", "airport", "locality"]);
const EXCLUDED_KINDS = new Set(["peak", "hill", "natural"]);
const kindOf = (l) => l.kind ?? "town";
const unknownKind = drawn.filter((l) => !ALLOWED_KINDS.has(kindOf(l)));
const excludedDrawn = drawn.filter((l) => EXCLUDED_KINDS.has(kindOf(l)));
if (unknownKind.length === 0 && excludedDrawn.length === 0) {
  pass(`G1 kind hygiene: ${drawn.length} drawn ⊆ {town,village,airport,locality}; 0 peak/hill/natural @ z=${deckZoom}`);
} else {
  const offenders = [...excludedDrawn, ...unknownKind].slice(0, 6).map((l) => `${l.name}:${kindOf(l)}`);
  fail(`G1 kind hygiene: ${excludedDrawn.length} excluded + ${unknownKind.length} unknown kind drawn [${offenders.join(", ")}]`);
}

// G4 (T-152.17) fade band: alpha 1.0 → 0.5 → 0.0 over z ∈ [2.0, 3.0].
const approx = (a, b) => Math.abs(a - b) < 1e-6;
const fa = (z) => wasm.town_label_fade_alpha(z);
if (approx(fa(2.0), 1.0) && approx(fa(2.5), 0.5) && approx(fa(3.0), 0.0)) {
  pass(`G4 fade α: 2.0→${fa(2.0)} 2.5→${fa(2.5)} 3.0→${fa(3.0)}`);
} else {
  fail(`G4 fade endpoints wrong: α(2.0)=${fa(2.0)} α(2.5)=${fa(2.5)} α(3.0)=${fa(3.0)}`);
}

// G4 band edges: nothing drawn above the fade ceiling or below the widened floor.
const aboveCeil = JSON.parse(wasm.declutter_town_labels_json(JSON.stringify(sourceRaw), 3.1));
const belowFloor = JSON.parse(wasm.declutter_town_labels_json(JSON.stringify(sourceRaw), -4.6));
if (aboveCeil.length === 0 && belowFloor.length === 0) {
  pass("G4 band edges: |drawn|=0 @ z=3.1 (above ceiling) and z=−4.6 (below floor)");
} else {
  fail(`G4 band edges: ${aboveCeil.length} drawn @ z=3.1, ${belowFloor.length} drawn @ z=−4.6`);
}

const bytes = wasm.pack_town_label_bytes(JSON.stringify(drawn), deckZoom);
if (drawn.length === 0 && bytes.length === 0) {
  pass("pack bytes empty when no labels");
} else if (bytes.length > 0 && bytes.length % 20 === 0) {
  pass(`pack ${bytes.length / 20} glyph instances (${bytes.length} B)`);
} else {
  fail(`pack bytes invalid len=${bytes.length}`);
}

if (failures) {
  console.error(`\nverify-town-labels: FAIL (${failures})`);
  process.exit(1);
}
console.log("\nverify-town-labels: OK");
