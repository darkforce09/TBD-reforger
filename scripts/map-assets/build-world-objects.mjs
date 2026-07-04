#!/usr/bin/env node
// T-090.3.1 — catalog-v1 world-object build: staged raw JSONL -> committed objects/ artifacts.
//
// Single streaming pass over staging/export/raw-entities.jsonl (streamRawEntities — full Everon is
// ~1M rows, never readFileSync), then:
//   objects/prefabs.json.gz            {schemaVersion, terrainId, prefabs[]} — deduped rows sorted
//                                      by resourceName; prefabId = array index (G4 determinism)
//   objects/chunks/{cx}_{cy}.json.gz   {instances: [[prefabId, x, y, z, rotationDeg], ...]}
//                                      all-number 5-tuples, sorted by (x, y, prefabId)
//   objects/chunks/manifest.json       {chunkSizeM, cells: [{cx, cy, path, instanceCount}]}
//   objects/type-inventory.json        census of the EXPORTED CATALOG (phase scope) so I1/I7 hold;
//                                      raw-side unclassified types -> needsReview
//   manifest.json objects.* patch      (--patch-manifest only — skipped on --out scratch builds)
//   .ai/artifacts/map_export_<t>.json  fullExport.objects append (--ops-log only)
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

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const argv = process.argv.slice(2);
const arg = (flag) => {
  const i = argv.indexOf(flag);
  return i >= 0 && i + 1 < argv.length ? argv[i + 1] : undefined;
};

const terrain = arg("--terrain");
const phase = arg("--phase");
const CHUNK_SIZE_M = 512;
// Cumulative kind filter per import phase (t090_phased_object_import.md). This slice implements
// P1 only; the table exists so the phase gate fails loudly instead of silently exporting nothing.
const PHASE_KINDS = {
  P1_buildings: ["building"],
};

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
const kept = []; // phase-kind rows: { resourceName, x, y, z, rot } (map space, rounded)

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

  if (!phaseKinds.has(cls.kind)) return;
  // S6 label-swap compat: the T-090.3.0 spike JSONL carries the real heading in "pitchDeg"
  // (GetAngles()[1]); the T-090.3.1 plugin emits it as "headingDeg". Prefer the fixed name.
  const heading = row.headingDeg ?? row.pitchDeg ?? 0;
  const x = round2(row.x);
  const y = round2(row.z); // map.y = engine z (north)
  if (x < 0 || x > worldSizeM || y < 0 || y > worldSizeM) {
    outOfBounds++;
    return;
  }
  kept.push({ resourceName: rn, x, y, z: round2(row.y), rot: normHeading(heading) });
});

if (typeof exportMeta.keptCount === "number" && exportMeta.keptCount !== lineCount) {
  console.error(`build-world-objects: FATAL — raw line count ${lineCount} != export-meta keptCount ${exportMeta.keptCount} (truncated staging?)`);
  process.exit(1);
}

// ---- prefab table (deduped, sorted by resourceName — G4) ----------------------------------------
const rules = loadRules();
const buildingNames = [...new Set(kept.map((k) => k.resourceName))].sort();
const prefabIdByName = new Map(buildingNames.map((n, i) => [n, i]));

const labelOf = (rn) => basename(rn).replace(/\.et$/, "");
const prefabs = buildingNames.map((rn, i) => {
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
    spatial: rule.spatial,
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
const byBuildingClass = {};
for (const p of prefabs) {
  byKind[p.kind].prefabTypes++;
  const bucket = (byBuildingClass[p.class] ??= { prefabTypes: 0, instances: 0 });
  bucket.prefabTypes++;
  const iz = rules.rules.find((r) => r.kind === "building" && r.class === p.class)?.render?.importanceZoom;
  if (typeof iz === "number") bucket.importanceZoom = iz;
}
const instByPrefab = new Array(prefabs.length).fill(0);
for (const list of chunks.values()) for (const row of list) instByPrefab[row[0]]++;
for (const [i, p] of prefabs.entries()) {
  byKind[p.kind].instances += instByPrefab[i];
  byBuildingClass[p.class].instances += instByPrefab[i];
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

const inventory = {
  schemaVersion: "1.0.0",
  terrainId: terrain,
  censusStatus: "partial",
  generatedAt: stagedAt,
  importPhaseMax: phase,
  sourceExportPath: "staging/export/raw-entities.jsonl",
  levels: { uniquePrefabs: prefabs.length, totalInstances },
  byKind,
  byBuildingClass: Object.fromEntries(Object.entries(byBuildingClass).sort(([a], [b]) => (a < b ? -1 : 1))),
  byRoadClass: {},
  bySpeciesClass: {},
  needsReview: { prefabTypes: needsReviewPrefabs.length, prefabs: needsReviewPrefabs },
};
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
    importPhaseShipped: [phase],
    exportedAt: stagedAt,
  };
  writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
}

// ---- ops log append ------------------------------------------------------------------------------
const summary = {
  slice: "T-090.3.1",
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
  writeFileSync(opsPath, `${JSON.stringify(ops, null, 2)}\n`);
}

console.log(`build-world-objects: ${terrain} ${phase} — ${JSON.stringify(summary)}`);
