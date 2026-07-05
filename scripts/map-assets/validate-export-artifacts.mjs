#!/usr/bin/env node
// T-090.3.1 — `make map-export-validate`: CI-safe validation of COMMITTED export artifacts for
// every terrain in terrain-registry.json. No staging, no pak VFS, no Workbench — the staging-
// dependent gates (G11, E6, P1-4-staged) live in verify-phase.mjs / `make map-verify-phase`.
//
// Per terrain with a manifest.objects.prefabsPath: prefab rows + chunk rows + roads + inventory
// Ajv-valid, sidecar sums == manifest.instanceCount, chunk partition spot-check.
// E2 (identical script path for every terrain), three computable assertions:
//   (a) registry has >= 2 terrain rows;
//   (b) export-terrain.sh <non-exported terrain> exits 2 through the no-staged-raw
//       operator-instructions branch (proves the same code path runs for a second terrain);
//   (c) grep gate — no literal terrain id in the T-090.3.1 pipeline scripts outside per-terrain
//       config tables (terrain always flows from argv/registry).
import { createRequire } from "node:module";
import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { gunzipSync } from "node:zlib";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { cellOf, chunkKey } from "./lib/anchor-check.mjs";
import {
  DENSITY_CELL_M,
  DENSITY_CHANNELS,
  DENSITY_COLS,
  DENSITY_ROWS,
  TBDD_FILE_BYTES,
  TBDD_VERSION,
  accumulateCorners,
  decodeTBDD,
  sliceChunkCorners,
} from "./lib/density-grid.mjs";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const here = dirname(fileURLToPath(import.meta.url));
const schemaPkg = join(repoRoot, "packages", "tbd-schema");
const require = createRequire(join(schemaPkg, "package.json"));
const Ajv = require("ajv/dist/2020.js").default;
const addFormats = require("ajv-formats").default;
const readJSON = (p) => JSON.parse(readFileSync(p, "utf8"));
const gunzipJSON = (p) => JSON.parse(gunzipSync(readFileSync(p)).toString("utf8"));

const ajv = new Ajv({ allErrors: false, strict: true, strictTuples: false, allowUnionTypes: true });
addFormats(ajv);
ajv.addSchema(
  [
    "map-object-enums.schema.json",
    "map-object-prefab.schema.json",
    "map-object-instance.schema.json",
    "map-object-region.schema.json",
    "map-object-roads.schema.json",
    "map-object-catalog.schema.json",
    "map-object-resolved.schema.json",
    "map-object-type-inventory.schema.json",
    "terrain-registry.schema.json",
  ].map((f) => readJSON(join(schemaPkg, "schema", f))),
);
const byId = (n) => ajv.getSchema(`https://schema.tbdevent.eu/${n}/v1.json`);
const vPrefab = byId("map-object-prefab");
const vInstance = byId("map-object-instance");
const vRoads = byId("map-object-roads");
const vRegion = byId("map-object-region");
const vRegistry = byId("terrain-registry");

let failures = 0;
const fail = (msg) => {
  failures++;
  console.log(`  FAIL  ${msg}`);
};
const pass = (msg) => console.log(`  PASS  ${msg}`);

const registryPath = join(repoRoot, "packages", "map-assets", "terrain-registry.json");
const registry = readJSON(registryPath);
if (!vRegistry(registry)) fail(`terrain-registry.json schema: ${JSON.stringify(vRegistry.errors?.[0])}`);
else pass("terrain-registry.json schema valid");

