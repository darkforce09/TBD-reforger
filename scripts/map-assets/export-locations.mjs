#!/usr/bin/env node
// T-152.6 — export locations.json from staged Workbench raw-entities JSONL (Path B).
//
// Usage:
//   node scripts/map-assets/export-locations.mjs --terrain everon
//   node scripts/map-assets/export-locations.mjs --terrain everon --src /path/raw-entities.jsonl
//   node scripts/map-assets/export-locations.mjs --terrain everon --dry-run
import { existsSync, mkdirSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { exportLocationsFromJsonl, verifyLocationsGates } from "./lib/locations-export.mjs";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const argv = process.argv.slice(2);
const arg = (flag) => {
  const i = argv.indexOf(flag);
  return i >= 0 && i + 1 < argv.length ? argv[i + 1] : undefined;
};
const dryRun = argv.includes("--dry-run");
const terrain =
  arg("--terrain") ??
  process.env.TERRAIN ??
  argv.find((a) => a.startsWith("TERRAIN="))?.split("=")[1] ??
  "everon";

const defaultSrc = join(repoRoot, "packages", "map-assets", terrain, "staging", "export", "raw-entities.jsonl");
const src = arg("--src") ?? defaultSrc;
const outPath = join(repoRoot, "packages", "map-assets", terrain, "locations.json");

if (!existsSync(src)) {
  console.error(`export-locations: source not found: ${src}`);
  console.error("  Run TBD_TerrainWorldExportPlugin (full) + `world copy-export-profile --full` first.");
  console.error("  Or pass --src to a raw-entities.jsonl with World/Locations rows.");
  process.exit(1);
}

const locs = exportLocationsFromJsonl(src, { terrainId: terrain, includePeaks: true });
const gateErrors = verifyLocationsGates(locs);
if (gateErrors.length) {
  for (const e of gateErrors) console.error(`  FAIL  ${e}`);
  process.exit(1);
}

console.log(`export-locations: ${locs.length} rows for ${terrain} (source: ${src})`);
if (dryRun) {
  console.log(JSON.stringify(locs.slice(0, 5), null, 2));
  process.exit(0);
}

mkdirSync(dirname(outPath), { recursive: true });
writeFileSync(outPath, `${JSON.stringify(locs, null, 2)}\n`);
console.log(`  wrote ${outPath}`);
