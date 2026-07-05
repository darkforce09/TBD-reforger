#!/usr/bin/env node
// T-090.3.1 — objects/roads.json.gz from the terrain's .topo (Q1: roads pulled forward into the
// first export slice). Host-side only: decode-topo.mjs (pak VFS, proven T-090.1.2.5.2) already
// yields section-1 polylines; no Workbench pass involved.
//
// Class mapping (T-090.3.3, supersedes the provisional T-090.3.1 mapping): G1-B established the
// .topo carries ROADS ONLY — the legacy "RIVER"/"STREAM" type names in decode-topo are wrong. The
// five type classes have engineered constant cross-widths on Eden (0: 20 m ×5 = runways ·
// 1: 12 m ×12, 19.7 km chaining the central valley = the main asphalt highway · 2: 8 m ×110,
// 57.9 km = secondary asphalt · 3: 4.5 m ×367, 133 km = gravel/country net · 5: 1.75 m ×394,
// 128 km = farm tracks/trails), which fixes the semantic mapping below. `path` has no Eden
// records. decode-topo's exported names stay untouched (frozen legacy consumers).
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
  [TOPO_TYPES.RIVER]: "highway_paved", // 12 m main asphalt (legacy constant name is wrong — see header)
  [TOPO_TYPES.STREAM]: "road_paved", // 8 m secondary asphalt
  [TOPO_TYPES.ROAD_A]: "road_dirt", // 4.5 m gravel/country roads
  [TOPO_TYPES.ROAD_B]: "track", // 1.75 m farm tracks / trails
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
  slice: "T-090.3.3",
  source: "decode-topo section 1",
  classMappingProvisional: false,
  classByTopoType: { 0: "runway", 1: "highway_paved", 2: "road_paved", 3: "road_dirt", 5: "track" },
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
