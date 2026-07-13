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
  console.error("verify-town-labels: wasm pkg missing — run make wasm");
  process.exit(1);
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
