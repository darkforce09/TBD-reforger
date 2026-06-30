// SIZE-1/3 (CODING_STANDARDS.md §8): warn on files >600 lines, fail on >1000 unless
// allowlisted. SIZE-2 (tactical-map MC-perf glob) is fully exempt; SIZE-3 names standing
// debt (exact path). Reads .coding-standards-allowlist.yaml with a tiny hand-rolled parse
// (no npm dep) so the backend CI job runs it without `npm ci`.
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const root = process.cwd();
const al = readFileSync(join(root, ".coding-standards-allowlist.yaml"), "utf8").split("\n");
const size2 = [];
const size3 = [];
let rule = null;
for (const line of al) {
  const r = line.match(/rule:\s*(SIZE-\d)/);
  if (r) { rule = r[1]; continue; }
  const p = line.match(/path:\s*(\S+)/);
  if (!p) continue;
  if (rule === "SIZE-2") size2.push(p[1]);
  else if (rule === "SIZE-3") size3.push(p[1]);
}
const isSize2 = (rel) => size2.some((g) => rel.startsWith(g.replace(/\/\*\*.*$/, "")));
const isSize3 = (rel) => size3.includes(rel);

const EXCL = new Set(["node_modules", "dist", "build", ".git", "coverage"]);
const EXT = new Set([".go", ".ts", ".tsx"]);
function* walk(dir) {
  for (const e of readdirSync(dir, { withFileTypes: true })) {
    if (e.isDirectory()) {
      if (!EXCL.has(e.name)) yield* walk(join(dir, e.name));
    } else if (EXT.has(e.name.slice(e.name.lastIndexOf(".")))) {
      yield join(dir, e.name);
    }
  }
}

let warns = 0;
let fails = 0;
for (const f of walk(join(root, "apps/website"))) {
  const rel = f.slice(root.length + 1);
  if (isSize2(rel)) continue;
  const n = readFileSync(f, "utf8").split("\n").length;
  if (n > 1000) {
    if (!isSize3(rel)) { console.error(`SIZE-3: ${rel} is ${n} lines (>1000, not allowlisted)`); fails++; }
  } else if (n > 600) {
    console.warn(`SIZE-1 warn: ${rel} is ${n} lines (>600)`);
    warns++;
  }
}
console.log(`file-length: ${warns} warning(s), ${fails} violation(s).`);
process.exit(fails > 0 ? 1 : 0);
