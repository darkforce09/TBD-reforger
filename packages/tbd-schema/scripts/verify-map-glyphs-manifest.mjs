// T-090.5 glyph coverage gate (GL-G1…G6, t090_world_object_glyphs.md §Mathematical verification).
// Every golden prefab render.iconKey must have a glyph manifest entry, every manifest entry must
// point at an SVG that exists and declares a viewBox (G2/G3), manifest fields must be sane (G6),
// and — since T-090.5.2 — every iconKey in a committed terrain catalog (everon prefabs.json.gz)
// must be covered too, and the built atlas (when present) must map every glyph inside its own
// power-of-two bounds (G4). Dep-free by design (T-125 portability): atlas dims come from the
// build-emitted world-glyphs.json meta, cross-checked against the webp RIFF header.
import { existsSync, readFileSync } from "node:fs";
import { gunzipSync } from "node:zlib";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = resolve(root, "..", "..");
const readJSON = (p) => JSON.parse(readFileSync(p, "utf8"));

const glyphDir = join(repoRoot, "packages", "map-assets", "glyphs");
const manifest = readJSON(join(glyphDir, "manifest.json"));
const glyphs = manifest.glyphs ?? {};
const prefabs = readJSON(join(root, "golden", "map-objects", "map-object-prefabs-sample.json"));

const errors = [];

// 1. Coverage (G2, golden): every iconKey referenced by a golden prefab exists in the manifest.
for (const p of prefabs) {
  const key = p.render?.iconKey;
  if (key && !glyphs[key]) errors.push(`prefab ${p.prefabId}: render.iconKey '${key}' missing from glyph manifest`);
}

// 1b. Coverage (G2, committed terrain catalogs): every iconKey in an exported prefab catalog
// exists in the manifest — the render layer (T-090.5.2+) resolves these at runtime.
const catalogPaths = [join(repoRoot, "packages", "map-assets", "everon", "objects", "prefabs.json.gz")];
for (const catalogPath of catalogPaths) {
  if (!existsSync(catalogPath)) continue;
  let rows;
  try {
    rows = JSON.parse(gunzipSync(readFileSync(catalogPath)).toString("utf8")).prefabs ?? [];
  } catch (e) {
    errors.push(`catalog ${catalogPath}: unreadable (${e.message})`);
    continue;
  }
  const missing = new Map();
  for (const p of rows) {
    const key = p.render?.iconKey;
    if (key && !glyphs[key]) missing.set(key, (missing.get(key) ?? 0) + 1);
  }
  for (const [key, n] of missing) {
    errors.push(`catalog everon: render.iconKey '${key}' (${n} prefabs) missing from glyph manifest`);
  }
}

// 2. Each manifest glyph points at a valid SVG (file exists + has a viewBox) (G3) and has sane
// render fields (G6: baseSizePx > 0, anchor components in [0,1]).
for (const [key, g] of Object.entries(glyphs)) {
  if (!g.svg) {
    errors.push(`glyph '${key}': no svg path`);
    continue;
  }
  const svgPath = join(glyphDir, g.svg);
  if (!existsSync(svgPath)) {
    errors.push(`glyph '${key}': svg file not found (${g.svg})`);
    continue;
  }
  const svg = readFileSync(svgPath, "utf8");
  if (!/viewBox\s*=/.test(svg)) errors.push(`glyph '${key}': ${g.svg} has no viewBox`);
  if (!/<svg[\s>]/.test(svg)) errors.push(`glyph '${key}': ${g.svg} is not a valid <svg>`);
  if (!(typeof g.baseSizePx === "number" && g.baseSizePx > 0)) {
    errors.push(`glyph '${key}': baseSizePx must be > 0 (got ${g.baseSizePx})`);
  }
  const a = g.anchor;
  if (!(Array.isArray(a) && a.length === 2 && a.every((v) => typeof v === "number" && v >= 0 && v <= 1))) {
    errors.push(`glyph '${key}': anchor must be [x,y] with components in [0,1] (got ${JSON.stringify(a)})`);
  }
}

// 3. Atlas gate (G4, when built): every manifest glyph has a rect, rects fit the declared
// canvas, canvas is power-of-two ≤ 4096², and the webp really is a RIFF/WEBP container.
const atlasJsonPath = join(glyphDir, manifest.atlas?.rects ?? "atlas/world-glyphs.json");
const atlasWebpPath = join(glyphDir, manifest.atlas?.image ?? "atlas/world-glyphs.webp");
if (existsSync(atlasJsonPath)) {
  const atlas = readJSON(atlasJsonPath);
  const { width, height } = atlas.meta ?? {};
  const isPow2 = (n) => Number.isInteger(n) && n > 0 && (n & (n - 1)) === 0;
  if (!isPow2(width) || !isPow2(height) || width > 4096 || height > 4096) {
    errors.push(`atlas: dims ${width}×${height} not power-of-two ≤ 4096²`);
  }
  for (const key of Object.keys(glyphs)) {
    const r = atlas.icons?.[key];
    if (!r) {
      errors.push(`atlas: glyph '${key}' has no rect in world-glyphs.json (rebuild: make map-glyphs-build)`);
      continue;
    }
    if (r.x < 0 || r.y < 0 || r.x + r.width > width || r.y + r.height > height) {
      errors.push(`atlas: glyph '${key}' rect exceeds ${width}×${height} bounds`);
    }
    if (!(r.anchorX >= 0 && r.anchorX <= r.width && r.anchorY >= 0 && r.anchorY <= r.height)) {
      errors.push(`atlas: glyph '${key}' anchor outside its rect`);
    }
  }
  if (!existsSync(atlasWebpPath)) {
    errors.push("atlas: world-glyphs.json present but world-glyphs.webp missing");
  } else {
    const head = readFileSync(atlasWebpPath).subarray(0, 12);
    if (head.length < 12 || head.toString("ascii", 0, 4) !== "RIFF" || head.toString("ascii", 8, 12) !== "WEBP") {
      errors.push("atlas: world-glyphs.webp is not a RIFF/WEBP file");
    }
  }
}

if (errors.length) {
  console.error("verify-map-glyphs: FAIL");
  for (const e of errors) console.error(`  ${e}`);
  process.exit(1);
}
const atlasNote = existsSync(atlasJsonPath) ? ", atlas rects verified" : ", no atlas built";
console.log(`verify-map-glyphs: OK (${Object.keys(glyphs).length} glyphs, golden + everon iconKeys covered${atlasNote})`);
