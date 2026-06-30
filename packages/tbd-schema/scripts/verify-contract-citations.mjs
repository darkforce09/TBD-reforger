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

// --- TS-6 (CODING_STANDARDS §10) ------------------------------------------------------------
// Every exported interface / type alias in the front-end contract layer (types/, api/, hooks/)
// MUST carry an @model or @contract tag in its immediately-preceding TSDoc block — not just a
// block (that presence is TS-5/eslint), but the cross-boundary content (DOCUMENTATION_STANDARDS
// §3). Generic envelope types (e.g. `Paginated<T>`) are exempt: their contract is carried by the
// type argument, not a single model. Generated `types/contract/**` is skipped (schema codegen).
const TS_CONTRACT_DIRS = [
  "apps/website/frontend/src/types",
  "apps/website/frontend/src/api",
  "apps/website/frontend/src/hooks",
];
const TS_EXTS = new Set([".ts", ".tsx"]);
const EXPORT_DECL = /^export\s+(interface|type)\s+([A-Za-z0-9_]+)(<)?/;

function* walkTs(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = join(dir, entry.name);
    if (entry.isDirectory()) {
      if (!IGNORE_DIRS.has(entry.name) && entry.name !== "contract") yield* walkTs(full);
    } else if (TS_EXTS.has(extname(entry.name))) {
      yield full;
    }
  }
}

// The TSDoc block immediately above line `idx` (blank lines skipped), or null if there is none.
function precedingDocBlock(lines, idx) {
  let i = idx - 1;
  while (i >= 0 && lines[i].trim() === "") i--;
  if (i < 0 || !lines[i].trim().endsWith("*/")) return null;
  const end = i;
  while (i >= 0 && !lines[i].includes("/**")) i--;
  if (i < 0) return null;
  return lines.slice(i, end + 1).join("\n");
}

function checkTsContractTags() {
  const tagProblems = [];
  let checked = 0;
  for (const d of TS_CONTRACT_DIRS) {
    const abs = join(repoRoot, d);
    if (!existsSync(abs)) continue;
    for (const file of walkTs(abs)) {
      const lines = readFileSync(file, "utf8").split("\n");
      const rel = file.slice(repoRoot.length + 1);
      for (let i = 0; i < lines.length; i++) {
        const m = EXPORT_DECL.exec(lines[i]);
        if (!m) continue;
        const [, , name, generic] = m;
        if (generic) continue; // generic envelope (e.g. Paginated<T>) — contract is in its type arg
        checked += 1;
        const block = precedingDocBlock(lines, i);
        if (!block || !/@model|@contract/.test(block)) {
          tagProblems.push(`${rel}:${i + 1}: exported ${name} missing @model or @contract (TS-6)`);
        }
      }
    }
  }
  return { tagProblems, checked };
}

// --- GO-7 (CODING_STANDARDS §2): every exported HTTP handler carries an @route tag whose method +
// path MATCH what apps/website/internal/handlers/handlers.go Register() actually wires — not mere
// presence. A renamed route, a wrong verb, or a missing tag becomes a CI failure.
const HANDLERS_DIR = join(repoRoot, "apps/website/internal/handlers");
const HTTP_METHODS = "GET|POST|PUT|PATCH|DELETE";

// Parse Register(): group-variable prefixes (rg.Group("/ingest") etc.) + each
// `<grp>.<METHOD>("<path>", [mw,] h.<Handler>)` route line -> { method, route }.
function parseWiredRoutes() {
  const src = readFileSync(join(HANDLERS_DIR, "handlers.go"), "utf8").split("\n");
  const group = { rg: "" };
  const wired = {};
  const routeRe = new RegExp(`^\\s*(\\w+)\\.(${HTTP_METHODS})\\("([^"]*)"`);
  for (const line of src) {
    const g = line.match(/(\w+)\s*:=\s*rg\.Group\("([^"]*)"/);
    if (g) { group[g[1]] = g[2]; continue; }
    const r = line.match(routeRe);
    if (!r) continue;
    const h = line.match(/h\.(\w+)\)\s*$/); // handler is the last arg
    if (!h || !(r[1] in group)) continue;
    wired[h[1]] = { method: r[2], route: "/api/v1" + group[r[1]] + r[3] };
  }
  return wired;
}

function checkGoRoutes() {
  const routeProblems = [];
  let checked = 0;
  const wired = parseWiredRoutes();
  const tagRe = new RegExp(`@route\\s+(${HTTP_METHODS})\\s+(\\S+)`);
  for (const entry of readdirSync(HANDLERS_DIR)) {
    if (!entry.endsWith(".go") || entry.endsWith("_test.go")) continue;
    const lines = readFileSync(join(HANDLERS_DIR, entry), "utf8").split("\n");
    for (let i = 0; i < lines.length; i++) {
      const fn = lines[i].match(/^func \(h \*Handler\) ([A-Z]\w*)\(c \*gin\.Context\) \{$/);
      if (!fn) continue; // exported route handler (value-returning + lowercase helpers excluded)
      const name = fn[1];
      checked += 1;
      const loc = `apps/website/internal/handlers/${entry}:${i + 1}`;
      const w = wired[name];
      if (!w) { routeProblems.push(`${loc}: ${name} not wired in Register() (GO-7)`); continue; }
      let j = i - 1;
      const blk = [];
      while (j >= 0 && lines[j].trim().startsWith("//")) { blk.push(lines[j]); j--; }
      const tag = blk.map((l) => l.match(tagRe)).find(Boolean);
      if (!tag) { routeProblems.push(`${loc}: ${name} missing @route (GO-7)`); continue; }
      if (tag[1] !== w.method || tag[2] !== w.route) {
        routeProblems.push(`${loc}: ${name} @route ${tag[1]} ${tag[2]} != Register ${w.method} ${w.route} (GO-7)`);
      }
    }
  }
  return { routeProblems, checked };
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
} else {
  console.log("All @contract citations resolve.");
}

// TS-6: @model / @contract presence on front-end contract-layer exports.
const { tagProblems, checked } = checkTsContractTags();
console.log(`Checked ${checked} front-end contract export(s) for @model/@contract (TS-6).`);
if (tagProblems.length > 0) {
  console.error(`\n${tagProblems.length} export(s) missing @model/@contract:`);
  for (const p of tagProblems) console.error(`  ${p}`);
} else {
  console.log("All front-end contract exports carry @model/@contract.");
}

// GO-7: @route presence + method/path match against Register().
const { routeProblems, checked: routesChecked } = checkGoRoutes();
console.log(`Checked ${routesChecked} handler(s) against Register() routes (GO-7).`);
if (routeProblems.length > 0) {
  console.error(`\n${routeProblems.length} @route problem(s):`);
  for (const p of routeProblems) console.error(`  ${p}`);
} else {
  console.log("All handler @route tags match Register().");
}

if (problems.length > 0 || tagProblems.length > 0 || routeProblems.length > 0) process.exit(1);
