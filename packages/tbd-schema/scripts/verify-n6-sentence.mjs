// N6 single-source gate: the building-geometry "normative shipped geometry" sentence must appear
// (markdown-stripped, whitespace-collapsed) in all five canonical locations. Fails on any drift.
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = resolve(root, "..", "..");
const specDir = join(repoRoot, "docs", "specs", "Mission_Creator_Architecture");
const norm = (s) => s.replace(/[`*]/g, "").replace(/\s+/g, " ");

const core = norm(
  "oriented bounding rectangle from spatial.halfExtentsM + rotationDeg. Real footprint polygon rings " +
    "are populated only when T-090.3.0 proves Enfusion footprint export; when present, polygons " +
    "supersede OBB rectangles for render.",
);

const files = [
  join(specDir, "t090_2_map_object_taxonomy.md"),
  join(specDir, "t090_5_map_object_render_layer.md"),
  join(specDir, "t090_6_geometry_placement_audit.md"),
  join(specDir, "t090_world_object_glyphs.md"),
  join(root, "schema", "map-object-prefab.schema.json"),
];

const missing = files.filter((f) => !norm(readFileSync(f, "utf8")).includes(core));
if (missing.length) {
  console.error("verify-n6-sentence: FAIL — N6 building-geometry sentence missing/drifted in:");
  for (const m of missing) console.error(`  ${m.replace(repoRoot + "/", "")}`);
  process.exit(1);
}
console.log(`verify-n6-sentence: OK (N6 sentence identical across ${files.length} locations)`);
