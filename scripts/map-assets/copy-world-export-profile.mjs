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
//   node scripts/map-assets/copy-world-export-profile.mjs TERRAIN=everon             (spike files)
//   node scripts/map-assets/copy-world-export-profile.mjs TERRAIN=everon --full      (T-090.3.1 full-world)
//   node scripts/map-assets/copy-world-export-profile.mjs TERRAIN=everon --profile /path/to/profile
//   node scripts/map-assets/copy-world-export-profile.mjs TERRAIN=everon --src /abs/x.jsonl --meta /abs/x.json
// Profile dir resolution: --profile | $PROFILE | $ENFUSION_PROFILE_PATH | ~/Documents/Games/ArmaReforgerWorkbench/profile
//
// --full (T-090.3.1): sources $profile:TBD_WorldExport_full.{jsonl, _meta.json} and stages to
// staging/export/{raw-entities.jsonl, export-meta.json} (gitignored). The plugin writes the meta
// file only AFTER the JSONL closes — meta is the completion sentinel — so --full REFUSES to stage
// when meta is missing or meta.keptCount != JSONL line count (crash / truncated-copy guard).
// A stagedAt stamp (staging/export/staged-meta.json) is written per copy; the census/build steps
// derive generatedAt/exportedAt from it instead of wall clock (determinism gates G4/E6/I6).
import { copyFileSync, existsSync, mkdirSync, readFileSync, unlinkSync, writeFileSync } from "node:fs";
import { createReadStream } from "node:fs";
import { createInterface } from "node:readline";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const argv = process.argv.slice(2);
const arg = (flag) => {
  const i = argv.indexOf(flag);
  return i >= 0 && i + 1 < argv.length ? argv[i + 1] : undefined;
};
const full = argv.includes("--full");
const terrain =
  process.env.TERRAIN ?? argv.find((a) => a.startsWith("TERRAIN="))?.split("=")[1] ?? "everon";

const profileDir =
  arg("--profile") ??
  process.env.PROFILE ??
  process.env.ENFUSION_PROFILE_PATH ??
  join(homedir(), "Documents", "Games", "ArmaReforgerWorkbench", "profile");

const srcJsonl =
  arg("--src") ?? join(profileDir, full ? "TBD_WorldExport_full.jsonl" : "TBD_WorldExport_subregion.jsonl");
const srcMeta =
  arg("--meta") ?? join(profileDir, full ? "TBD_WorldExport_full_meta.json" : "TBD_WorldExport_meta.json");

const destDir = join(repoRoot, "packages", "map-assets", terrain, "staging", full ? "export" : "spike");
const destJsonl = join(destDir, "raw-entities.jsonl");
const destMeta = join(destDir, "export-meta.json");
const destStamp = join(destDir, "staged-meta.json");

if (!existsSync(srcJsonl)) {
  console.error(`copy-world-export-profile: source jsonl not found: ${srcJsonl}`);
  console.error("  Run the TBD_TerrainWorldExportPlugin in Workbench first, or pass --src / --profile.");
  process.exit(1);
}

const countLines = async (path) => {
  const rl = createInterface({ input: createReadStream(path, { encoding: "utf8" }), crlfDelay: Infinity });
  let n = 0;
  for await (const line of rl) if (line.trim()) n++;
  return n;
};

if (full && !existsSync(srcMeta)) {
  console.error(`copy-world-export-profile: --full refused — completion-sentinel meta missing: ${srcMeta}`);
  console.error("  The plugin writes meta only after the JSONL closes; a missing meta = crashed/partial run.");
  process.exit(1);
}

mkdirSync(destDir, { recursive: true });
copyFileSync(srcJsonl, destJsonl);
const lineCount = await countLines(destJsonl);

if (full) {
  const meta = JSON.parse(readFileSync(srcMeta, "utf8"));
  if (typeof meta.keptCount !== "number" || meta.keptCount !== lineCount) {
    unlinkSync(destJsonl);
    console.error(
      `copy-world-export-profile: --full refused — meta.keptCount ${meta.keptCount} != staged line count ${lineCount} (truncated copy?). Staged jsonl removed.`,
    );
    process.exit(1);
  }
  copyFileSync(srcMeta, destMeta);
  writeFileSync(
    destStamp,
    `${JSON.stringify({ terrain, stagedAt: new Date().toISOString(), keptCount: lineCount, source: srcJsonl }, null, 2)}\n`,
  );
  console.log(
    `copy-world-export-profile: ${terrain} FULL — staged ${lineCount} rows → ${destJsonl}; meta + stagedAt stamp written`,
  );
} else {
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
}
