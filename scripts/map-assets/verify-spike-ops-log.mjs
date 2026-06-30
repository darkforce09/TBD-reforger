#!/usr/bin/env node
// T-090.3.0 — K7 ops-log validation + K2/K3/K4 gate↔artifact consistency (exit 0/1).
//
// Enforces that no `gates.*: "pass"` is mere JSON theatre: a passing tile gate must have a real file,
// a passing OBB gate must have real half-extents or a logged kind-default decision, etc. (micro-nit 2).
// Dep-free.
import { existsSync, readFileSync, statSync } from "node:fs";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { classifyRawEntitiesJsonl } from "./lib/classify-prefab.mjs";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const terrain =
  process.env.TERRAIN ?? process.argv.find((a) => a.startsWith("TERRAIN="))?.split("=")[1] ?? "everon";

const opsPath = join(repoRoot, ".ai", "artifacts", `map_export_${terrain}.json`);
const stagingDir = join(repoRoot, "packages", "map-assets", terrain, "staging", "spike");
const rawPath = join(stagingDir, "raw-entities.jsonl");

const fail = [];
const F = (m) => fail.push(m);

if (!existsSync(opsPath)) {
  console.error(`verify-spike-ops-log: FAIL — ops log not found: ${opsPath}`);
  process.exit(1);
}

let ops;
try {
  ops = JSON.parse(readFileSync(opsPath, "utf8"));
} catch (e) {
  console.error(`verify-spike-ops-log: FAIL — ops log is not valid JSON: ${e}`);
  process.exit(1);
}

const isStr = (v) => typeof v === "string" && v.length > 0;
const isNum = (v) => typeof v === "number" && Number.isFinite(v);
const isObjNonEmpty = (v) => v && typeof v === "object" && !Array.isArray(v) && Object.keys(v).length > 0;
const sizeGt0 = (p) => existsSync(p) && statSync(p).size > 0;
const resolveArtifact = (p) => {
  if (!isStr(p)) return null;
  for (const cand of [isAbsolute(p) ? p : null, join(repoRoot, p), join(stagingDir, p)]) {
    if (cand && existsSync(cand)) return cand;
  }
  return isAbsolute(p) ? p : join(repoRoot, p); // return best-guess path for the error message
};

// --- Required top-level keys ---
const REQUIRED = [
  "schemaVersion", "terrainId", "slice", "generatedAt", "subregionBBoxM",
  "probes", "gates", "handednessRemap", "forestSource", "tileFindings", "sampleRows", "mcpToolsUsed",
];
for (const k of REQUIRED) {
  if (!(k in ops)) F(`missing required key: ${k}`);
}
if ("terrainId" in ops && ops.terrainId !== terrain) F(`terrainId ${ops.terrainId} !== ${terrain}`);
if ("slice" in ops && ops.slice !== "T-090.3.0") F(`slice ${ops.slice} !== T-090.3.0`);
if ("subregionBBoxM" in ops && !(Array.isArray(ops.subregionBBoxM) && ops.subregionBBoxM.length === 4 && ops.subregionBBoxM.every(isNum))) {
  F("subregionBBoxM must be [minX,minY,maxX,maxY] finite numbers");
}

// --- gates ---
const GATE_KEYS = ["K1", "K1b", "K2", "K3", "K4", "K5", "K6", "K7"];
const gates = ops.gates ?? {};
for (const g of GATE_KEYS) {
  if (gates[g] !== "pass" && gates[g] !== "fail") F(`gates.${g} must be "pass" or "fail" (lowercase), got ${JSON.stringify(gates[g])}`);
}
const isPass = (g) => gates[g] === "pass";

// --- K6 handedness ---
if (isPass("K6")) {
  const h = ops.handednessRemap ?? {};
  if (!isStr(h.enfusionBasis)) F("K6 pass requires handednessRemap.enfusionBasis non-empty");
  if (!isStr(h.editorToExport)) F("K6 pass requires handednessRemap.editorToExport non-empty");
  if (!isObjNonEmpty(h.sampleEntity)) F("K6 pass requires handednessRemap.sampleEntity non-empty");
}

