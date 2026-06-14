// Validate a single Mission JSON file or stdin. Used by the website upload handler.
// Usage: node validate-file.mjs path/to/mission.json
//        cat mission.json | node validate-file.mjs -
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import Ajv from "ajv/dist/2020.js";
import addFormats from "ajv-formats";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const target = process.argv[2];
if (!target) {
  console.error("usage: validate-file.mjs <path|->");
  process.exit(2);
}

const raw =
  target === "-"
    ? readFileSync(0, "utf8")
    : readFileSync(resolve(target), "utf8");

let data;
try {
  data = JSON.parse(raw);
} catch {
  console.error("invalid JSON");
  process.exit(1);
}

const ajv = new Ajv({ allErrors: true, strict: true });
addFormats(ajv);
const schema = JSON.parse(
  readFileSync(join(root, "schema", "mission.schema.json"), "utf8")
);
const validate = ajv.compile(schema);

if (!validate(data)) {
  for (const err of validate.errors ?? []) {
    console.error(`${err.instancePath || "/"} ${err.message}`);
  }
  process.exit(1);
}

if (data.schemaVersion === "1.1") {
  let expected = 0;
  for (const [factionKey, factionOrbat] of Object.entries(data.orbat || {})) {
    for (const group of factionOrbat.groups || []) {
      for (const role of group.roles || []) {
        expected += role.count;
      }
    }
  }
  const actual = (data.slots || []).length;
  if (actual !== expected) {
    console.error(
      `/slots ORBAT instance count mismatch: orbat expects ${expected}, slots has ${actual}`
    );
    process.exit(1);
  }
  const ids = new Set();
  for (const slot of data.slots) {
    if (ids.has(slot.id)) {
      console.error(`/slots duplicate slot id '${slot.id}'`);
      process.exit(1);
    }
    ids.add(slot.id);
  }
}

console.log("ok");
