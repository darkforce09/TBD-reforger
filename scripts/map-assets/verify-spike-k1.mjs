#!/usr/bin/env node
// T-090.3.0 — K1 predicate (exit 0/1; no manual judgment).
//
// K1 PASS iff ∃ line in staging/spike/raw-entities.jsonl where the SHARED classifier resolves
// kind==="building" AND resourceName is non-empty AND x,y,z,yawDeg,pitchDeg,rollDeg are all finite
// numbers. Uses scripts/map-assets/lib/classify-prefab.mjs — the same module census-spike.mjs uses,
// so K1 and K1b cannot disagree (micro-nit 1).
import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { classifyRawEntitiesJsonl } from "./lib/classify-prefab.mjs";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const terrain =
  process.env.TERRAIN ?? process.argv.find((a) => a.startsWith("TERRAIN="))?.split("=")[1] ?? "everon";

const rawPath = join(repoRoot, "packages", "map-assets", terrain, "staging", "spike", "raw-entities.jsonl");

const isFiniteNum = (v) => typeof v === "number" && Number.isFinite(v);

/** Shared K1 predicate over a classified entry — exported so census-spike can cross-check. */
export function entryIsK1Building(entry) {
  const r = entry.row ?? {};
  return (
    entry.kind === "building" &&
    typeof r.resourceName === "string" &&
    r.resourceName.length > 0 &&
    ["x", "y", "z", "yawDeg", "pitchDeg", "rollDeg"].every((k) => isFiniteNum(r[k]))
  );
}

export function k1Result(terrainId = terrain) {
  const p = join(repoRoot, "packages", "map-assets", terrainId, "staging", "spike", "raw-entities.jsonl");
  if (!existsSync(p)) return { pass: false, reason: `raw-entities.jsonl not found: ${p}`, match: null };
  const { entries, errors } = classifyRawEntitiesJsonl(p);
  if (errors.length) return { pass: false, reason: `jsonl parse errors: ${JSON.stringify(errors.slice(0, 3))}`, match: null };
  const match = entries.find(entryIsK1Building);
  return match
    ? { pass: true, reason: `building row: ${match.row.resourceName}`, match }
    : { pass: false, reason: `no building-classified row with complete transform among ${entries.length} rows`, match: null };
}

// CLI
if (import.meta.url === `file://${process.argv[1]}`) {
  if (!existsSync(rawPath)) {
    console.error(`verify-spike-k1: FAIL — raw-entities.jsonl not found: ${rawPath}`);
    process.exit(1);
  }
  const res = k1Result();
  if (res.pass) {
    console.log(`verify-spike-k1: PASS (K1) — ${res.reason}`);
    process.exit(0);
  }
  console.error(`verify-spike-k1: FAIL (K1) — ${res.reason}`);
  process.exit(1);
}
