#!/usr/bin/env node
// T-090.2 — prefab/instance census → type-inventory.json (exact integers only).
// Today: validates the committed inventory + exits 0 for pending_export.
// After T-090.3 export ships: scans raw-entities.jsonl / catalog and writes exact counts.
import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const terrain = process.env.TERRAIN ?? process.argv.find((a) => a.startsWith("TERRAIN="))?.split("=")[1];
if (!terrain) {
  console.error("map-census: TERRAIN=<id> required (e.g. make map-census TERRAIN=everon)");
  process.exit(1);
}

const inventoryPath = join(repoRoot, "packages", "map-assets", terrain, "objects", "type-inventory.json");
if (!existsSync(inventoryPath)) {
  console.error(`map-census: missing ${inventoryPath}`);
  process.exit(1);
}

const verifyScript = join(repoRoot, "packages", "tbd-schema", "scripts", "verify-type-inventory.mjs");
const verify = spawnSync(process.execPath, [verifyScript], { stdio: "inherit" });
if (verify.status !== 0) process.exit(verify.status ?? 1);

const inv = JSON.parse(readFileSync(inventoryPath, "utf8"));
// T-090.3.0 census guard (Option 1): the "finish full census" guard keys off the FULL-MAP export
// path only. The spike subregion (staging/spike/raw-entities.jsonl) is a feasibility probe — it must
// NOT block `make map-census` while the committed inventory is legitimately still pending_export.
const fullExport = join(repoRoot, "packages", "map-assets", terrain, "export", "raw-entities.jsonl");
const spikeExport = join(repoRoot, "packages", "map-assets", terrain, "staging", "spike", "raw-entities.jsonl");

if (inv.censusStatus === "pending_export") {
  if (existsSync(fullExport)) {
    console.error(
      "map-census: full-map export exists but censusStatus is still pending_export — run full classify + census implementation (T-090.2/.3)",
    );
    process.exit(1);
  }
  if (existsSync(spikeExport)) {
    console.log(
      `map-census: ${terrain} censusStatus=pending_export — T-090.3.0 spike subregion export present (staging/spike); full-map census still pending (expected)`,
    );
    process.exit(0);
  }
  console.log(
    `map-census: ${terrain} censusStatus=pending_export — exact counts unknown until Workbench export + classify (see t090_world_object_type_inventory.md)`,
  );
  process.exit(0);
}

// Future: compute from export and write inventoryPath, then re-verify.
console.log(`map-census: ${terrain} censusStatus=${inv.censusStatus} — validation only (compute path T-090.2/.3)`);
writeFileSync(
  join(repoRoot, ".ai", "artifacts", `type_inventory_${terrain}.json`),
  `${JSON.stringify(inv, null, 2)}\n`,
);
