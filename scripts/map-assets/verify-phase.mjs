#!/usr/bin/env node
// T-090.3.1 — `make map-verify-phase`: mathematical phase gate (t090_phased_object_import.md).
// Runs G1-G12 global invariants + P1-*/PH-P2-* phase gates + E6/G4/I6 determinism on the STAGED
// raw export and the COMMITTED objects/ artifacts. No eyeball checks — every gate is a computable
// predicate.
//
// T-090.3.2 phase-scope split: phases are CUMULATIVE (the committed catalog holds every kind up to
// manifest.objects.importPhaseMax), so phase-scoped gates (G11, P1-*, PH-P2-*) filter committed
// rows/prefabs down to the REQUESTED phase's kinds, while catalog-scope gates (G1-G10, G12, SIZE)
// always run on the whole committed artifact set. E6 rebuilds at the COMMITTED importPhaseMax —
// re-running PHASE=P1_buildings on a P2 catalog must stay green (no-regression rule).
// New in 3.2: D1/D2 (TBDD density grids), F1/F2/F6 (forest regions), PH-P2-1..5.
//
// Needs: staging/export/{raw-entities.jsonl, export-meta.json, staged-meta.json} (gitignored) and a
// prior `make map-export` run. Ajv is borrowed from packages/tbd-schema/node_modules via
// createRequire (run `npm ci` there once — `make schema-validate` does).
//
// The P1-4 anchor check imports the SAME checkAnchors used on the synthetic golden
// (verify-map-object-golden S12) but re-derives remap/partition inline via lib/anchor-check.mjs —
// build-world-objects.mjs is deliberately NOT imported here (non-circularity). The D2/F6
// recompute imports the pure libs (density-grid, forest-regions) — those ARE the contract.
import { createRequire } from "node:module";
import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync } from "node:fs";
import { gunzipSync } from "node:zlib";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { checkAnchors, cellOf, chunkKey } from "./lib/anchor-check.mjs";
import { createClassifier, streamRawEntities } from "./lib/classify-prefab.mjs";
import {
  DENSITY_CELL_M,
  DENSITY_CHANNELS,
  DENSITY_COLS,
  DENSITY_ROWS,
  TBDD_FILE_BYTES,
  TBDD_VERSION,
  accumulateCorners,
  decodeTBDD,
  encodeTBDD,
  sliceChunkCorners,
  sumGrid,
} from "./lib/density-grid.mjs";
import { deriveForestRegions } from "./lib/forest-regions.mjs";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const argv = process.argv.slice(2);
const arg = (flag) => {
  const i = argv.indexOf(flag);
  return i >= 0 && i + 1 < argv.length ? argv[i + 1] : undefined;
};
const terrain = arg("--terrain");
const phase = arg("--phase");
if (!terrain || !phase) {
  console.error("verify-phase: --terrain <id> --phase <Pn> required");
  process.exit(1);
}
const PHASE_KINDS = { P1_buildings: ["building"], P2_trees: ["building", "tree"] };
if (!PHASE_KINDS[phase]) {
  console.error(`verify-phase: phase '${phase}' not implemented (have: ${Object.keys(PHASE_KINDS).join(", ")})`);
  process.exit(1);
}
const phaseKinds = new Set(PHASE_KINDS[phase]);
const densityPhase = phase === "P2_trees";
const CHUNK_SIZE_M = 512;
const MAX_CHUNK_AGGREGATE_BYTES = 40 * 1024 * 1024; // LFS decision forced before P2 trees

const schemaPkg = join(repoRoot, "packages", "tbd-schema");
const require = createRequire(join(schemaPkg, "package.json"));
const Ajv = require("ajv/dist/2020.js").default;
const addFormats = require("ajv-formats").default;
const readJSON = (p) => JSON.parse(readFileSync(p, "utf8"));

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
const vResolved = byId("map-object-resolved");
const vInventory = byId("map-object-type-inventory");
const vRegion = byId("map-object-region");

