// Compatibility test: every golden mission must always validate against the
// frozen Mission JSON schema, and the example registry against the registry schema.
// Run in CI for the web validator; run manually pre-release for the Enfusion loader.
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import Ajv from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");

const readJSON = (p) => JSON.parse(readFileSync(p, "utf8"));

const ajv = new Ajv({ allErrors: true, strict: true, strictTuples: false, allowUnionTypes: true });
addFormats(ajv);

const missionSchema = readJSON(join(root, "schema", "mission.schema.json"));
const registrySchema = readJSON(join(root, "schema", "registry.schema.json"));
const registryItemsSchema = readJSON(join(root, "schema", "registry-items.schema.json"));
const registryCompatSchema = readJSON(join(root, "schema", "registry-compat.schema.json"));
const loadoutExportSchema = readJSON(join(root, "schema", "loadout-export.schema.json"));
const bridgeSchema = readJSON(join(root, "bridge", "bridge-messages.schema.json"));
const terrainManifestSchema = readJSON(join(root, "schema", "terrain-manifest.schema.json"));
const terrainAnchorsSchema = readJSON(join(root, "schema", "terrain-anchors.schema.json"));
const missionEditorPayloadSchema = readJSON(join(root, "schema", "mission-editor-payload.schema.json"));
const locationsSchema = readJSON(join(root, "schema", "locations.schema.json"));
const heightLabelsSchema = readJSON(join(root, "schema", "height-labels.schema.json"));

const repoRoot = resolve(root, "..", "..");

const validateMission = ajv.compile(missionSchema);
const validateRegistry = ajv.compile(registrySchema);
const validateRegistryItems = ajv.compile(registryItemsSchema);
const validateRegistryCompat = ajv.compile(registryCompatSchema);
const validateLoadoutExport = ajv.compile(loadoutExportSchema);
const validateBridge = ajv.compile(bridgeSchema);
const validateTerrainManifest = ajv.compile(terrainManifestSchema);
const validateTerrainAnchors = ajv.compile(terrainAnchorsSchema);
const validateMissionEditorPayload = ajv.compile(missionEditorPayloadSchema);
const validateLocations = ajv.compile(locationsSchema);
const validateHeightLabels = ajv.compile(heightLabelsSchema);

// T-090.2 map-object contracts. Register every schema by $id first so the cross-file $refs
// (enums single-source + catalog/resolved bundles) resolve, then pull compiled validators.
const mapObjectSchemas = [
  "map-object-enums.schema.json",
  "map-object-prefab.schema.json",
  "map-object-instance.schema.json",
  "map-object-region.schema.json",
  "map-object-roads.schema.json",
  "map-object-catalog.schema.json",
  "map-object-resolved.schema.json",
  "map-object-type-inventory.schema.json",
  "terrain-registry.schema.json",
].map((f) => readJSON(join(root, "schema", f)));
ajv.addSchema(mapObjectSchemas);
const byId = (name) => ajv.getSchema(`https://schema.tbdevent.eu/${name}/v1.json`);
const validateMapPrefab = byId("map-object-prefab");
const validateMapInstance = byId("map-object-instance");
const validateMapRegion = byId("map-object-region");
const validateMapRoads = byId("map-object-roads");
const validateMapCatalog = byId("map-object-catalog");
const validateMapResolved = byId("map-object-resolved");
const validateMapTypeInventory = byId("map-object-type-inventory");
const validateTerrainRegistry = byId("terrain-registry");

let failures = 0;

const check = (label, validate, data) => {
  if (validate(data)) {
    console.log(`  PASS  ${label}`);
  } else {
    failures += 1;
    console.log(`  FAIL  ${label}`);
    for (const err of validate.errors ?? []) {
      console.log(`        ${err.instancePath || "/"} ${err.message}`);
    }
  }
};

console.log("Golden missions:");
const missionsDir = join(root, "golden-missions");
for (const file of readdirSync(missionsDir).filter((f) => f.endsWith(".json"))) {
  check(file, validateMission, readJSON(join(missionsDir, file)));
}

console.log("Registry:");
check("registry.example.json", validateRegistry, readJSON(join(root, "registry", "registry.example.json")));
check("registry.vanilla-poc.json", validateRegistry, readJSON(join(root, "registry", "registry.vanilla-poc.json")));

console.log("Registry items:");
check("registry-items.sample.json", validateRegistryItems, readJSON(join(root, "registry", "registry-items.sample.json")));
check("registry-items.workbench.json", validateRegistryItems, readJSON(join(root, "registry", "registry-items.workbench.json")));

// T-150: compat edges must validate AND reference only resource_names present in the paired
// items envelope (edges are derived from the item map in the exporter, so a dangling endpoint
// is an exporter bug and would break registry_compat FK ingest in T-068.9).
const checkEdgeRefs = (label, itemsJson, compatJson) => {
  const known = new Set(itemsJson.items.map((it) => it.resource_name));
  const dangling = [];
  for (const edge of compatJson.edges) {
    if (!known.has(edge.from_node)) dangling.push(`${edge.edge_type} from_node ${edge.from_node}`);
    if (!known.has(edge.to_node)) dangling.push(`${edge.edge_type} to_node ${edge.to_node}`);
  }
  if (dangling.length === 0) {
    console.log(`  PASS  ${label} (referential integrity, ${compatJson.edges.length} edges)`);
  } else {
    failures += 1;
    console.log(`  FAIL  ${label} (referential integrity)`);
    for (const miss of dangling.slice(0, 10)) console.log(`        dangling ${miss}`);
    if (dangling.length > 10) console.log(`        ... ${dangling.length - 10} more`);
  }
};

