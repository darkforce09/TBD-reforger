// T-090.2 semantic golden gates S2–S9. AJV shape validation (S1) lives in validate.mjs and the
// enum drift gate (S10) in verify-map-object-enums.mjs — this script owns everything AJV cannot
// express: prefabId resolution, prefab-table dedup, and closed-enum *coverage* (>=1 golden example
// per class enum member per kind). Bundle isolation: each catalog bundle's instances resolve ONLY
// against its own prefabs[]; map-object-instances-sample.json pairs with map-object-prefabs-sample.json.
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import Ajv from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const readJSON = (p) => JSON.parse(readFileSync(p, "utf8"));
const moPath = (...p) => join(root, "golden", "map-objects", ...p);

const enums = readJSON(join(root, "schema", "map-object-enums.schema.json")).$defs;
const enumSet = (name) => new Set(enums[name].enum);

// Instance/prefab kind -> its closed class enum (same mapping as verify-map-object-enums.mjs).
const classEnumForKind = {
  building: "buildingClass",
  road: "roadClass",
  tree: "speciesClass",
  vegetation: "speciesClass",
  rock: "rockClass",
  prop: "propClass",
  utility: "utilityClass",
  water: "waterClass",
};
const instanceKinds = Object.keys(classEnumForKind);

// S9 expected classes per kind. speciesClass is a tree∪vegetation union in the enums file, so the
// per-kind subsets are pinned here from the spec taxonomy tables (t090_2_map_object_taxonomy.md);
// every other kind expects its full closed enum.
const expectedClassesForKind = {
  tree: ["conifer", "deciduous", "palm", "dead", "unknown"],
  vegetation: ["bush", "grass", "fern", "dead", "unknown"],
  building: enums.buildingClass.enum,
  road: enums.roadClass.enum,
  rock: enums.rockClass.enum,
  prop: enums.propClass.enum,
  utility: enums.utilityClass.enum,
  water: enums.waterClass.enum,
};

const prefabsSample = readJSON(moPath("map-object-prefabs-sample.json"));
const instancesSample = readJSON(moPath("map-object-instances-sample.json"));
const regionsSample = readJSON(moPath("map-object-regions-everon-sample.json"));
const roadsSample = readJSON(moPath("map-object-roads-sample.json"));
const resolvedSample = readJSON(moPath("map-object-resolved-sample.json"));
const catalogBundles = [
  { label: "map-object-catalog-everon-sample.json", data: readJSON(moPath("map-object-catalog-everon-sample.json")) },
  { label: "phased/P1-buildings.json", data: readJSON(moPath("phased", "P1-buildings.json")) },
];

// Prefab tables paired with their instance arrays (bundle isolation).
const tables = [
  { label: "prefabs-sample", prefabs: prefabsSample, instances: instancesSample, roadSegments: roadsSample.roadSegments },
  ...catalogBundles.map((b) => ({
    label: b.label,
    prefabs: b.data.prefabs ?? [],
    instances: b.data.instances ?? [],
    roadSegments: b.data.roadSegments ?? [],
  })),
];

const gates = [];
const gate = (id, label, errs) => gates.push({ id, label, errs });
const instId = (row) => (Array.isArray(row) ? row[0] : row.id);
const instPrefabId = (row) => (Array.isArray(row) ? row[1] : row.prefabId);

// S2 — every prefab row has kind + class; every instance resolves to a prefab carrying both.
{
  const errs = [];
  for (const t of tables) {
    const byId = new Map(t.prefabs.map((p) => [p.prefabId, p]));
    for (const p of t.prefabs) {
      if (!p.kind) errs.push(`${t.label}: prefab ${p.prefabId} missing kind`);
      if (!p.class) errs.push(`${t.label}: prefab ${p.prefabId} missing class`);
    }
    for (const row of t.instances) {
      const p = byId.get(instPrefabId(row));
      if (p && (!p.kind || !p.class)) errs.push(`${t.label}: instance ${instId(row)} resolves to prefab without kind/class`);
    }
  }
  gate("S2", "every prefab + instance row has resolvable kind + class", errs);
}

// S3 — >=1 prefab example per instance kind in the main golden table.
{
  const have = new Set(prefabsSample.map((p) => p.kind));
  const errs = instanceKinds.filter((k) => !have.has(k)).map((k) => `prefabs-sample: no prefab example for kind '${k}'`);
  gate("S3", "≥1 prefab example per instance kind", errs);
}

// S4 — every road segment uses a valid roadClass; every kind=road prefab class ∈ roadClass.
{
  const errs = [];
  const roadEnum = enumSet("roadClass");
  for (const t of tables) {
    for (const seg of t.roadSegments) {
      if (!roadEnum.has(seg.roadClass)) errs.push(`${t.label}: segment ${seg.id} roadClass '${seg.roadClass}' invalid`);
    }
    for (const p of t.prefabs.filter((p) => p.kind === "road")) {
      if (!roadEnum.has(p.class)) errs.push(`${t.label}: road prefab ${p.prefabId} class '${p.class}' not a roadClass`);
    }
  }
  gate("S4", "road segments + road prefabs use valid roadClass", errs);
}

