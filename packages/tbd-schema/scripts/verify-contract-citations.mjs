// Citation integrity gate (DOCUMENTATION_STANDARDS.md §3/§10). Scans the repo's code for
// cross-boundary "@contract <file>.schema.json#<json-pointer>" tags and asserts each resolves
// to a real schema under packages/tbd-schema/schema/ AND a valid RFC-6901 pointer within it.
// A renamed/removed schema definition turns a doc comment into a CI failure instead of silent rot.
//
// Exit 0 = all citations resolve; exit 1 = one or more dangling citations.
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { dirname, extname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, ".."); // packages/tbd-schema
const repoRoot = resolve(root, "..", "..");
const schemaDir = join(root, "schema");

const SCAN_DIRS = ["apps", "packages"];
const CODE_EXTS = new Set([".go", ".ts", ".tsx", ".c", ".mjs", ".js"]);
const IGNORE_DIRS = new Set(["node_modules", "dist", ".git", "build", "coverage", "vendor"]);

// Matches:  @contract <name>.schema.json[#<pointer>]   (pointer stops at whitespace, ) or ")
const TAG = /@contract\s+([A-Za-z0-9_.\-]+\.schema\.json)(#[^\s)"']*)?/g;

function* walk(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.isDirectory()) {
      if (!IGNORE_DIRS.has(entry.name)) yield* walk(join(dir, entry.name));
    } else if (CODE_EXTS.has(extname(entry.name))) {
      yield join(dir, entry.name);
    }
  }
}

const schemaCache = new Map();
function loadSchema(name) {
  if (!schemaCache.has(name)) {
    const p = join(schemaDir, name);
    schemaCache.set(name, existsSync(p) ? JSON.parse(readFileSync(p, "utf8")) : null);
  }
  return schemaCache.get(name);
}

// RFC-6901 JSON pointer resolution. "", "#", "#/" all mean the document root.
function pointerResolves(doc, pointer) {
  if (!pointer || pointer === "#" || pointer === "#/") return true;
  const path = pointer.replace(/^#/, "");
  if (!path.startsWith("/")) return false;
  let cur = doc;
  for (const raw of path.split("/").slice(1)) {
    const key = raw.replace(/~1/g, "/").replace(/~0/g, "~");
    if (cur && typeof cur === "object" && Object.prototype.hasOwnProperty.call(cur, key)) {
      cur = cur[key];
    } else {
      return false;
    }
  }
  return true;
}

let citations = 0;
const problems = [];
for (const d of SCAN_DIRS) {
  const abs = join(repoRoot, d);
  if (!existsSync(abs)) continue;
  for (const file of walk(abs)) {
    const text = readFileSync(file, "utf8");
    for (const m of text.matchAll(TAG)) {
      citations += 1;
      const [, name, pointer = ""] = m;
      const rel = file.slice(repoRoot.length + 1);
      const doc = loadSchema(name);
      if (!doc) {
        problems.push(`${rel}: @contract ${name}${pointer} -> schema/${name} not found`);
      } else if (!pointerResolves(doc, pointer)) {
        problems.push(`${rel}: @contract ${name}${pointer} -> JSON pointer not found in schema`);
      }
    }
  }
}

console.log(`Checked ${citations} @contract citation(s) across ${SCAN_DIRS.join(", ")}.`);
if (problems.length > 0) {
  console.error(`\n${problems.length} dangling citation(s):`);
  for (const p of problems) console.error(`  ${p}`);
  process.exit(1);
}
console.log("All @contract citations resolve.");