// T-068.10.2: per-item addon provenance must reference the envelope's addons[] scan set
// (vanilla-ness derives from addons[].vanilla — a dangling addon id breaks that join).
// Items without an addon field are legal (v2 envelopes); the v3 exporter always writes it.
const checkAddonRefs = (label, itemsJson) => {
  const known = new Set((itemsJson.addons ?? []).map((a) => a.name));
  const bad = [];
  let withAddon = 0;
  for (const it of itemsJson.items) {
    if (it.addon === undefined) continue;
    withAddon += 1;
    if (!known.has(it.addon)) bad.push(`${it.resource_name} addon ${it.addon}`);
  }
  if (bad.length === 0) {
    console.log(`  PASS  ${label} (addon provenance, ${withAddon}/${itemsJson.items.length} items carry addon)`);
  } else {
    failures += 1;
    console.log(`  FAIL  ${label} (addon provenance)`);
    for (const miss of bad.slice(0, 10)) console.log(`        dangling ${miss}`);
    if (bad.length > 10) console.log(`        ... ${bad.length - 10} more`);
  }
};
checkAddonRefs("registry-items.sample.json", readJSON(join(root, "registry", "registry-items.sample.json")));
checkAddonRefs("registry-items.workbench.json", readJSON(join(root, "registry", "registry-items.workbench.json")));

// T-068.10.5: variant_of must reference an item in the same envelope (a dangling parent
// would break the picker back-link and the T-068.12 variant-equip resolution).
const checkVariantRefs = (label, itemsJson) => {
  const known = new Set(itemsJson.items.map((it) => it.resource_name));
  const bad = [];
  let variants = 0;
  for (const it of itemsJson.items) {
    if (it.variant_of === undefined) continue;
    variants += 1;
    if (!known.has(it.variant_of)) bad.push(`${it.resource_name} variant_of ${it.variant_of}`);
    if (it.variant_of === it.resource_name) bad.push(`${it.resource_name} is its own variant`);
  }
  if (bad.length === 0) {
    console.log(`  PASS  ${label} (variant_of integrity, ${variants} variants)`);
  } else {
    failures += 1;
    console.log(`  FAIL  ${label} (variant_of integrity)`);
    for (const miss of bad.slice(0, 10)) console.log(`        ${miss}`);
  }
};
checkVariantRefs("registry-items.sample.json", readJSON(join(root, "registry", "registry-items.sample.json")));
checkVariantRefs("registry-items.workbench.json", readJSON(join(root, "registry", "registry-items.workbench.json")));

console.log("Registry compat:");
{
  const itemsSample = readJSON(join(root, "registry", "registry-items.sample.json"));
  const compatSample = readJSON(join(root, "registry", "registry-compat.sample.json"));
  check("registry-compat.sample.json", validateRegistryCompat, compatSample);
  checkEdgeRefs("registry-compat.sample.json vs registry-items.sample.json", itemsSample, compatSample);

  const itemsWb = readJSON(join(root, "registry", "registry-items.workbench.json"));
  const compatWb = readJSON(join(root, "registry", "registry-compat.workbench.json"));
  check("registry-compat.workbench.json", validateRegistryCompat, compatWb);
  checkEdgeRefs("registry-compat.workbench.json vs registry-items.workbench.json", itemsWb, compatWb);
}

console.log("Faction library:");
{
  const factionSchema = readJSON(join(root, "schema", "faction-library.schema.json"));
  const validateFaction = ajv.compile(factionSchema);
  check("faction-library.sample.json", validateFaction, readJSON(join(root, "registry", "faction-library.sample.json")));
}

console.log("Loadout export:");
check("loadout-export.sample.json", validateLoadoutExport, readJSON(join(root, "registry", "loadout-export.sample.json")));
check("loadout-export.v2.sample.json", validateLoadoutExport, readJSON(join(root, "registry", "loadout-export.v2.sample.json")));

console.log("Mission editor payload:");
check(
  "mission-editor-payload.sample.json",
  validateMissionEditorPayload,
  readJSON(join(root, "registry", "mission-editor-payload.sample.json")),
);

console.log("Bridge message samples:");
const samplesDir = join(root, "bridge", "samples");
for (const file of readdirSync(samplesDir).filter((f) => f.endsWith(".json"))) {
  check(file, validateBridge, readJSON(join(samplesDir, file)));
}

console.log("Terrain manifest:");
check(
  "everon/manifest.json",
  validateTerrainManifest,
  readJSON(join(repoRoot, "packages", "map-assets", "everon", "manifest.json")),
);

