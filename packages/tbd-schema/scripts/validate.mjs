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

const ajv = new Ajv({ allErrors: true, strict: true });
addFormats(ajv);

const missionSchema = readJSON(join(root, "schema", "mission.schema.json"));
const registrySchema = readJSON(join(root, "schema", "registry.schema.json"));
const registryItemsSchema = readJSON(join(root, "schema", "registry-items.schema.json"));
const loadoutExportSchema = readJSON(join(root, "schema", "loadout-export.schema.json"));
const bridgeSchema = readJSON(join(root, "bridge", "bridge-messages.schema.json"));
const terrainManifestSchema = readJSON(join(root, "schema", "terrain-manifest.schema.json"));
const terrainAnchorsSchema = readJSON(join(root, "schema", "terrain-anchors.schema.json"));

const repoRoot = resolve(root, "..", "..");

const validateMission = ajv.compile(missionSchema);
const validateRegistry = ajv.compile(registrySchema);
const validateRegistryItems = ajv.compile(registryItemsSchema);
const validateLoadoutExport = ajv.compile(loadoutExportSchema);
const validateBridge = ajv.compile(bridgeSchema);
const validateTerrainManifest = ajv.compile(terrainManifestSchema);
const validateTerrainAnchors = ajv.compile(terrainAnchorsSchema);

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

if (failures > 0) {
  console.error(`\n${failures} validation failure(s).`);
  process.exit(1);
}
console.log("\nAll contracts valid.");