for (const t of registry.terrains) {
  const terrainDir = join(repoRoot, "packages", "map-assets", t.terrainId);
  const manifestPath = join(repoRoot, "packages", "map-assets", t.manifestPath);
  if (!existsSync(manifestPath)) {
    pass(`${t.terrainId}: no manifest (status ${t.status}) — skipped`);
    continue;
  }
  const manifest = readJSON(manifestPath);
  if (!manifest.objects?.prefabsPath) {
    pass(`${t.terrainId}: manifest has no objects export yet — skipped`);
    continue;
  }
  const worldSizeM = t.worldBoundsM[2];
  const objects = manifest.objects;

  const prefabsDoc = gunzipJSON(join(terrainDir, objects.prefabsPath));
  let bad = 0;
  for (const p of prefabsDoc.prefabs) if (!vPrefab(p)) bad++;
  bad === 0 ? pass(`${t.terrainId}: ${prefabsDoc.prefabs.length} prefab rows schema-valid`) : fail(`${t.terrainId}: ${bad} invalid prefab rows`);

  const chunksDir = join(terrainDir, objects.chunksPath);
  const sidecar = readJSON(join(chunksDir, "manifest.json"));
  let rowTotal = 0;
  let chunkErrs = 0;
  const treeRows = []; // committed tree instances — density tree-channel recompute below
  for (const c of sidecar.cells) {
    const doc = gunzipJSON(join(terrainDir, c.path));
    rowTotal += doc.instances.length;
    if (doc.instances.length !== c.instanceCount) chunkErrs++;
    for (const row of doc.instances) {
      if (!vInstance(row)) chunkErrs++;
      else if (cellOf(row[1], sidecar.chunkSizeM, worldSizeM) !== c.cx || cellOf(row[2], sidecar.chunkSizeM, worldSizeM) !== c.cy) chunkErrs++;
      if (prefabsDoc.prefabs[row[0]]?.kind === "tree") treeRows.push({ x: row[1], y: row[2] });
    }
  }
  chunkErrs === 0
    ? pass(`${t.terrainId}: ${sidecar.cells.length} chunks, ${rowTotal} rows valid + partition-correct`)
    : fail(`${t.terrainId}: ${chunkErrs} chunk row/partition error(s)`);
  rowTotal === objects.instanceCount
    ? pass(`${t.terrainId}: manifest.objects.instanceCount = ${rowTotal}`)
    : fail(`${t.terrainId}: manifest.objects.instanceCount ${objects.instanceCount} != chunk rows ${rowTotal}`);
  prefabsDoc.prefabs.length === objects.prefabCount
    ? pass(`${t.terrainId}: manifest.objects.prefabCount = ${objects.prefabCount}`)
    : fail(`${t.terrainId}: manifest.objects.prefabCount ${objects.prefabCount} != ${prefabsDoc.prefabs.length}`);

  const roadsDoc = gunzipJSON(join(terrainDir, objects.roadsPath));
  vRoads(roadsDoc) && roadsDoc.roadSegments.length > 0
    ? pass(`${t.terrainId}: roads.json.gz valid (${roadsDoc.roadSegments.length} segments)`)
    : fail(`${t.terrainId}: roads.json.gz invalid or empty`);

  const inventory = readJSON(join(terrainDir, objects.typeInventoryPath));

  // T-090.3.2 — density grids (committed-only: tree channel recomputes from committed chunks;
  // the rock channel needs the gitignored staged raw, so CI checks its header/size only — the
  // full rock byte-compare lives in `make map-verify-phase` D2).
  if (objects.densityPath) {
    const densityDir = join(terrainDir, objects.densityPath);
    const gridCells = Math.round(worldSizeM / objects.chunkSizeM);
    const onDisk = existsSync(densityDir) ? readdirSync(densityDir).filter((f) => f.endsWith(".bin")) : [];
    let dErrs = 0;
    if (onDisk.length !== gridCells * gridCells) {
      dErrs++;
      console.log(`        ${t.terrainId}: density file count ${onDisk.length} != ${gridCells * gridCells}`);
    }
    if (objects.densityCellM !== DENSITY_CELL_M) {
      dErrs++;
      console.log(`        ${t.terrainId}: manifest densityCellM ${objects.densityCellM} != ${DENSITY_CELL_M}`);
    }
    const treeAcc = accumulateCorners(treeRows, worldSizeM);
    for (let cy = 0; cy < gridCells && dErrs < 8; cy++) {
      for (let cx = 0; cx < gridCells && dErrs < 8; cx++) {
        const p = join(densityDir, `${chunkKey(cx, cy)}.bin`);
        if (!existsSync(p)) {
          dErrs++;
          continue;
        }
        const buf = readFileSync(p);
        let dec;
        try {
          dec = decodeTBDD(buf);
        } catch {
          dErrs++;
          continue;
        }
        if (
          buf.length !== TBDD_FILE_BYTES ||
          dec.version !== TBDD_VERSION ||
          dec.cellM !== DENSITY_CELL_M ||
          dec.cols !== DENSITY_COLS ||
          dec.rows !== DENSITY_ROWS ||
          dec.channelCount !== DENSITY_CHANNELS.length
        ) {
          dErrs++;
          continue;
        }
        const expectTree = sliceChunkCorners(treeAcc.grid, treeAcc.size, cx, cy);
        for (let k = 0; k < expectTree.length; k++) {
          if (dec.channels[0][k] !== expectTree[k]) {
            dErrs++;
            console.log(`        ${t.terrainId}: density ${cx}_${cy} tree channel differs from committed chunks at corner ${k}`);
            break;
          }
        }
      }
    }
    dErrs === 0
      ? pass(`${t.terrainId}: ${onDisk.length} density bins valid (header + tree channel == committed chunks)`)
      : fail(`${t.terrainId}: ${dErrs} density error(s)`);
  }

  // T-090.3.2 — forest regions: rows schema-valid + F2 exact identity against the inventory.
  if (objects.regionsPath) {
    const doc = gunzipJSON(join(terrainDir, objects.regionsPath));
    let rErrs = 0;
    for (const r of doc.regions) if (!vRegion(r)) rErrs++;
    const regionTreeSum = doc.regions.reduce((s, r) => s + (r.treeCount ?? 0), 0);
    const f2 = regionTreeSum + (inventory.unassignedTrees ?? 0) === inventory.byKind.tree.instances;
    if (!f2) {
      rErrs++;
      console.log(`        ${t.terrainId}: F2 ${regionTreeSum} + ${inventory.unassignedTrees} != tree instances ${inventory.byKind.tree.instances}`);
    }
    if ((inventory.byRegionKind?.forest?.count ?? -1) !== doc.regions.length) rErrs++;
    if ((inventory.byRegionKind?.forest?.treeCount ?? -1) !== regionTreeSum) rErrs++;
    rErrs === 0
      ? pass(`${t.terrainId}: forest-regions.json.gz valid (${doc.regions.length} regions, F2 exact)`)
      : fail(`${t.terrainId}: ${rErrs} forest-region error(s)`);
  }
}