console.log("Locations (T-152.6):");
check(
  "locations-everon-sample.json",
  validateLocations,
  readJSON(join(root, "golden", "locations-everon-sample.json")),
);
const everonLocPath = join(repoRoot, "packages", "map-assets", "everon", "locations.json");
if (existsSync(everonLocPath)) {
  check("map-assets/everon/locations.json", validateLocations, readJSON(everonLocPath));
}

console.log("Height labels (T-152.16):");
const everonHeightLabelsPath = join(repoRoot, "packages", "map-assets", "everon", "height-labels.json");
if (existsSync(everonHeightLabelsPath)) {
  check("map-assets/everon/height-labels.json", validateHeightLabels, readJSON(everonHeightLabelsPath));
}

console.log("Terrain anchors example:");
check(
  "everon/anchors/verification.example.json",
  validateTerrainAnchors,
  readJSON(join(repoRoot, "packages", "map-assets", "everon", "anchors", "verification.example.json")),
);

// ENF-4 (CODING_STANDARDS §5): every Enfusion Backend DTO that carries an @contract tag has a
// golden fixture that validates against its mission-schema pointer. Data-driven: filename ->
// pointer (root -> the whole mission, else #/$defs/<name>). Drop a new <def>.sample.json to enrol.
console.log("Enfusion DTO fixtures (ENF-4):");
const enfusionDir = join(root, "enfusion");
for (const file of readdirSync(enfusionDir).filter((f) => f.endsWith(".sample.json"))) {
  const base = file.replace(/\.sample\.json$/, "");
  const validate = base === "root" ? validateMission : ajv.getSchema(missionSchema.$id + `#/$defs/${base}`);
  if (!validate) {
    failures += 1;
    console.log(`  FAIL  ${file} (no schema for #/$defs/${base})`);
    continue;
  }
  check(file, validate, readJSON(join(enfusionDir, file)));
}

// T-090.2 map-object goldens (golden/map-objects/*). Arrays are validated per row.
const moDir = join(root, "golden", "map-objects");
const moPath = (...p) => join(moDir, ...p);

console.log("Map object prefabs (S9 — one row per buildingClass):");
for (const [i, row] of readJSON(moPath("map-object-prefabs-sample.json")).entries()) {
  check(`prefab[${i}] ${row.kind}/${row.class}`, validateMapPrefab, row);
}

console.log("Map object instances:");
for (const [i, row] of readJSON(moPath("map-object-instances-sample.json")).entries()) {
  check(`instance[${i}]`, validateMapInstance, row);
}

console.log("Map object chunk sample (T-090.3.1 — all-number 5-tuples):");
for (const [i, row] of readJSON(moPath("map-object-chunk-sample.json")).chunk.instances.entries()) {
  check(`chunk-instance[${i}]`, validateMapInstance, row);
}

console.log("Map object regions (forest / field):");
for (const [i, row] of readJSON(moPath("map-object-regions-everon-sample.json")).entries()) {
  check(`region[${i}] ${row.kind}`, validateMapRegion, row);
}

console.log("Map object roads:");
check("map-object-roads-sample.json", validateMapRoads, readJSON(moPath("map-object-roads-sample.json")));

console.log("Map object catalog bundle (validation-only, N12):");
check("map-object-catalog-everon-sample.json", validateMapCatalog, readJSON(moPath("map-object-catalog-everon-sample.json")));
check("phased/P1-buildings.json", validateMapCatalog, readJSON(moPath("phased", "P1-buildings.json")));

console.log("ResolvedWorldObject (Eden AI + T-090.7):");
for (const [i, row] of readJSON(moPath("map-object-resolved-sample.json")).entries()) {
  check(`resolved[${i}] ${row.kind}`, validateMapResolved, row);
}

console.log("Terrain registry:");
check("golden terrain-registry.sample.json", validateTerrainRegistry, readJSON(moPath("terrain-registry.sample.json")));
check(
  "map-assets/terrain-registry.json",
  validateTerrainRegistry,
  readJSON(join(repoRoot, "packages", "map-assets", "terrain-registry.json")),
);

console.log("Dual + legacy terrain manifests (T-090.1/.1.1):");
check("everon-dual-tiles", validateTerrainManifest, readJSON(moPath("terrain-manifest-everon-dual-tiles.json")));
check("everon-legacy-tiles", validateTerrainManifest, readJSON(moPath("terrain-manifest-everon-legacy-tiles.json")));
check(
  "everon-unified-satellite",
  validateTerrainManifest,
  readJSON(moPath("terrain-manifest-everon-unified-satellite.json")),
);

console.log("Map object type inventory (exact counts — pending until export):");
check(
  "type-inventory-pending-everon.json",
  validateMapTypeInventory,
  readJSON(moPath("type-inventory-pending-everon.json")),
);
check(
  "map-assets/everon/objects/type-inventory.json",
  validateMapTypeInventory,
  readJSON(join(repoRoot, "packages", "map-assets", "everon", "objects", "type-inventory.json")),
);

if (failures > 0) {
  console.error(`\n${failures} validation failure(s).`);
  process.exit(1);
}
console.log("\nAll contracts valid.");
