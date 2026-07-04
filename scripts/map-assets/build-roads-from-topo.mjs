#!/usr/bin/env node
// T-090.3.1 — objects/roads.json.gz from the terrain's .topo (Q1: roads pulled forward into the
// first export slice). Host-side only: decode-topo.mjs (pak VFS, proven T-090.1.2.5.2) already
// yields section-1 polylines; no Workbench pass involved.
//
// Class mapping is PROVISIONAL (plan decision 5 — recorded in the ops log, not in roads.json.gz;
// the roads schema is additionalProperties:false): .topo type semantics beyond runway/river are
// unlabeled, so type 3 ("road class A", 367 recs on Eden) -> road_paved and type 5 ("road class B",
// 394 recs) -> road_dirt until the P6-P9 road-phase purity gates own the correction. No P1 gate
// reads roadClass.
//
// Determinism (G4/E6): records sorted by (type, first x, first y, vertexCount); ids assigned after
// the sort; points rounded to 2 dp; gzip level 9 (node gzip mtime=0).
//
// Coordinates: .topo x = world metres east; y = north-up IMAGE metres -> map.y = worldSizeM - y
// (same map space as the object chunks: x east, y north).
//
// Usage: node build-roads-from-topo.mjs --terrain <id> [--out /abs/scratch-dir] [--ops-log]
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { gzipSync } from "node:zlib";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { decodeTopo, TOPO_TYPES } from "./decode-topo.mjs";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const argv = process.argv.slice(2);
const arg = (flag) => {
  const i = argv.indexOf(flag);
  return i >= 0 && i + 1 < argv.length ? argv[i + 1] : undefined;
};
const terrain = arg("--terrain");
if (!terrain) {
  console.error("build-roads-from-topo: --terrain <id> required");
  process.exit(1);
}

const ROAD_CLASS_BY_TYPE = {
  [TOPO_TYPES.AIRFIELD]: "runway",
  [TOPO_TYPES.ROAD_A]: "road_paved",
  [TOPO_TYPES.ROAD_B]: "road_dirt",
};

const round2 = (v) => Math.round(v * 100) / 100;

const topo = await decodeTopo(terrain);
const roadRecords = topo.records
  .filter((r) => ROAD_CLASS_BY_TYPE[r.type] !== undefined)
  .map((r) => {
    const points = [];
    for (let i = 0; i < r.verts.length; i += 2) {
      points.push([round2(r.verts[i]), round2(topo.worldSizeM - r.verts[i + 1])]);
    }
    return { type: r.type, points };
  })
  .sort(
    (a, b) =>
      a.type - b.type ||
      a.points[0][0] - b.points[0][0] ||
      a.points[0][1] - b.points[0][1] ||
      a.points.length - b.points.length,
  );

const roadSegments = roadRecords.map((r, i) => ({
  id: `road-${terrain}-${String(i).padStart(4, "0")}`,
  roadClass: ROAD_CLASS_BY_TYPE[r.type],
  points: r.points,
}));

const doc = { schemaVersion: "1.0.0", terrainId: terrain, roadSegments };

const outBase = arg("--out") ?? join(repoRoot, "packages", "map-assets", terrain);
const objectsDir = join(outBase, "objects");
mkdirSync(objectsDir, { recursive: true });
writeFileSync(join(objectsDir, "roads.json.gz"), gzipSync(Buffer.from(JSON.stringify(doc)), { level: 9 }));

const byClass = {};
for (const s of roadSegments) byClass[s.roadClass] = (byClass[s.roadClass] ?? 0) + 1;
const summary = {
  slice: "T-090.3.1",
  source: "decode-topo section 1",
  classMappingProvisional: true,
  classByTopoType: { 0: "runway", 3: "road_paved", 5: "road_dirt" },
  segments: roadSegments.length,
  byClass,
  points: roadSegments.reduce((n, s) => n + s.points.length, 0),
};

if (argv.includes("--ops-log")) {
  const opsPath = join(repoRoot, ".ai", "artifacts", `map_export_${terrain}.json`);
  const ops = existsSync(opsPath) ? JSON.parse(readFileSync(opsPath, "utf8")) : { terrainId: terrain };
  ops.fullExport = { ...ops.fullExport, roads: summary };
  writeFileSync(opsPath, `${JSON.stringify(ops, null, 2)}\n`);
}

console.log(`build-roads-from-topo: ${terrain} — ${JSON.stringify(summary)}`);