// S5 — prefab-table dedup: unique prefabId + unique resourceName; instances never carry type fields.
{
  const errs = [];
  for (const t of tables) {
    const seenId = new Set();
    const seenRes = new Set();
    for (const p of t.prefabs) {
      if (seenId.has(p.prefabId)) errs.push(`${t.label}: duplicate prefabId ${p.prefabId}`);
      if (seenRes.has(p.resourceName)) errs.push(`${t.label}: duplicate resourceName ${p.resourceName}`);
      seenId.add(p.prefabId);
      seenRes.add(p.resourceName);
    }
    for (const row of t.instances) {
      if (Array.isArray(row)) continue; // compact tuple carries no type fields by construction
      for (const key of ["resourceName", "kind", "class", "bounds"]) {
        if (key in row) errs.push(`${t.label}: instance ${row.id} duplicates prefab field '${key}'`);
      }
    }
  }
  gate("S5", "prefab dedup — unique prefabId/resourceName; instances carry no type fields", errs);
}

// S6 — every instance prefabId resolves in its paired prefab table (bundle isolation).
{
  const errs = [];
  for (const t of tables) {
    const ids = new Set(t.prefabs.map((p) => p.prefabId));
    for (const row of t.instances) {
      const pid = instPrefabId(row);
      if (!ids.has(pid)) errs.push(`${t.label}: instance ${instId(row)} prefabId ${pid} does not resolve`);
    }
  }
  gate("S6", "every instance prefabId resolves in its own prefab table", errs);
}

// S7 — mandatory AI/gameplay/spatial fields present on every prefab (presence, not truthiness —
// heightM 0 is legal for roads).
{
  const errs = [];
  for (const t of tables) {
    for (const p of t.prefabs) {
      if (!p.ai?.summary) errs.push(`${t.label}: prefab ${p.prefabId} missing ai.summary`);
      if (!p.ai?.taxonomyPath) errs.push(`${t.label}: prefab ${p.prefabId} missing ai.taxonomyPath`);
      if (p.gameplay?.cover?.type === undefined) errs.push(`${t.label}: prefab ${p.prefabId} missing gameplay.cover.type`);
      if (p.spatial?.heightM === undefined) errs.push(`${t.label}: prefab ${p.prefabId} missing spatial.heightM`);
    }
  }
  gate("S7", "every prefab has ai.summary + ai.taxonomyPath + gameplay.cover.type + spatial.heightM", errs);
}

// S8 — materialized resolved samples validate map-object-resolved.schema.json.
{
  const ajv = new Ajv({ allErrors: true, strict: true, allowUnionTypes: true });
  addFormats(ajv);
  ajv.addSchema(
    [
      "map-object-enums.schema.json",
      "map-object-prefab.schema.json",
      "map-object-resolved.schema.json",
    ].map((f) => readJSON(join(root, "schema", f))),
  );
  const validateResolved = ajv.getSchema("https://schema.tbdevent.eu/map-object-resolved/v1.json");
  const errs = [];
  for (const [i, row] of resolvedSample.entries()) {
    if (!validateResolved(row)) {
      for (const err of validateResolved.errors ?? []) {
        errs.push(`resolved[${i}] ${row.id}: ${err.instancePath || "/"} ${err.message}`);
      }
    }
  }
  gate("S8", "resolved samples validate map-object-resolved.schema.json", errs);
}

// S9 — closed-enum coverage: every expected class per kind has >=1 golden prefab; every roadClass
// has >=1 golden segment; every regionKind has >=1 golden region.
{
  const errs = [];
  const byKind = new Map();
  for (const p of prefabsSample) {
    if (!byKind.has(p.kind)) byKind.set(p.kind, new Set());
    byKind.get(p.kind).add(p.class);
  }
  for (const [kind, expected] of Object.entries(expectedClassesForKind)) {
    const have = byKind.get(kind) ?? new Set();
    for (const cls of expected) {
      if (!have.has(cls)) errs.push(`prefabs-sample: missing enum example ${kind}/${cls}`);
    }
  }
  const segClasses = new Set(roadsSample.roadSegments.map((s) => s.roadClass));
  for (const cls of enums.roadClass.enum) {
    if (!segClasses.has(cls)) errs.push(`roads-sample: missing segment example roadClass '${cls}'`);
  }
  const regionKinds = new Set(regionsSample.map((r) => r.kind));
  for (const kind of enums.regionKind.enum) {
    if (!regionKinds.has(kind)) errs.push(`regions-sample: missing region example kind '${kind}'`);
  }
  gate("S9", "full closed-enum coverage (prefab classes + road segments + region kinds)", errs);
}

let failures = 0;
for (const g of gates) {
  if (g.errs.length === 0) {
    console.log(`  PASS  ${g.id} — ${g.label}`);
  } else {
    failures += g.errs.length;
    console.log(`  FAIL  ${g.id} — ${g.label}`);
    for (const e of g.errs) console.log(`        ${e}`);
  }
}

if (failures) {
  console.error(`\nverify-map-object-golden: FAIL (${failures} error(s))`);
  process.exit(1);
}
console.log(
  `\nverify-map-object-golden: OK (S2–S9; ${prefabsSample.length} prefabs, ${instancesSample.length} instances, ${roadsSample.roadSegments.length} segments, ${regionsSample.length} regions, ${resolvedSample.length} resolved; zero missing enum examples)`,
);