const terrainDir = join(repoRoot, "packages", "map-assets", terrain);
const objectsDir = join(terrainDir, "objects");
const chunksDir = join(objectsDir, "chunks");
const stagingDir = join(terrainDir, "staging", "export");
const rawPath = join(stagingDir, "raw-entities.jsonl");
if (!existsSync(rawPath)) {
  console.error(`verify-phase: staged raw missing (${rawPath}) — run make map-export first`);
  process.exit(2);
}

const registry = readJSON(join(repoRoot, "packages", "map-assets", "terrain-registry.json"));
const terrainRow = registry.terrains.find((t) => t.terrainId === terrain);
const worldSizeM = terrainRow.worldBoundsM[2];

const gunzipJSON = (p) => JSON.parse(gunzipSync(readFileSync(p)).toString("utf8"));
const prefabsDoc = gunzipJSON(join(objectsDir, "prefabs.json.gz"));
const prefabs = prefabsDoc.prefabs;
const chunkManifest = readJSON(join(chunksDir, "manifest.json"));
const roadsDoc = gunzipJSON(join(objectsDir, "roads.json.gz"));
const inventory = readJSON(join(objectsDir, "type-inventory.json"));
const manifest = readJSON(join(terrainDir, "manifest.json"));

const chunkCache = new Map();
const loadChunk = (cx, cy) => {
  const key = chunkKey(cx, cy);
  if (!chunkCache.has(key)) {
    const p = join(chunksDir, `${key}.json.gz`);
    chunkCache.set(key, existsSync(p) ? gunzipJSON(p) : null);
  }
  return chunkCache.get(key);
};

const gates = [];
const gate = (id, label, errs) => gates.push({ id, label, errs: errs.slice(0, 8), errCount: errs.length });
const round2 = (v) => Math.round(v * 100) / 100;

// ---- stream staged raw once: G11 parity + P1-4 anchor pool + D2 rock rows -----------------------
const classify = createClassifier();
let rawPhaseCount = 0; // classified phase-kind, resourceName != "", in-bounds after round (builder predicate)
const rawKindCounts = new Map(); // kind -> in-bounds classified count (phase-scoped G11 splits)
const anchorPool = []; // P1-4 only — building anchors
const rawRockRows = []; // D2 rock channel recompute (rocks are not committed instances before P4)
await streamRawEntities(rawPath, (row) => {
  const rn = typeof row.resourceName === "string" ? row.resourceName : "";
  if (rn === "") return;
  const cls = classify(rn);
  const x = round2(row.x);
  const y = round2(row.z);
  const inBounds = x >= 0 && x <= worldSizeM && y >= 0 && y <= worldSizeM;
  if (densityPhase && cls.kind === "rock" && inBounds) rawRockRows.push({ x, y });
  if (!phaseKinds.has(cls.kind)) return;
  if (!inBounds) return;
  rawPhaseCount++;
  rawKindCounts.set(cls.kind, (rawKindCounts.get(cls.kind) ?? 0) + 1);
  if (phase === "P1_buildings") {
    anchorPool.push({ resourceName: rn, x: row.x, y: row.y, z: row.z, headingDeg: row.headingDeg ?? row.pitchDeg ?? 0 });
  }
});

// ---- load all chunk rows once --------------------------------------------------------------------
const allChunkFiles = readdirSync(chunksDir).filter((f) => f.endsWith(".json.gz"));
let chunkAggregateBytes = 0;
const rowsByKey = new Map();
for (const f of allChunkFiles) {
  chunkAggregateBytes += statSync(join(chunksDir, f)).size;
  rowsByKey.set(f.replace(".json.gz", ""), gunzipJSON(join(chunksDir, f)).instances);
}
let actualInstanceCount = 0;
for (const rows of rowsByKey.values()) actualInstanceCount += rows.length;

