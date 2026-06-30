// N10 single-source gate: the tile cache/storage budget table must carry identical canonical rows in
// both basemap_dual_view + terrain_export_pipeline, and no t090 doc may restate a conflicting budget.
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = resolve(root, "..", "..");
const specDir = join(repoRoot, "docs", "specs", "Mission_Creator_Architecture");
// Dash-agnostic: normalize figure/en/em dashes to a plain hyphen so the check is robust.
const norm = (p) => readFileSync(join(specDir, p), "utf8").replace(/[‒-―]/g, "-");

const canonical = ["200-400 MB", "400-800 MB", "512 tiles", "Max concurrent tile fetches", "one basemap pyramid"];
const forbidden = ["1.6 GB", "200-800 MB"]; // old conflicting budgets

const errors = [];
for (const f of ["t090_basemap_dual_view.md", "t090_terrain_export_pipeline.md"]) {
  const text = norm(f);
  for (const row of canonical) if (!text.includes(row)) errors.push(`${f}: N10 row missing "${row}"`);
}
for (const f of readdirSync(specDir).filter((x) => /^t090.*\.md$/.test(x))) {
  const text = norm(f);
  for (const bad of forbidden) if (text.includes(bad)) errors.push(`${f}: restates conflicting tile budget "${bad}" (N10 is single source)`);
}

if (errors.length) {
  console.error("verify-n10-tile-budget: FAIL");
  for (const e of errors) console.error(`  ${e}`);
  process.exit(1);
}
console.log("verify-n10-tile-budget: OK (N10 tile-budget single-source across basemap + pipeline)");