// --- K5 forest ---
if (isPass("K5")) {
  if (ops.forestSource !== "engine-mask" && ops.forestSource !== "derived-hull-mandated") {
    F(`K5 pass requires forestSource ∈ {engine-mask, derived-hull-mandated}, got ${JSON.stringify(ops.forestSource)}`);
  }
  const ev = ops.probes?.S5?.evidence ?? ops.probes?.S5?.note;
  if (!isStr(ev)) F("K5 pass requires probes.S5.evidence (MCP citation) non-empty");
}

// --- sampleRows resolve to raw-entities.jsonl (within 0.001 m) ---
const sampleRows = ops.sampleRows ?? [];
if (!Array.isArray(sampleRows) || sampleRows.length !== 3) {
  F(`sampleRows must be exactly 3 (got ${Array.isArray(sampleRows) ? sampleRows.length : "non-array"})`);
}
if (existsSync(rawPath)) {
  const { entries } = classifyRawEntitiesJsonl(rawPath);
  const rows = entries.map((e) => e.row);
  for (let i = 0; i < (Array.isArray(sampleRows) ? sampleRows.length : 0); i++) {
    const s = sampleRows[i];
    if (!isStr(s?.resourceName) || !["x", "y", "z"].every((k) => isNum(s?.[k]))) {
      F(`sampleRows[${i}] needs resourceName + finite x,y,z`);
      continue;
    }
    const hit = rows.some(
      (r) =>
        r.resourceName === s.resourceName &&
        Math.abs(r.x - s.x) <= 0.001 &&
        Math.abs(r.y - s.y) <= 0.001 &&
        Math.abs(r.z - s.z) <= 0.001,
    );
    if (!hit) F(`sampleRows[${i}] (${s.resourceName}) does not resolve to any raw-entities.jsonl line within 0.001 m`);
  }
} else if (sampleRows.length) {
  F(`cannot resolve sampleRows — raw-entities.jsonl missing: ${rawPath}`);
}

// --- K2 OBB consistency ---
if (isPass("K2")) {
  let realObb = false;
  if (existsSync(rawPath)) {
    const { entries } = classifyRawEntitiesJsonl(rawPath);
    realObb = entries.some(
      (e) => e.kind === "building" && Array.isArray(e.row.halfExtentsM) && e.row.halfExtentsM.length === 3 && e.row.halfExtentsM.every(isNum),
    );
  }
  const kindDefault = ops.probes?.S2?.obbDecision === "kind-default" && isStr(ops.probes?.S2?.mcpEvidence);
  if (!realObb && !kindDefault) {
    F("K2 pass requires a building row with numeric halfExtentsM[3] OR probes.S2.obbDecision==='kind-default' + probes.S2.mcpEvidence");
  }
}

// --- K3 satellite tile consistency ---
const sat = ops.tileFindings?.satellite ?? {};
const satFile = resolveArtifact(sat.path);
if (isPass("K3")) {
  if (!isStr(sat.path) || !sizeGt0(satFile)) F(`K3 pass requires tileFindings.satellite.path to be a >0-byte file (looked at ${satFile})`);
} else if (gates.K3 === "fail") {
  if (isStr(sat.path) && sizeGt0(satFile)) F("K3 fail but a satellite tile file is present — inconsistent");
  else if (!(sat.escalate === true && isStr(sat.evidence))) F("K3 fail requires no tile OR tileFindings.satellite.escalate===true with non-empty evidence");
}

// --- K4 map tile consistency ---
const map = ops.tileFindings?.map ?? {};
const mapFile = resolveArtifact(map.path);
const N9 = "synthesized-cartographic required";
const hasN9 =
  map.synthesizedCartographicRequired === true &&
  (String(map.note ?? "").includes(N9) || String(ops.probes?.S4?.note ?? "").includes(N9));
if (isPass("K4")) {
  const hasTile = isStr(map.path) && sizeGt0(mapFile);
  if (!hasTile && !hasN9) F(`K4 pass requires a >0-byte map tile OR synthesizedCartographicRequired + literal "${N9}" note`);
} else if (gates.K4 === "fail") {
  if (isStr(map.path) && sizeGt0(mapFile)) F("K4 fail but a map tile file is present — inconsistent");
}

if (fail.length) {
  console.error(`verify-spike-ops-log: FAIL (${fail.length})`);
  for (const f of fail) console.error(`  ${f}`);
  process.exit(1);
}
console.log("verify-spike-ops-log: OK (K7 + K2/K3/K4 gate↔artifact)");
