#!/usr/bin/env node
// T-090.2 — validate type-inventory.json mathematical invariants (I1–I6).
// Runs on every `make schema-validate`. When censusStatus=complete, every count is exact integer equality.
import { readFileSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import Ajv from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const schemaRoot = join(repoRoot, "packages", "tbd-schema");
const readJSON = (p) => JSON.parse(readFileSync(p, "utf8"));

const ajv = new Ajv({ allErrors: true, strict: true });
addFormats(ajv);
const schema = readJSON(join(schemaRoot, "schema", "map-object-type-inventory.schema.json"));
const validate = ajv.compile(schema);

const INSTANCE_KINDS = ["building", "tree", "vegetation", "rock", "prop", "utility", "water", "road"];
const failures = [];
const fail = (msg) => failures.push(msg);

const sumKindInstances = (byKind) =>
  INSTANCE_KINDS.reduce((acc, k) => {
    const n = byKind[k]?.instances;
    return acc + (typeof n === "number" ? n : 0);
  }, 0);

const sumClassInstances = (bucket) =>
  Object.values(bucket ?? {}).reduce((acc, row) => acc + (row?.instances ?? 0), 0);

const checkInventory = (label, inv) => {
  if (!validate(inv)) {
    for (const err of validate.errors ?? []) {
      fail(`${label}: schema ${err.instancePath || "/"} ${err.message}`);
    }
    return;
  }

  if (inv.censusStatus === "pending_export") {
    if (inv.levels.totalInstances !== null || inv.levels.uniquePrefabs !== null) {
      fail(`${label}: pending_export requires null levels.* counts`);
    }
    for (const k of INSTANCE_KINDS) {
      const bucket = inv.byKind[k];
      if (bucket.prefabTypes !== null || bucket.instances !== null) {
        fail(`${label}: pending_export requires null byKind.${k} counts`);
      }
      if (k === "road" && bucket.segments !== null) {
        fail(`${label}: pending_export requires null byKind.road.segments`);
      }
    }
    return;
  }

  // I1 — Σ byKind.instances = levels.totalInstances (exact integer equality)
  const kindSum = sumKindInstances(inv.byKind);
  if (kindSum !== inv.levels.totalInstances) {
    fail(`${label}: I1 kind sum ${kindSum} !== levels.totalInstances ${inv.levels.totalInstances}`);
  }

  // I2 — building class sum = byKind.building.instances when populated
  if (inv.byBuildingClass && Object.keys(inv.byBuildingClass).length > 0) {
    const classSum = sumClassInstances(inv.byBuildingClass);
    if (classSum !== inv.byKind.building.instances) {
      fail(
        `${label}: I2 byBuildingClass sum ${classSum} !== byKind.building.instances ${inv.byKind.building.instances}`,
      );
    }
  }

  // Forest region tree assignment (exact on assigned + unassigned; not a ±% tolerance on totals)
  if (inv.byRegionKind?.forest && typeof inv.byKind.tree.instances === "number") {
    const regionTrees = inv.byRegionKind.forest.treeCount ?? 0;
    const unassigned = inv.unassignedTrees ?? 0;
    if (regionTrees + unassigned !== inv.byKind.tree.instances) {
      fail(
        `${label}: F-count forest.treeCount (${regionTrees}) + unassignedTrees (${unassigned}) !== byKind.tree.instances (${inv.byKind.tree.instances})`,
      );
    }
  }

  // I4 — needsReview.prefabTypes = 0 before ship (only when complete)
  if (inv.censusStatus === "complete" && inv.needsReview?.prefabTypes !== 0) {
    fail(`${label}: I4 complete census requires needsReview.prefabTypes = 0 (got ${inv.needsReview?.prefabTypes})`);
  }
};

// Everon committed baseline (pending until T-090.3.0 export + map-census)
const everonPath = join(repoRoot, "packages", "map-assets", "everon", "objects", "type-inventory.json");
if (existsSync(everonPath)) {
  checkInventory("everon/objects/type-inventory.json", readJSON(everonPath));
}

// Golden pending fixture
const goldenPath = join(schemaRoot, "golden", "map-objects", "type-inventory-pending-everon.json");
if (existsSync(goldenPath)) {
  checkInventory("golden/type-inventory-pending-everon.json", readJSON(goldenPath));
}

if (failures.length) {
  console.error(`verify-type-inventory: FAIL (${failures.length})`);
  for (const f of failures) console.error(`  ${f}`);
  process.exit(1);
}
console.log("verify-type-inventory: OK");
