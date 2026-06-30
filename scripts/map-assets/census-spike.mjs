#!/usr/bin/env node
// T-090.3.0 — K1b subregion census (exact integers) + K1/K1b drift cross-check.
//
// Scans staging/spike/raw-entities.jsonl via the SHARED classifier, writes a schema-valid `partial`
// type-inventory-spike.json (exact integer counts; I1 holds), and cross-checks that K1 (a building row
// exists) ⇔ byKind.building.instances >= 1. Full JSON-Schema conformance is exercised separately by
// verify-type-inventory.mjs (opt-in on this file) / `make schema-validate`; this script keeps the
// math invariants dep-free.
import { existsSync, statSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { classifyRawEntitiesJsonl } from "./lib/classify-prefab.mjs";
import { entryIsK1Building } from "./verify-spike-k1.mjs";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const terrain =
  process.env.TERRAIN ?? process.argv.find((a) => a.startsWith("TERRAIN="))?.split("=")[1] ?? "everon";

const ALL_KINDS = ["building", "tree", "vegetation", "rock", "prop", "utility", "water", "road"];
const stagingDir = join(repoRoot, "packages", "map-assets", terrain, "staging", "spike");
const rawPath = join(stagingDir, "raw-entities.jsonl");
const outPath = join(stagingDir, "type-inventory-spike.json");

if (!existsSync(rawPath)) {
  console.error(`census-spike: raw-entities.jsonl not found: ${rawPath} — run the export plugin + copy-world-export-profile.mjs first`);
  process.exit(1);
}

const { entries, errors } = classifyRawEntitiesJsonl(rawPath);
if (errors.length) {
  console.error(`census-spike: jsonl parse errors: ${JSON.stringify(errors.slice(0, 5))}`);
  process.exit(1);
}

// Per-kind instance + distinct-prefab counts.
const byKind = {};
for (const k of ALL_KINDS) byKind[k] = { prefabTypes: 0, instances: 0, _prefabs: new Set() };
const allPrefabs = new Set();
const buildingClasses = {}; // class -> { instances, prefabs:Set }
const unmatchedPrefabs = new Set();

for (const e of entries) {
  const rn = typeof e.row.resourceName === "string" ? e.row.resourceName : "";
  const bucket = byKind[e.kind] ?? byKind.prop;
  bucket.instances++;
  if (rn) {
    bucket._prefabs.add(rn);
    allPrefabs.add(rn);
  }
  if (!e.matched && rn) unmatchedPrefabs.add(rn);
  if (e.kind === "building") {
    const cls = e.class ?? "unknown";
    (buildingClasses[cls] ??= { instances: 0, prefabs: new Set() });
    buildingClasses[cls].instances++;
    if (rn) buildingClasses[cls].prefabs.add(rn);
  }
}

const byKindOut = {};
for (const k of ALL_KINDS) {
  byKindOut[k] = { prefabTypes: byKind[k]._prefabs.size, instances: byKind[k].instances };
  if (k === "road") byKindOut[k].segments = 0; // spike does not extract road polylines
}

const byBuildingClass = {};
for (const [cls, v] of Object.entries(buildingClasses)) {
  byBuildingClass[cls] = { prefabTypes: v.prefabs.size, instances: v.instances };
}

const totalInstances = entries.length;
const uniquePrefabs = allPrefabs.size;
// Deterministic timestamp from the source export's mtime (re-census without re-export → identical file).
const generatedAt = new Date(statSync(rawPath).mtimeMs).toISOString();

const inventory = {
  schemaVersion: "1.0.0",
  terrainId: terrain,
  censusStatus: "partial",
  generatedAt,
  importPhaseMax: "spike_subregion",
  sourceExportPath: "staging/spike/raw-entities.jsonl",
  levels: { uniquePrefabs, totalInstances },
  byKind: byKindOut,
  byBuildingClass,
  byRoadClass: {},
  bySpeciesClass: {},
  needsReview: { prefabTypes: unmatchedPrefabs.size, prefabs: [] },
};

writeFileSync(outPath, `${JSON.stringify(inventory, null, 2)}\n`);

// --- Invariant checks (dep-free) ---
const failures = [];
// I1 — Σ byKind.instances = levels.totalInstances
const kindSum = ALL_KINDS.reduce((a, k) => a + byKindOut[k].instances, 0);
if (kindSum !== totalInstances) failures.push(`I1 kind sum ${kindSum} !== totalInstances ${totalInstances}`);
// integers
for (const k of ALL_KINDS) {
  if (!Number.isInteger(byKindOut[k].instances) || !Number.isInteger(byKindOut[k].prefabTypes)) {
    failures.push(`byKind.${k} non-integer count`);
  }
}
// I2 — building class sum = byKind.building.instances (we populate byBuildingClass)
const classSum = Object.values(byBuildingClass).reduce((a, v) => a + v.instances, 0);
if (classSum !== byKindOut.building.instances) {
  failures.push(`I2 byBuildingClass sum ${classSum} !== byKind.building.instances ${byKindOut.building.instances}`);
}
// K1/K1b drift — a K1 building row exists ⇔ byKind.building.instances >= 1
const k1Pass = entries.some(entryIsK1Building);
const k1bBuilding = byKindOut.building.instances >= 1;
if (k1Pass !== k1bBuilding) {
  failures.push(`K1/K1b classify drift: verify-spike-k1=${k1Pass} but byKind.building.instances>=1=${k1bBuilding}`);
}

if (failures.length) {
  console.error(`census-spike: FAIL (${failures.length}) — wrote ${outPath}`);
  for (const f of failures) console.error(`  ${f}`);
  process.exit(1);
}

console.log(
  `census-spike: OK (K1b) — ${totalInstances} instances, ${uniquePrefabs} prefabs; ` +
    `building=${byKindOut.building.instances}, tree=${byKindOut.tree.instances}, road=${byKindOut.road.instances}, ` +
    `needsReview=${unmatchedPrefabs.size} → ${outPath}`,
);