// Phase-scope views over the committed catalog (cumulative phases — see header).
const committedKindCounts = new Map(); // kind -> committed instance count
for (const rows of rowsByKey.values()) {
  for (const row of rows) {
    const k = prefabs[row[0]]?.kind;
    committedKindCounts.set(k, (committedKindCounts.get(k) ?? 0) + 1);
  }
}
let committedPhaseCount = 0;
for (const [k, n] of committedKindCounts) if (phaseKinds.has(k)) committedPhaseCount += n;

// ---- G1 schema validity ---------------------------------------------------------------------------
{
  const errs = [];
  for (const p of prefabs) if (!vPrefab(p)) errs.push(`prefab ${p.prefabId}: ${vPrefab.errors?.[0]?.message}`);
  for (const [key, rows] of rowsByKey) {
    for (const [i, row] of rows.entries()) {
      if (!vInstance(row)) errs.push(`chunk ${key}[${i}]: ${vInstance.errors?.[0]?.message}`);
      else if (!Array.isArray(row) || row.length !== 5 || typeof row[0] !== "number") errs.push(`chunk ${key}[${i}]: not a 5-number tuple`);
    }
  }
  if (!vRoads(roadsDoc)) errs.push(`roads.json.gz: ${JSON.stringify(vRoads.errors?.[0])}`);
  if (!vInventory(inventory)) errs.push(`type-inventory.json: ${JSON.stringify(vInventory.errors?.[0])}`);
  gate("G1", "schema valid (prefabs, chunk rows, roads, inventory)", errs);
}

// ---- G2 resolved materialization -----------------------------------------------------------------
{
  const errs = [];
  for (const [key, rows] of rowsByKey) {
    for (const [i, row] of rows.entries()) {
      const p = prefabs[row[0]];
      if (!p) continue; // G3's finding
      const resolved = {
        id: `${key}:${i}`,
        prefabId: p.prefabId,
        resourceName: p.resourceName,
        kind: p.kind,
        class: p.class,
        label: p.label ?? "",
        taxonomyPath: p.ai.taxonomyPath,
        summary: p.ai.summary,
        x: row[1],
        y: row[2],
        z: row[3],
        rotationDeg: row[4],
        spatial: p.spatial,
        gameplay: p.gameplay,
        tags: p.tags ?? [],
      };
      if (!vResolved(resolved)) errs.push(`resolved ${key}:${i}: ${JSON.stringify(vResolved.errors?.[0])}`);
    }
  }
  gate("G2", "all instances materialize to valid ResolvedWorldObject", errs);
}

// ---- G3 / G12 prefab bijection + orphans ----------------------------------------------------------
{
  const errs = [];
  const referenced = new Array(prefabs.length).fill(0);
  for (const [key, rows] of rowsByKey) {
    for (const [i, row] of rows.entries()) {
      if (!(Number.isInteger(row[0]) && row[0] >= 0 && row[0] < prefabs.length)) errs.push(`chunk ${key}[${i}]: prefabId ${row[0]} out of range`);
      else referenced[row[0]]++;
    }
  }
  gate("G3", "prefabId bijection (0 <= id < prefabs.length)", errs);
  const orphans = prefabs.filter((p, i) => referenced[i] === 0 && !p.tags?.includes("prefabOnly"));
  gate("G12", "no orphan prefabs", orphans.map((p) => `prefab ${p.prefabId} ${p.resourceName} has 0 instances`));
}

// ---- G5 derived-id uniqueness + sidecar consistency ----------------------------------------------
{
  const errs = [];
  const sidecarKeys = new Set(chunkManifest.cells.map((c) => chunkKey(c.cx, c.cy)));
  if (sidecarKeys.size !== chunkManifest.cells.length) errs.push("chunks/manifest.json: duplicate (cx,cy) cells");
  for (const c of chunkManifest.cells) {
    const rows = rowsByKey.get(chunkKey(c.cx, c.cy));
    if (!rows) errs.push(`sidecar cell ${c.cx}_${c.cy}: chunk file missing`);
    else if (rows.length !== c.instanceCount) errs.push(`sidecar cell ${c.cx}_${c.cy}: instanceCount ${c.instanceCount} != actual ${rows.length}`);
  }
  for (const key of rowsByKey.keys()) if (!sidecarKeys.has(key)) errs.push(`chunk file ${key} not in sidecar manifest`);
  gate("G5", "derived instance ids unique (sidecar <-> files consistent)", errs);
}

