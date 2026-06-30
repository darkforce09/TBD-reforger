#!/usr/bin/env node
// T-090.3.0 — copy the Workbench plugin's $profile: world-export output into the spike staging tree.
//
// Mirrors the T-091 pattern (raw-u16-to-dem-png.mjs reads $PROFILE/* written by TBD_TerrainExportPlugin).
// TBD_TerrainWorldExportPlugin.c writes:
//   $profile:TBD_WorldExport_subregion.jsonl   (one entity per line — see plan "Raw entity row schema")
//   $profile:TBD_WorldExport_meta.json         (bbox, totalScanned, keptCount, obbApiAvailable, anglesOrderNote)
// This copies them to packages/map-assets/<terrain>/staging/spike/{raw-entities.jsonl, export-meta.json}.
//
// Usage:
//   node scripts/map-assets/copy-world-export-profile.mjs TERRAIN=everon
//   node scripts/map-assets/copy-world-export-profile.mjs TERRAIN=everon --profile /path/to/profile
//   node scripts/map-assets/copy-world-export-profile.mjs TERRAIN=everon --src /abs/x.jsonl --meta /abs/x.json
// Profile dir resolution: --profile | $PROFILE | $ENFUSION_PROFILE_PATH | ~/Documents/Games/ArmaReforgerWorkbench/profile
import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const argv = process.argv.slice(2);
const arg = (flag) => {
  const i = argv.indexOf(flag);
  return i >= 0 && i + 1 < argv.length ? argv[i + 1] : undefined;
};
const terrain =
  process.env.TERRAIN ?? argv.find((a) => a.startsWith("TERRAIN="))?.split("=")[1] ?? "everon";

const profileDir =
  arg("--profile") ??
  process.env.PROFILE ??
  process.env.ENFUSION_PROFILE_PATH ??
  join(homedir(), "Documents", "Games", "ArmaReforgerWorkbench", "profile");

const srcJsonl = arg("--src") ?? join(profileDir, "TBD_WorldExport_subregion.jsonl");
const srcMeta = arg("--meta") ?? join(profileDir, "TBD_WorldExport_meta.json");

const destDir = join(repoRoot, "packages", "map-assets", terrain, "staging", "spike");
const destJsonl = join(destDir, "raw-entities.jsonl");
const destMeta = join(destDir, "export-meta.json");

if (!existsSync(srcJsonl)) {
  console.error(`copy-world-export-profile: source jsonl not found: ${srcJsonl}`);
  console.error("  Run the TBD_TerrainWorldExportPlugin in Workbench first, or pass --src / --profile.");
  process.exit(1);
}

mkdirSync(destDir, { recursive: true });
copyFileSync(srcJsonl, destJsonl);

let lineCount = 0;
for (const line of readFileSync(destJsonl, "utf8").split("\n")) {
  if (line.trim()) lineCount++;
}

let metaNote = "no meta file";
if (existsSync(srcMeta)) {
  copyFileSync(srcMeta, destMeta);
  metaNote = `meta → ${destMeta}`;
} else {
  // Record provenance even when the plugin wrote no meta.
  writeFileSync(destMeta, `${JSON.stringify({ source: srcJsonl, copiedRows: lineCount }, null, 2)}\n`);
  metaNote = `meta synthesized (plugin wrote none) → ${destMeta}`;
}

console.log(`copy-world-export-profile: ${terrain} — copied ${lineCount} rows → ${destJsonl}; ${metaNote}`);
