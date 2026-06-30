// T-090.5 glyph coverage gate. Every golden prefab render.iconKey must have a glyph manifest entry,
// and every manifest entry must point at an SVG file that exists and declares a viewBox (G2/G3).
import { existsSync, readFileSync } from "node:fs";
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

// 1. Coverage: every iconKey referenced by a golden prefab exists in the manifest.
for (const p of prefabs) {
  const key = p.render?.iconKey;
  if (key && !glyphs[key]) errors.push(`prefab ${p.prefabId}: render.iconKey '${key}' missing from glyph manifest`);
}

// 2. Each manifest glyph points at a valid SVG (file exists + has a viewBox).
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
}

if (errors.length) {
  console.error("verify-map-glyphs: FAIL");
  for (const e of errors) console.error(`  ${e}`);
  process.exit(1);
}
console.log(`verify-map-glyphs: OK (${Object.keys(glyphs).length} glyphs, all golden iconKeys covered)`);
