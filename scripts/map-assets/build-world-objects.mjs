#!/usr/bin/env node
// T-090.3.1 — catalog-v1 world-object build: staged raw JSONL -> committed objects/ artifacts.
// T-090.3.2 — adds the P2 tree phase artifacts from the same pass: TBDD corner-density grids
//             (objects/density/{cx}_{cy}.bin, lib/density-grid.mjs) + Path B derived-hull forest
//             regions (objects/forest-regions.json.gz, lib/forest-regions.mjs).
//
// Single streaming pass over staging/export/raw-entities.jsonl (streamRawEntities — full Everon is
// ~1M rows, never readFileSync), then:
//   objects/prefabs.json.gz            {schemaVersion, terrainId, prefabs[]} — deduped rows sorted
//                                      by resourceName; prefabId = array index (G4 determinism)
//   objects/chunks/{cx}_{cy}.json.gz   {instances: [[prefabId, x, y, z, rotationDeg], ...]}
//                                      all-number 5-tuples, sorted by (x, y, prefabId)
//   objects/chunks/manifest.json       {chunkSizeM, cells: [{cx, cy, path, instanceCount}]}
//   objects/density/{cx}_{cy}.bin      (P2+) TBDD tree/rock corner densities — ALL grid cells
//                                      written unconditionally; rock channel from classified raw
//                                      rows (rocks are density aggregates, NOT imported instances)
//   objects/forest-regions.json.gz     (P2+) derived-hull forest regions; F2 identity holds by
//                                      construction (every tree -> a region or unassignedTrees)
//   objects/type-inventory.json        census of the EXPORTED CATALOG (phase scope) so I1/I7 hold;
//                                      raw-side unclassified types -> needsReview
//   manifest.json objects.* patch      (--patch-manifest only — skipped on --out scratch builds)
//   .ai/artifacts/map_export_<t>.json  fullExport.objects + fullExport.phases.<Pn> append
//                                      (--ops-log only)
//
// Conventions (plan decisions 2/4/7 — MUST agree with scripts/map-assets/lib/anchor-check.mjs,
// which re-implements them independently for P1-4):
//   remap:      map.x = engine.x, map.y = engine.z, map.z = engine.y, rotationDeg = headingDeg
//   rounding:   coords + rotation rounded to 2 dp AT INGEST, partition computed on ROUNDED values
//               (round-then-partition, or a 511.9999 row would file into cell 0 while its stored
//               512.0 value partitions to cell 1 -> G6 drift)
//   partition:  cell = clamp(floor(coord / 512), 0, cells-1); out-of-bounds origins dropped+counted
//   timestamps: generatedAt/exportedAt = staged-meta.json stagedAt (never wall clock — I6/E6)
//   noPrefab:   rows with resourceName === "" are excluded from catalog/census byKind/G11 by rule;
//               counted (+ top classNames) for the ops log
//
// Usage: node build-world-objects.mjs --terrain <id> --phase P1_buildings
//          [--out /abs/scratch-dir]   write objects/ tree under this dir instead of the terrain dir
//          [--patch-manifest]         patch packages/map-assets/<t>/manifest.json objects block
//          [--ops-log]                append fullExport.objects to .ai/artifacts/map_export_<t>.json
import { existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { gzipSync } from "node:zlib";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createClassifier, loadRules, streamRawEntities } from "./lib/classify-prefab.mjs";
import { cellOf, chunkKey } from "./lib/anchor-check.mjs";
import {
  DENSITY_CELL_M,
  accumulateCorners,
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
const CHUNK_SIZE_M = 512;
// Cumulative kind filter per import phase (t090_phased_object_import.md). Phases stay cumulative:
// P2 re-exports P1 buildings alongside trees (prefabIds renumber over the combined sorted set —
// legal, no consumer pins ids across phases; G4 = stable across re-export of the SAME phase).
const PHASE_KINDS = {
  P1_buildings: ["building"],
  // T-090.3.3: water (piers/docks — walkable hard structures) imports alongside P2; props/utility
  // stay out until their own phases.
  P2_trees: ["building", "tree", "water"],
  P3_vegetation: ["building", "tree", "water", "vegetation"],
  P4_rocks: ["building", "tree", "water", "vegetation", "rock"],
  // T-152.4: fence props (kind=prop, class=fence) for cartographic strip lane.
  P5_props: ["building", "tree", "water", "vegetation", "rock", "prop"],
};
const PHASE_ORDER = Object.keys(PHASE_KINDS);

if (!terrain || !phase) {
  console.error("build-world-objects: --terrain <id> --phase <Pn> required");
  process.exit(1);
}
if (!PHASE_KINDS[phase]) {
  console.error(`build-world-objects: phase '${phase}' not implemented in this slice (have: ${Object.keys(PHASE_KINDS).join(", ")})`);
  process.exit(1);
}
const phaseKinds = new Set(PHASE_KINDS[phase]);

const registry = JSON.parse(readFileSync(join(repoRoot, "packages", "map-assets", "terrain-registry.json"), "utf8"));
const terrainRow = registry.terrains.find((t) => t.terrainId === terrain);
if (!terrainRow) {
  console.error(`build-world-objects: terrain '${terrain}' not in terrain-registry.json`);
  process.exit(1);
}
const [minX, minY, maxX, maxY] = terrainRow.worldBoundsM;
if (minX !== 0 || minY !== 0 || maxX !== maxY) {
  console.error(`build-world-objects: worldBoundsM ${JSON.stringify(terrainRow.worldBoundsM)} unsupported (expect square, origin 0)`);
  process.exit(1);
}
const worldSizeM = maxX;

const terrainDir = join(repoRoot, "packages", "map-assets", terrain);
const stagingDir = join(terrainDir, "staging", "export");
const rawPath = join(stagingDir, "raw-entities.jsonl");
const exportMetaPath = join(stagingDir, "export-meta.json");
const stampPath = join(stagingDir, "staged-meta.json");
const outBase = arg("--out") ?? terrainDir;
const objectsDir = join(outBase, "objects");
const chunksDir = join(objectsDir, "chunks");

for (const p of [rawPath, exportMetaPath, stampPath]) {
  if (!existsSync(p)) {
    console.error(`build-world-objects: missing ${p} — stage the Workbench export first (copy-world-export-profile.mjs --full)`);
    process.exit(2);
  }
}
const exportMeta = JSON.parse(readFileSync(exportMetaPath, "utf8"));
const stamp = JSON.parse(readFileSync(stampPath, "utf8"));
const stagedAt = stamp.stagedAt;

const round2 = (v) => Math.round(v * 100) / 100;
const normHeading = (h) => round2(((h % 360) + 360) % 360);

// ---- single streaming pass ---------------------------------------------------------------------
const classify = createClassifier();
const rawCensus = new Map(); // resourceName -> { count, kind, class, matched }
const noPrefab = { count: 0, classNames: new Map() };
let outOfBounds = 0;
const kept = []; // phase-kind rows: { resourceName, kind, class, x, y, z, rot } (map space, rounded)
// T-090.3.2 — the TBDD rock channel reads classified kind=rock RAW rows (rocks are density
// aggregates, not phase-imported instances; P4 imports them). Collected only on density phases.
const densityPhase = phaseKinds.has("tree");
const rockRows = []; // { x, y } rounded, in-bounds
let rockOutOfBounds = 0;

// T-090.3.3 — measured prefab OBBs: the raw rows carry per-entity engine halfExtentsM
// ([x, y-up, z-north]); collect a few samples per phase-kind resourceName and take the per-axis
// median so prefab spatial reflects the real footprint (barns big, sheds small) instead of the
// rule-template constants. Rule spatial stays the fallback for degenerate bounds (composition
// entities export [0,0,0]).
const HE_SAMPLE_CAP = 9;
const heSamples = new Map(); // resourceName -> [ [ex, ey, ez], ... ]

const { lineCount } = await streamRawEntities(rawPath, (row) => {
  const rn = typeof row.resourceName === "string" ? row.resourceName : "";
  if (rn === "") {
    noPrefab.count++;
    const cn = row.className ?? "?";
    noPrefab.classNames.set(cn, (noPrefab.classNames.get(cn) ?? 0) + 1);
    return;
  }
  const cls = classify(rn);
  const c = rawCensus.get(rn);
  if (c) c.count++;
  else rawCensus.set(rn, { count: 1, kind: cls.kind, class: cls.class, matched: cls.matched });

  if (densityPhase && cls.kind === "rock" && !phaseKinds.has("rock")) {
    const rx = round2(row.x);
    const ry = round2(row.z);
    if (rx < 0 || rx > worldSizeM || ry < 0 || ry > worldSizeM) rockOutOfBounds++;
    else rockRows.push({ x: rx, y: ry });
    return; // rock density channel only before P4 — instances once kind=rock is in phase
  }
  if (!phaseKinds.has(cls.kind)) return;
  // P5: skip composition/FX props without Enfusion resource GUIDs (schema G1).
  if (cls.class === "composition" || cls.class === "buildingpart") return;
  if (!/^\{[0-9A-F]{16}\}/.test(rn)) return;
  // S6 label-swap compat: the T-090.3.0 spike JSONL carries the real heading in "pitchDeg"
  // (GetAngles()[1]); the T-090.3.1 plugin emits it as "headingDeg". Prefer the fixed name.
  const heading = row.headingDeg ?? row.pitchDeg ?? 0;
  const x = round2(row.x);
  const y = round2(row.z); // map.y = engine z (north)
  if (x < 0 || x > worldSizeM || y < 0 || y > worldSizeM) {
    outOfBounds++;
    return;
  }
  kept.push({ resourceName: rn, kind: cls.kind, class: cls.class, x, y, z: round2(row.y), rot: normHeading(heading) });
  const he = row.halfExtentsM;
  if (Array.isArray(he) && he.length === 3 && he.every((v) => Number.isFinite(v) && v >= 0)) {
    let s = heSamples.get(rn);
    if (!s) heSamples.set(rn, (s = []));
    if (s.length < HE_SAMPLE_CAP) s.push(he);
  }
});

if (typeof exportMeta.keptCount === "number" && exportMeta.keptCount !== lineCount) {
  console.error(`build-world-objects: FATAL — raw line count ${lineCount} != export-meta keptCount ${exportMeta.keptCount} (truncated staging?)`);
  process.exit(1);
}

// ---- prefab table (deduped, sorted by resourceName — G4) ----------------------------------------
const rules = loadRules();
const phasePrefabNames = [...new Set(kept.map((k) => k.resourceName))].sort();
const prefabIdByName = new Map(phasePrefabNames.map((n, i) => [n, i]));

const labelOf = (rn) => basename(rn).replace(/\.et$/, "");

// Measured spatial (T-090.3.3): per-axis median of the sampled engine halfExtents, remapped to
// map axes (map x = engine x, map y = engine z/north, vertical = engine y/up — same remap as
// positions). Degenerate medians (any axis ≤ 1 cm) fall back to the rule template.
const median = (vals) => {
  const s = [...vals].sort((a, b) => a - b);
  return s[Math.floor(s.length / 2)];
};
const measuredSpatial = (rn, rule) => {
  const samples = heSamples.get(rn);
  if (!samples || samples.length === 0) return rule.spatial;
  const ex = median(samples.map((s) => s[0]));
  const eyUp = median(samples.map((s) => s[1]));
  const ezNorth = median(samples.map((s) => s[2]));
  if (ex <= 0.01 || eyUp <= 0.01 || ezNorth <= 0.01) return rule.spatial;
  const hx = round2(ex);
  const hy = round2(ezNorth);
  const hv = round2(eyUp);
  return {
    model: "obb",
    pivot: rule.spatial?.pivot ?? "center",
    halfExtentsM: { x: hx, y: hy, z: hv },
    heightM: round2(2 * hv),
    footprintM2: round2(4 * hx * hy),
  };
};

const prefabs = phasePrefabNames.map((rn, i) => {
  const cls = classify(rn);
  const rule = cls.rule;
  const row = {
    prefabId: i,
    resourceName: rn,
    kind: cls.kind,
    class: cls.class,
    label: labelOf(rn),
    ai: {
      summary: rule.ai.summary,
      taxonomyPath: rule.ai.taxonomyPath,
      classificationSource: "rules-v1/prefab-name",
      confidence: rule.ai.confidence ?? 0.5,
      needsReview: !cls.matched,
    },
    spatial: measuredSpatial(rn, rule),
    gameplay: rule.gameplay,
  };
  if (rule.render) row.render = rule.render;
  if (rule.tags) row.tags = rule.tags;
  return row;
});

// ---- chunk partition (round-then-partition on stored values) ------------------------------------
const chunks = new Map(); // "cx_cy" -> rows
for (const k of kept) {
  const cx = cellOf(k.x, CHUNK_SIZE_M, worldSizeM);
  const cy = cellOf(k.y, CHUNK_SIZE_M, worldSizeM);
  const key = chunkKey(cx, cy);
  let list = chunks.get(key);
  if (!list) chunks.set(key, (list = []));
  list.push([prefabIdByName.get(k.resourceName), k.x, k.y, k.z, k.rot]);
}
for (const list of chunks.values()) {
  list.sort((a, b) => a[1] - b[1] || a[2] - b[2] || a[0] - b[0]);
}
const sortedChunkKeys = [...chunks.keys()].sort((a, b) => {
  const [ax, ay] = a.split("_").map(Number);
  const [bx, by] = b.split("_").map(Number);
  return ax - bx || ay - by;
});

// ---- write artifacts ----------------------------------------------------------------------------
rmSync(chunksDir, { recursive: true, force: true });
mkdirSync(chunksDir, { recursive: true });

const gz = (obj) => gzipSync(Buffer.from(JSON.stringify(obj)), { level: 9 });
writeFileSync(join(objectsDir, "prefabs.json.gz"), gz({ schemaVersion: "1.0.0", terrainId: terrain, prefabs }));

const cells = [];
for (const key of sortedChunkKeys) {
  const [cx, cy] = key.split("_").map(Number);
  const rel = `objects/chunks/${key}.json.gz`;
  writeFileSync(join(chunksDir, `${key}.json.gz`), gz({ instances: chunks.get(key) }));
  cells.push({ cx, cy, path: rel, instanceCount: chunks.get(key).length });
}
writeFileSync(
  join(chunksDir, "manifest.json"),
  `${JSON.stringify({ chunkSizeM: CHUNK_SIZE_M, cells }, null, 2)}\n`,
);

// ---- density grids + forest regions (P2+, T-090.3.2) --------------------------------------------
const densityDir = join(objectsDir, "density");
let densitySummary = null;
let regionsResult = null;
if (densityPhase) {
  const treeRows = kept.filter((k) => k.kind === "tree");

  const treeAcc = accumulateCorners(treeRows, worldSizeM);
  const rockAcc = accumulateCorners(rockRows, worldSizeM);
  const treeCornerSum = sumGrid(treeAcc.grid);
  const rockCornerSum = sumGrid(rockAcc.grid);
  // PH-P2-5 identity: every instance lands in exactly one global corner window.
  if (treeCornerSum !== treeRows.length) {
    console.error(`build-world-objects: FATAL — density tree corner sum ${treeCornerSum} != tree instances ${treeRows.length}`);
    process.exit(1);
  }
  if (rockCornerSum !== rockRows.length) {
    console.error(`build-world-objects: FATAL — density rock corner sum ${rockCornerSum} != rock rows ${rockRows.length}`);
    process.exit(1);
  }

  rmSync(densityDir, { recursive: true, force: true });
  mkdirSync(densityDir, { recursive: true });
  const gridCells = Math.round(worldSizeM / CHUNK_SIZE_M);
  let densityBytes = 0;
  for (let cy = 0; cy < gridCells; cy++) {
    for (let cx = 0; cx < gridCells; cx++) {
      const buf = encodeTBDD([
        sliceChunkCorners(treeAcc.grid, treeAcc.size, cx, cy),
        sliceChunkCorners(rockAcc.grid, rockAcc.size, cx, cy),
      ]);
      writeFileSync(join(densityDir, `${chunkKey(cx, cy)}.bin`), buf);
      densityBytes += buf.length;
    }
  }
  densitySummary = {
    cellM: DENSITY_CELL_M,
    files: gridCells * gridCells,
    bytes: densityBytes,
    treeCornerSum,
    rockCornerSum,
    rockRawRows: rockRows.length,
    rockOutOfBounds,
  };

  regionsResult = deriveForestRegions(treeRows, { worldSizeM, terrainId: terrain });
  writeFileSync(
    join(objectsDir, "forest-regions.json.gz"),
    gz({
      schemaVersion: "1.0.0",
      terrainId: terrain,
      generatedAt: stagedAt,
      ...regionsResult.params,
      regions: regionsResult.regions,
    }),
  );
} else {
  // pre-density phases never leave stale density/regions artifacts in a scratch --out dir
  rmSync(densityDir, { recursive: true, force: true });
}

// ---- census (catalog scope — I1/I7 hold; needsReview = raw unclassified types) -------------------
const zero = { prefabTypes: 0, instances: 0 };
const byKind = {
  building: { ...zero },
  tree: { ...zero },
  vegetation: { ...zero },
  rock: { ...zero },
  prop: { ...zero },
  utility: { ...zero },
  water: { ...zero },
  road: { ...zero, segments: 0 },
};
const instByPrefab = new Array(prefabs.length).fill(0);
for (const list of chunks.values()) for (const row of list) instByPrefab[row[0]]++;

// Per-class buckets split by kind (I3: byBuildingClass keys ∈ buildingClass enum, bySpeciesClass
// keys ∈ speciesClass enum — a tree prefab must never land in byBuildingClass).
const byBuildingClass = {};
const bySpeciesClass = {};
const classBucketTargetOf = (p) =>
  p.kind === "building" ? byBuildingClass : p.kind === "tree" || p.kind === "vegetation" ? bySpeciesClass : null;
for (const [i, p] of prefabs.entries()) {
  byKind[p.kind].prefabTypes++;
  byKind[p.kind].instances += instByPrefab[i];
  const target = classBucketTargetOf(p);
  if (target) {
    const bucket = (target[p.class] ??= { prefabTypes: 0, instances: 0 });
    bucket.prefabTypes++;
    bucket.instances += instByPrefab[i];
    const iz = rules.rules.find((r) => r.kind === p.kind && r.class === p.class)?.render?.importanceZoom;
    if (typeof iz === "number") bucket.importanceZoom = iz;
  }
}
const totalInstances = kept.length;

const needsReviewPrefabs = [...rawCensus.entries()]
  .filter(([, c]) => !c.matched)
  .map(([rn, c]) => ({
    resourceName: rn,
    instanceCount: c.count,
    reason: `unclassified (fallback ${c.kind}/${c.class}) — excluded from ${phase} catalog`,
  }))
  .sort((a, b) => b.instanceCount - a.instanceCount || (a.resourceName < b.resourceName ? -1 : 1));

const sortKeys = (obj) => Object.fromEntries(Object.entries(obj).sort(([a], [b]) => (a < b ? -1 : 1)));
const inventory = {
  schemaVersion: "1.0.0",
  terrainId: terrain,
  censusStatus: "partial",
  generatedAt: stagedAt,
  importPhaseMax: phase,
  sourceExportPath: "staging/export/raw-entities.jsonl",
  levels: { uniquePrefabs: prefabs.length, totalInstances },
  byKind,
  byBuildingClass: sortKeys(byBuildingClass),
  byRoadClass: {},
  bySpeciesClass: sortKeys(bySpeciesClass),
  needsReview: { prefabTypes: needsReviewPrefabs.length, prefabs: needsReviewPrefabs },
};
if (regionsResult) {
  // F2 ship identity (exact, by construction): forest.treeCount + unassignedTrees = tree instances
  inventory.byRegionKind = {
    forest: {
      count: regionsResult.regions.length,
      treeCount: regionsResult.regions.reduce((s, r) => s + r.treeCount, 0),
    },
  };
  inventory.unassignedTrees = regionsResult.unassignedTrees;
}
writeFileSync(join(objectsDir, "type-inventory.json"), `${JSON.stringify(inventory, null, 2)}\n`);

// ---- manifest patch (real terrain dir only) ------------------------------------------------------
if (argv.includes("--patch-manifest")) {
  const manifestPath = join(terrainDir, "manifest.json");
  const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  manifest.objects = {
    ...manifest.objects,
    schemaVersion: "1.0.0",
    format: "catalog-v1",
    prefabsPath: "objects/prefabs.json.gz",
    prefabCount: prefabs.length,
    instanceCount: totalInstances,
    chunksPath: "objects/chunks",
    chunkSizeM: CHUNK_SIZE_M,
    roadsPath: "objects/roads.json.gz",
    typeInventoryPath: "objects/type-inventory.json",
    importPhaseMax: phase,
    importPhaseShipped: PHASE_ORDER.slice(0, PHASE_ORDER.indexOf(phase) + 1),
    exportedAt: stagedAt,
  };
  if (densityPhase) {
    manifest.objects.regionsPath = "objects/forest-regions.json.gz";
    manifest.objects.densityPath = "objects/density";
    manifest.objects.densityCellM = DENSITY_CELL_M;
    // LOD gate snapshot for cache-busting (plan §3.4/§5 constants v2) — consumers read the live
    // table from worldmap/lodGates.ts (T-090.5.1); this block only invalidates caches on change.
    manifest.objects.lod = {
      schemaVersion: "1.0.0",
      refZoom: 3,
      gates: {
        tree: 0,
        building: -2.5,
        buildingBadge: 1,
        forestOutline: -1.5,
        forestFillMax: 1,
        vegetation: 1.5,
        rockLarge: 1,
        prop: 3,
      },
    };
  }
  writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
}

// ---- ops log append ------------------------------------------------------------------------------
const summary = {
  slice: densityPhase ? "T-090.3.2" : "T-090.3.1",
  phase,
  stagedAt,
  rawLineCount: lineCount,
  rawUniqueResourceNames: rawCensus.size,
  noPrefab: {
    count: noPrefab.count,
    topClassNames: [...noPrefab.classNames.entries()]
      .sort((a, b) => b[1] - a[1] || (a[0] < b[0] ? -1 : 1))
      .slice(0, 10)
      .map(([className, count]) => ({ className, count })),
  },
  outOfBounds,
  catalog: { prefabCount: prefabs.length, instanceCount: totalInstances, chunkCount: cells.length },
  unclassifiedRawTypes: needsReviewPrefabs.length,
};
if (argv.includes("--ops-log")) {
  const opsPath = join(repoRoot, ".ai", "artifacts", `map_export_${terrain}.json`);
  const ops = existsSync(opsPath) ? JSON.parse(readFileSync(opsPath, "utf8")) : { terrainId: terrain };
  ops.fullExport = { ...ops.fullExport, objects: summary };
  if (densityPhase) {
    // APPEND-only per-phase block (spike keys + prior phase blocks untouched)
    ops.fullExport.phases = {
      ...ops.fullExport.phases,
      [phase]: {
        slice: "T-090.3.2",
        stagedAt,
        density: densitySummary,
        forestRegions: {
          ...regionsResult.params,
          regionCount: regionsResult.regions.length,
          treeCount: regionsResult.regions.reduce((s, r) => s + r.treeCount, 0),
          unassignedTrees: regionsResult.unassignedTrees,
          denseCellCount: regionsResult.denseCellCount,
          componentCount: regionsResult.componentCount,
          keptComponentCount: regionsResult.keptComponentCount,
        },
      },
    };
  }
  writeFileSync(opsPath, `${JSON.stringify(ops, null, 2)}\n`);
}

console.log(`build-world-objects: ${terrain} ${phase} — ${JSON.stringify(summary)}`);