// ---- G6 chunk partition + G8 bounds ---------------------------------------------------------------
{
  const g6 = [];
  const g8 = [];
  for (const [key, rows] of rowsByKey) {
    const [cx, cy] = key.split("_").map(Number);
    for (const [i, row] of rows.entries()) {
      const [, x, y] = row;
      if (cellOf(x, CHUNK_SIZE_M, worldSizeM) !== cx || cellOf(y, CHUNK_SIZE_M, worldSizeM) !== cy) {
        g6.push(`chunk ${key}[${i}]: (${x}, ${y}) partitions to ${chunkKey(cellOf(x, CHUNK_SIZE_M, worldSizeM), cellOf(y, CHUNK_SIZE_M, worldSizeM))}`);
      }
      if (x < 0 || x > worldSizeM || y < 0 || y > worldSizeM) g8.push(`chunk ${key}[${i}]: (${x}, ${y}) outside world bounds`);
    }
  }
  gate("G6", "chunk partition (clamp(floor(coord/512)))", g6);
  gate("G8", "world bounds 0 <= x,y <= maxX", g8);
}

// ---- G7 count identities --------------------------------------------------------------------------
{
  const errs = [];
  const sidecarSum = chunkManifest.cells.reduce((n, c) => n + c.instanceCount, 0);
  if (sidecarSum !== actualInstanceCount) errs.push(`sidecar sum ${sidecarSum} != actual rows ${actualInstanceCount}`);
  if (manifest.objects?.instanceCount !== actualInstanceCount) errs.push(`manifest.objects.instanceCount ${manifest.objects?.instanceCount} != actual ${actualInstanceCount}`);
  if (manifest.objects?.prefabCount !== prefabs.length) errs.push(`manifest.objects.prefabCount ${manifest.objects?.prefabCount} != prefabs ${prefabs.length}`);
  if (inventory.levels.totalInstances !== actualInstanceCount) errs.push(`inventory totalInstances ${inventory.levels.totalInstances} != actual ${actualInstanceCount}`);
  if (inventory.levels.uniquePrefabs !== prefabs.length) errs.push(`inventory uniquePrefabs ${inventory.levels.uniquePrefabs} != prefabs ${prefabs.length}`);
  gate("G7", "count identities (sidecar = files = manifest = inventory)", errs);
}

// ---- G9 / G10 prefab field sanity ------------------------------------------------------------------
{
  const g9 = prefabs.filter((p) => !["none", "soft", "hard"].includes(p.gameplay?.cover?.type)).map((p) => `prefab ${p.prefabId}: cover '${p.gameplay?.cover?.type}'`);
  const g10 = [];
  for (const p of prefabs) {
    if (!(p.spatial?.heightM >= 0)) g10.push(`prefab ${p.prefabId}: heightM ${p.spatial?.heightM}`);
    const he = p.spatial?.halfExtentsM;
    if (he && !(he.x >= 0 && he.y >= 0 && he.z >= 0)) g10.push(`prefab ${p.prefabId}: negative halfExtentsM`);
  }
  gate("G9", "gameplay.cover.type enum", g9);
  gate("G10", "spatial positive (heightM, halfExtentsM)", g10);
}

// ---- G11 raw <-> catalog parity (phase-scoped on both sides) ---------------------------------------
{
  const errs = [];
  if (rawPhaseCount !== committedPhaseCount) errs.push(`raw phase-filtered count ${rawPhaseCount} != committed phase-kind instances ${committedPhaseCount}`);
  gate("G11", `raw <-> catalog count parity for ${phase} filter`, errs);
}

