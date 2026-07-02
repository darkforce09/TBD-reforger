// Compatibility test: every golden mission must always validate against the
// frozen Mission JSON schema, and the example registry against the registry schema.
// Run in CI for the web validator; run manually pre-release for the Enfusion loader.
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import Ajv from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");

const readJSON = (p) => JSON.parse(readFileSync(p, "utf8"));

const ajv = new Ajv({ allErrors: true, strict: true, allowUnionTypes: true });
addFormats(ajv);

const missionSchema = readJSON(join(root, "schema", "mission.schema.json"));
const registrySchema = readJSON(join(root, "schema", "registry.schema.json"));
const registryItemsSchema = readJSON(join(root, "schema", "registry-items.schema.json"));
const loadoutExportSchema = readJSON(join(root, "schema", "loadout-export.schema.json"));
const bridgeSchema = readJSON(join(root, "bridge", "bridge-messages.schema.json"));
const terrainManifestSchema = readJSON(join(root, "schema", "terrain-manifest.schema.json"));
const terrainAnchorsSchema = readJSON(join(root, "schema", "terrain-anchors.schema.json"));
const missionEditorPayloadSchema = readJSON(join(root, "schema", "mission-editor-payload.schema.json"));

const repoRoot = resolve(root, "..", "..");

const validateMission = ajv.compile(missionSchema);
const validateRegistry = ajv.compile(registrySchema);
const validateRegistryItems = ajv.compile(registryItemsSchema);
const validateLoadoutExport = ajv.compile(loadoutExportSchema);
const validateBridge = ajv.compile(bridgeSchema);
const validateTerrainManifest = ajv.compile(terrainManifestSchema);
const validateTerrainAnchors = ajv.compile(terrainAnchorsSchema);
const validateMissionEditorPayload = ajv.compile(missionEditorPayloadSchema);

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

console.log("Loadout export:");
check("loadout-export.sample.json", validateLoadoutExport, readJSON(join(root, "registry", "loadout-export.sample.json")));

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
