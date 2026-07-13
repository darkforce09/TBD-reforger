#!/usr/bin/env node
// T-152.6 — mathematical gates G3–G6 for committed locations.json.
import { readFileSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";
import { N_MIN, REQUIRED_EVERON_TOWNS, verifyLocationsGates } from "./lib/locations-export.mjs";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const schemaPkg = join(repoRoot, "packages", "tbd-schema");
const require = createRequire(join(schemaPkg, "package.json"));
const Ajv = require("ajv/dist/2020.js").default;
const addFormats = require("ajv-formats").default;

const terrain = process.env.TERRAIN ?? process.argv.find((a) => a.startsWith("TERRAIN="))?.split("=")[1] ?? "everon";
const locPath = join(repoRoot, "packages", "map-assets", terrain, "locations.json");

if (!existsSync(locPath)) {
  console.error(`verify-locations: missing ${locPath}`);
  process.exit(1);
}

const locs = JSON.parse(readFileSync(locPath, "utf8"));
const ajv = new Ajv({ allErrors: true, strict: true });
addFormats(ajv);
const schema = JSON.parse(readFileSync(join(schemaPkg, "schema", "locations.schema.json"), "utf8"));
const validate = ajv.compile(schema);

let failures = 0;
const pass = (msg) => console.log(`  PASS  ${msg}`);
const fail = (msg) => {
  failures++;
  console.log(`  FAIL  ${msg}`);
};

console.log(`verify-locations (${terrain}):`);
if (validate(locs)) pass("G2 schema valid");
else {
  fail("G2 schema invalid");
  for (const err of validate.errors ?? []) console.log(`        ${err.instancePath || "/"} ${err.message}`);
}

const gateErrors = verifyLocationsGates(locs);
if (gateErrors.length === 0) {
  pass(`G3 count ${locs.length} ≥ N_MIN ${N_MIN}`);
  pass(`G4 REQUIRED_EVERON_TOWNS (${REQUIRED_EVERON_TOWNS.length}) covered`);
  pass("G5 row quality (name length, finite x/y)");
  pass('G6 no "Location composition" placeholder names');
} else {
  for (const e of gateErrors) fail(e);
}

if (failures) {
  console.error(`\nverify-locations: FAIL (${failures})`);
  process.exit(1);
}
console.log("\nverify-locations: OK");