// ---- P1 gates (phase scope = kind=building subset of the cumulative catalog) -----------------------
if (phase === "P1_buildings") {
  const buildings = prefabs.filter((p) => p.kind === "building");
  {
    // catalog may only hold kinds the COMMITTED importPhaseMax allows (cumulative superset rule)
    const allowed = new Set(PHASE_KINDS[manifest.objects?.importPhaseMax] ?? PHASE_KINDS[phase]);
    const errs = prefabs.filter((p) => !allowed.has(p.kind)).map((p) => `prefab ${p.prefabId} kind=${p.kind} outside importPhaseMax kinds`);
    if (buildings.length === 0) errs.push("no kind=building prefabs in catalog");
    gate("P1-1", "building prefabs present; catalog kinds within committed importPhaseMax", errs);
  }

  {
    const hard = buildings.filter((p) => p.gameplay.cover.type === "hard" || p.tags?.includes("ruin-open"));
    const pct = buildings.length ? hard.length / buildings.length : 1;
    gate("P1-2", "cover=hard >= 99.5% (ruin-open exceptions allowed)", pct >= 0.995 ? [] : [`only ${(pct * 100).toFixed(2)}% hard: ${buildings.filter((p) => p.gameplay.cover.type !== "hard" && !p.tags?.includes("ruin-open")).map((p) => p.resourceName).slice(0, 5).join(", ")}`]);
  }

  gate(
    "P1-3",
    "footprint or OBB volume > 0 per building prefab",
    buildings
      .filter((p) => !((p.spatial.footprintM2 ?? 0) > 0 || ((p.spatial.halfExtentsM?.x ?? 0) * (p.spatial.halfExtentsM?.y ?? 0) * (p.spatial.halfExtentsM?.z ?? 0)) > 0))
      .map((p) => `prefab ${p.prefabId} ${p.resourceName}`),
  );

  {
    // P1-4 — K=32 deterministic anchors: sort by (resourceName, x, z), evenly spaced, then force in
    // the global min-x/max-x rows and up to 4 boundary-adjacent rows (x%512<1 or z%512<1).
    const K = 32;
    const pool = [...anchorPool].sort((a, b) => (a.resourceName < b.resourceName ? -1 : a.resourceName > b.resourceName ? 1 : a.x - b.x || a.z - b.z));
    let errs = [];
    if (pool.length === 0) {
      errs = ["no building rows in staged raw"];
    } else {
      const picks = new Map();
      for (let i = 0; i < K; i++) picks.set(Math.round((i * (pool.length - 1)) / (K - 1)), true);
      const byX = [...pool].sort((a, b) => a.x - b.x);
      picks.set(pool.indexOf(byX[0]), true);
      picks.set(pool.indexOf(byX[byX.length - 1]), true);
      let boundary = 0;
      for (const [i, r] of pool.entries()) {
        if (boundary >= 4) break;
        if (round2(r.x) % CHUNK_SIZE_M < 1 || round2(r.z) % CHUNK_SIZE_M < 1) {
          picks.set(i, true);
          boundary++;
        }
      }
      const anchors = [...picks.keys()].sort((a, b) => a - b).map((i) => pool[i]);
      errs = checkAnchors({ anchors, prefabs, getChunk: loadChunk, chunkSizeM: CHUNK_SIZE_M, worldSizeM });
    }
    gate("P1-4", `K=32 anchor sample <= 2 m via committed chunks (${Math.min(32 + 6, anchorPool.length)} anchors)`, errs);
  }

  {
    const errs = [];
    const classes = Object.keys(inventory.byBuildingClass ?? {});
    if (classes.length === 0) errs.push("byBuildingClass empty");
    const unknown = inventory.byBuildingClass?.unknown?.instances ?? 0;
    const total = inventory.byKind.building.instances || 1; // building share, not cumulative total
    if (unknown / total >= 0.005) errs.push(`byBuildingClass.unknown ${unknown}/${total} >= 0.5%`);
    gate("P1-6", "byBuildingClass populated; unknown < 0.5% of building instances", errs);
  }
}

