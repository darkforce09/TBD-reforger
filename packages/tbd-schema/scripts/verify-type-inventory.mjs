#!/usr/bin/env node
// T-090.2 — validate type-inventory.json mathematical invariants (I1–I7; t090_world_object_type_inventory.md).
// Runs on every `make schema-validate`. When censusStatus=complete, every count is exact integer equality.
// T-090.3.1 adds: I3 (every class key ∈ its closed enum), I5/I7 (manifest.objects prefabCount /
// instanceCount cross-check against the committed sibling manifest.json).
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
const enums = readJSON(join(schemaRoot, "schema", "map-object-enums.schema.json")).$defs;

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

const checkInventory = (label, inv, manifest = null) => {
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

  // I3 — every per-class key is a member of its closed enum (T-090.3.1)
  for (const [bucket, enumName] of [
    ["byBuildingClass", "buildingClass"],
    ["byRoadClass", "roadClass"],
    ["bySpeciesClass", "speciesClass"],
  ]) {
    const allowed = new Set(enums[enumName].enum);
    for (const cls of Object.keys(inv[bucket] ?? {})) {
      if (!allowed.has(cls)) fail(`${label}: I3 ${bucket} key '${cls}' not in ${enumName} enum`);
    }
  }

  // I4 — needsReview.prefabTypes = 0 before ship (only when complete)
  if (inv.censusStatus === "complete" && inv.needsReview?.prefabTypes !== 0) {
    fail(`${label}: I4 complete census requires needsReview.prefabTypes = 0 (got ${inv.needsReview?.prefabTypes})`);
  }

  // I5 / I7 — manifest.objects cross-check (T-090.3.1): once the sibling manifest carries an
  // objects export block, its counts must equal the inventory levels exactly.
  if (manifest?.objects && typeof manifest.objects.prefabCount === "number") {
    if (manifest.objects.prefabCount !== inv.levels.uniquePrefabs) {
      fail(`${label}: I5 manifest.objects.prefabCount ${manifest.objects.prefabCount} !== levels.uniquePrefabs ${inv.levels.uniquePrefabs}`);
    }
    if (manifest.objects.instanceCount !== inv.levels.totalInstances) {
      fail(`${label}: I7 manifest.objects.instanceCount ${manifest.objects.instanceCount} !== levels.totalInstances ${inv.levels.totalInstances}`);
    }
  }
};

// Every registry terrain's committed inventory (+ I5/I7 against its sibling manifest.json)
const registryPath = join(repoRoot, "packages", "map-assets", "terrain-registry.json");
for (const t of existsSync(registryPath) ? readJSON(registryPath).terrains : []) {
  const invPath = join(repoRoot, "packages", "map-assets", t.terrainId, "objects", "type-inventory.json");
  if (!existsSync(invPath)) continue;
  const manifestPath = join(repoRoot, "packages", "map-assets", t.manifestPath);
  const manifest = existsSync(manifestPath) ? readJSON(manifestPath) : null;
  checkInventory(`${t.terrainId}/objects/type-inventory.json`, readJSON(invPath), manifest);
}

// Golden pending fixture
const goldenPath = join(schemaRoot, "golden", "map-objects", "type-inventory-pending-everon.json");
if (existsSync(goldenPath)) {
  checkInventory("golden/type-inventory-pending-everon.json", readJSON(goldenPath));
}

// T-090.3.0 spike subregion census (opt-in; staging is gitignored so this is a no-op in CI —
// a LOCAL gate that exercises K1b when the spike has produced type-inventory-spike.json).
for (const t of ["everon", "arland", "custom"]) {
  const spikePath = join(repoRoot, "packages", "map-assets", t, "staging", "spike", "type-inventory-spike.json");
  if (existsSync(spikePath)) {
    checkInventory(`${t}/staging/spike/type-inventory-spike.json`, readJSON(spikePath));
  }
}

if (failures.length) {
  console.error(`verify-type-inventory: FAIL (${failures.length})`);
  for (const f of failures) console.error(`  ${f}`);
  process.exit(1);
}
console.log("verify-type-inventory: OK");