// Inventory gates (I1-I7 subset) — delegate to the schema package verifier (validates every terrain).
const inv = spawnSync(process.execPath, [join(schemaPkg, "scripts", "verify-type-inventory.mjs")], { stdio: "pipe" });
inv.status === 0 ? pass("verify-type-inventory (I-gates) OK") : fail(`verify-type-inventory: ${String(inv.stdout).trim()} ${String(inv.stderr).trim()}`);

// ---- E2 — identical script path for every terrain -------------------------------------------------
registry.terrains.length >= 2 ? pass(`E2a: registry has ${registry.terrains.length} terrains`) : fail("E2a: registry needs >= 2 terrain rows");

const other = registry.terrains.find((t) => !existsSync(join(repoRoot, "packages", "map-assets", t.terrainId, "staging", "export", "raw-entities.jsonl")));
if (other) {
  const r = spawnSync("bash", [join(here, "export-terrain.sh"), other.terrainId, "--phase", "P1_buildings"], { stdio: "pipe" });
  r.status === 2
    ? pass(`E2b: export-terrain.sh ${other.terrainId} -> exit 2 (operator-instructions branch, same code path)`)
    : fail(`E2b: export-terrain.sh ${other.terrainId} expected exit 2, got ${r.status}`);
} else {
  pass("E2b: every terrain already staged — branch untestable (OK)");
}

{
  // E2c: terrain ids must flow from argv/registry — no literal id in the new pipeline scripts.
  const newScripts = [
    "build-world-objects.mjs",
    "build-roads-from-topo.mjs",
    "verify-phase.mjs",
    "validate-export-artifacts.mjs",
    "export-terrain.sh",
    join("lib", "anchor-check.mjs"),
    join("lib", "density-grid.mjs"),
    join("lib", "forest-regions.mjs"),
  ];
  const offenders = [];
  for (const s of newScripts) {
    const text = readFileSync(join(here, s), "utf8");
    for (const t of registry.terrains) {
      for (const [i, line] of text.split("\n").entries()) {
        if (line.includes(t.terrainId) && !/E2c-allow/.test(line)) offenders.push(`${s}:${i + 1} literal '${t.terrainId}'`);
      }
    }
  }
  offenders.length === 0 ? pass("E2c: no literal terrain ids in pipeline scripts") : fail(`E2c: ${offenders.join("; ")}`);
}

if (failures) {
  console.error(`\nmap-export-validate: FAIL (${failures})`);
  process.exit(1);
}
console.log("\nmap-export-validate: OK");