// ---- PH-P2 gates + D (density) + F (forest regions) — T-090.3.2 ------------------------------------
if (densityPhase) {
  const treePrefabs = prefabs.filter((p) => p.kind === "tree");
  const treeRows = []; // committed tree instances with class (D2 tree channel + F6 re-derivation)
  for (const rows of rowsByKey.values()) {
    for (const row of rows) {
      const p = prefabs[row[0]];
      if (p?.kind === "tree") treeRows.push({ x: row[1], y: row[2], class: p.class });
    }
  }

  {
    const badKinds = prefabs.filter((p) => !phaseKinds.has(p.kind)).map((p) => `prefab ${p.prefabId} kind=${p.kind}`);
    const errs = [...badKinds];
    if (treePrefabs.length === 0) errs.push("no kind=tree prefabs in catalog");
    if ((committedKindCounts.get("building") ?? 0) === 0) errs.push("cumulative rule broken: 0 building instances in P2 catalog");
    gate("PH-P2-1", "cumulative P1+P2 catalog; kinds subset {building, tree}; trees present", errs);
  }

  gate(
    "PH-P2-2",
    "tree prefabs cover=soft (dead exception)",
    treePrefabs
      .filter((p) => p.gameplay.cover.type !== "soft" && p.class !== "dead")
      .map((p) => `prefab ${p.prefabId} ${p.resourceName} cover=${p.gameplay.cover.type}`),
  );

  {
    const tall = treePrefabs.filter((p) => (p.spatial?.heightM ?? 0) >= 2);
    const pct = treePrefabs.length ? tall.length / treePrefabs.length : 0;
    gate("PH-P2-3", "heightM >= 2 for >= 95% of tree prefabs", pct >= 0.95 ? [] : [`only ${(pct * 100).toFixed(2)}% >= 2 m`]);
  }

  {
    const rawTree = rawKindCounts.get("tree") ?? 0;
    const committedTree = committedKindCounts.get("tree") ?? 0;
    gate("PH-P2-4", "G11 count conservation for kind=tree only", rawTree === committedTree ? [] : [`raw tree count ${rawTree} != committed tree instances ${committedTree}`]);
  }

  // ---- density recompute (shared by PH-P2-5 / D1 / D2) ----
  const treeAcc = accumulateCorners(treeRows, worldSizeM);
  const rockAcc = accumulateCorners(rawRockRows, worldSizeM);

  {
    const sum = sumGrid(treeAcc.grid);
    const committedTree = committedKindCounts.get("tree") ?? 0;
    gate("PH-P2-5", "density insert identity (sum of global tree corners = tree instances)", sum === committedTree ? [] : [`corner sum ${sum} != tree instances ${committedTree}`]);
  }

  {
    // D1 — exact file inventory + header contract; D2 — byte-identical recompute
    const d1 = [];
    const d2 = [];
    const densityDir = join(objectsDir, "density");
    const gridCells = Math.round(worldSizeM / CHUNK_SIZE_M);
    const expected = new Set();
    for (let cy = 0; cy < gridCells; cy++) for (let cx = 0; cx < gridCells; cx++) expected.add(chunkKey(cx, cy));
    const onDisk = existsSync(densityDir) ? readdirSync(densityDir).filter((f) => f.endsWith(".bin")) : [];
    for (const f of onDisk) {
      if (!expected.has(f.replace(".bin", ""))) d1.push(`unexpected density file ${f}`);
    }
    for (const key of expected) {
      const p = join(densityDir, `${key}.bin`);
      if (!existsSync(p)) {
        d1.push(`missing density file ${key}.bin`);
        continue;
      }
      const buf = readFileSync(p);
      if (buf.length !== TBDD_FILE_BYTES) {
        d1.push(`${key}.bin: ${buf.length} bytes, want ${TBDD_FILE_BYTES}`);
        continue;
      }
      let dec;
      try {
        dec = decodeTBDD(buf);
      } catch (e) {
        d1.push(`${key}.bin: ${e.message}`);
        continue;
      }
      if (dec.version !== TBDD_VERSION || dec.cellM !== DENSITY_CELL_M || dec.cols !== DENSITY_COLS || dec.rows !== DENSITY_ROWS || dec.channelCount !== DENSITY_CHANNELS.length) {
        d1.push(`${key}.bin: header ${JSON.stringify({ v: dec.version, cellM: dec.cellM, cols: dec.cols, rows: dec.rows, ch: dec.channelCount })} mismatch`);
        continue;
      }
      const [cx, cy] = key.split("_").map(Number);
      const rebuilt = encodeTBDD([
        sliceChunkCorners(treeAcc.grid, treeAcc.size, cx, cy),
        sliceChunkCorners(rockAcc.grid, rockAcc.size, cx, cy),
      ]);
      if (!buf.equals(rebuilt)) d2.push(`${key}.bin differs from recompute (committed chunks + raw rocks)`);
    }
    gate("D1", `density files complete (${expected.size} cells), TBDD header + size exact`, d1);
    gate("D2", "density byte-identical to recompute from committed chunks + staged raw rocks", d2);
  }

  {
    // F1 / F2 / F6 — forest regions
    const regionsPath = join(objectsDir, "forest-regions.json.gz");
    const f1 = [];
    const f2 = [];
    const f6 = [];
    if (!existsSync(regionsPath)) {
      f1.push("objects/forest-regions.json.gz missing");
      gate("F1", "forest regions present + rows schema-valid", f1);
    } else {
      const doc = JSON.parse(gunzipSync(readFileSync(regionsPath)).toString("utf8"));
      for (const [i, r] of doc.regions.entries()) {
        if (!vRegion(r)) f1.push(`region[${i}] ${r.id}: ${JSON.stringify(vRegion.errors?.[0])}`);
      }
      gate("F1", `forest regions present + ${doc.regions.length} rows schema-valid`, f1);

      const regionTreeSum = doc.regions.reduce((s, r) => s + (r.treeCount ?? 0), 0);
      const invTree = inventory.byKind.tree.instances;
      const invRegion = inventory.byRegionKind?.forest;
      if (!invRegion) f2.push("inventory.byRegionKind.forest missing");
      else {
        if (invRegion.treeCount !== regionTreeSum) f2.push(`inventory forest.treeCount ${invRegion.treeCount} != regions file sum ${regionTreeSum}`);
        if (invRegion.count !== doc.regions.length) f2.push(`inventory forest.count ${invRegion.count} != regions ${doc.regions.length}`);
      }
      if (regionTreeSum + (inventory.unassignedTrees ?? 0) !== invTree) {
        f2.push(`F2 identity broken: ${regionTreeSum} + ${inventory.unassignedTrees} != byKind.tree.instances ${invTree}`);
      }
      if (invTree !== (committedKindCounts.get("tree") ?? 0)) {
        f2.push(`inventory tree instances ${invTree} != committed tree rows ${committedKindCounts.get("tree")}`);
      }
      gate("F2", "forest.treeCount + unassignedTrees = byKind.tree.instances (exact)", f2);

      const redo = deriveForestRegions(treeRows, { worldSizeM, terrainId: terrain });
      if (JSON.stringify(redo.regions) !== JSON.stringify(doc.regions)) f6.push("re-derived regions differ from committed rings/aggregates");
      if (redo.unassignedTrees !== (inventory.unassignedTrees ?? -1)) f6.push(`re-derived unassignedTrees ${redo.unassignedTrees} != inventory ${inventory.unassignedTrees}`);
      gate("F6", "Path B derivation reproducible from committed chunk tree instances", f6);
    }
  }
}

// ---- roads (Q1 pulled forward) -----------------------------------------------------------------------
{
  const errs = [];
  if (roadsDoc.roadSegments.length === 0) errs.push("roads.json.gz has 0 segments");
  for (const [i, s] of roadsDoc.roadSegments.entries()) {
    if (s.points.length < 2) errs.push(`segment ${i} ${s.id}: < 2 points`);
  }
  gate("R-P1", "roads present (segments > 0, polylines >= 2 points)", errs);
}

// ---- size guard ----------------------------------------------------------------------------------------
gate(
  "SIZE",
  `chunk gz aggregate <= ${MAX_CHUNK_AGGREGATE_BYTES / 1024 / 1024} MB (forces LFS decision before P2)`,
  chunkAggregateBytes <= MAX_CHUNK_AGGREGATE_BYTES ? [] : [`aggregate ${(chunkAggregateBytes / 1024 / 1024).toFixed(1)} MB`],
);

// ---- E6 / G4 / I6 determinism: double scratch build + committed byte-compare ---------------------------
// Rebuild at the COMMITTED importPhaseMax, not the requested verify phase — a P1 re-verify on a P2
// catalog must byte-compare against a P2 rebuild (cumulative phases).
{
  const errs = [];
  const rebuildPhase = manifest.objects?.importPhaseMax ?? phase;
  const here = dirname(fileURLToPath(import.meta.url));
  const s1 = mkdtempSync(join(tmpdir(), "tbd-vp1-"));
  const s2 = mkdtempSync(join(tmpdir(), "tbd-vp2-"));
  try {
    for (const out of [s1, s2]) {
      execFileSync(process.execPath, [join(here, "build-world-objects.mjs"), "--terrain", terrain, "--phase", rebuildPhase, "--out", out], { stdio: "pipe" });
      execFileSync(process.execPath, [join(here, "build-roads-from-topo.mjs"), "--terrain", terrain, "--out", out], { stdio: "pipe" });
    }
    const listFiles = (dir, base = dir) => {
      const acc = [];
      for (const e of readdirSync(dir, { withFileTypes: true })) {
        const p = join(dir, e.name);
        if (e.isDirectory()) acc.push(...listFiles(p, base));
        else acc.push(p.slice(base.length + 1));
      }
      return acc.sort();
    };
    const f1 = listFiles(join(s1, "objects"));
    const f2 = listFiles(join(s2, "objects"));
    if (f1.join() !== f2.join()) errs.push("scratch builds produced different file sets");
    for (const rel of f1) {
      if (!readFileSync(join(s1, "objects", rel)).equals(readFileSync(join(s2, "objects", rel)))) errs.push(`nondeterministic: ${rel}`);
      const committed = join(objectsDir, rel);
      if (!existsSync(committed)) errs.push(`committed missing: objects/${rel}`);
      else if (!readFileSync(join(s1, "objects", rel)).equals(readFileSync(committed))) errs.push(`committed stale vs rebuild: objects/${rel}`);
    }
  } catch (e) {
    errs.push(`scratch build failed: ${String(e).slice(0, 200)}`);
  } finally {
    rmSync(s1, { recursive: true, force: true });
    rmSync(s2, { recursive: true, force: true });
  }
  gate("E6", "determinism — double scratch build byte-identical AND committed artifacts current (G4 + I6)", errs);
}

// ---- report ----------------------------------------------------------------------------------------------
let failures = 0;
for (const g of gates) {
  if (g.errCount === 0) console.log(`  PASS  ${g.id} — ${g.label}`);
  else {
    failures += g.errCount;
    console.log(`  FAIL  ${g.id} — ${g.label} (${g.errCount} error(s))`);
    for (const e of g.errs) console.log(`        ${e}`);
  }
}
if (failures) {
  console.error(`\nmap-verify-phase: FAIL — ${terrain} ${phase} (${failures} error(s))`);
  process.exit(1);
}
console.log(
  `\nmap-verify-phase: OK — ${terrain} ${phase} (${prefabs.length} prefabs, ${actualInstanceCount} instances, ${rowsByKey.size} chunks, ${roadsDoc.roadSegments.length} road segments, chunk gz ${(chunkAggregateBytes / 1024).toFixed(0)} KB)`,
);
