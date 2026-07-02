// T-090 spec consistency gates (Definition of Done). Greps the t090 spec corpus + engineering_plan
// for the audit's resolved contradictions and proves every referenced command exists. Exit 1 on any
// violation. These gates encode N1-N12: Deck-zoom LOD, no Deck pick, audit closure, command surface.
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = resolve(root, "..", "..");
const specDir = join(repoRoot, "docs", "specs", "Mission_Creator_Architecture");
const read = (p) => readFileSync(p, "utf8");

const t090Files = readdirSync(specDir).filter((f) => /^t090.*\.md$/.test(f));
const corpus = t090Files.map((f) => ({ name: f, text: read(join(specDir, f)) }));

const failures = [];
const fail = (gate, msg) => failures.push(`[${gate}] ${msg}`);

// Helper: does a context window around index `i` contain any allowed token?
const windowHas = (text, i, radius, re) => re.test(text.slice(Math.max(0, i - radius), i + radius));

// Gate 1 — no "Pick/select world objects (future" anywhere in the corpus.
for (const { name, text } of corpus) {
  if (/Pick\/select world objects \(future/i.test(text)) {
    fail("1", `${name}: contains forbidden "Pick/select world objects (future..."`);
  }
}

// Gate 2 — if a file mentions "reuse slotClusterIndex" it must also say "separate world".
for (const { name, text } of corpus) {
  if (/reuse\s+slotClusterIndex/i.test(text) && !/separate\s+world/i.test(text)) {
    fail("2", `${name}: "reuse slotClusterIndex" without "separate world" clarification`);
  }
}

// Gate 3 — LOD shorthand using tile zoom (z0-5: z<=2, z3-4, z0+) must have deckZoom / "Deck orthographic"
// within 800 chars (LOD must be expressed in Deck orthographic zoom, not tile z).
const lodZoom = /z\s*[≤≥<>]\s*[0-5]|\bz[0-5]\s*[-–]\s*z?[0-5]\b|\bz[0-5]\+/g;
const zoomCtx = /deckZoom|Deck orthographic/i;
for (const { name, text } of corpus) {
  for (const m of text.matchAll(lodZoom)) {
    if (!windowHas(text, m.index, 800, zoomCtx)) {
      fail("3", `${name}: tile-zoom LOD token "${m[0].trim()}" without deckZoom/Deck-orthographic context within 800 chars`);
    }
  }
}

// Gate 4 — "Deck pick" / world-layer "onHover" only allowed next to forbidden/removed/never/no-deck.
const pickCtx = /forbidden|removed|never|no\s+deck|not\s+re-?enable|do\s+not/i;
for (const { name, text } of corpus) {
  for (const re of [/Deck\s+pick/gi, /onHover/g]) {
    for (const m of text.matchAll(re)) {
      if (!windowHas(text, m.index, 220, pickCtx)) {
        fail("4", `${name}: "${m[0]}" without forbidden/removed/never context within 220 chars`);
      }
    }
  }
}

// Gate 5 — engineering_plan.md must not contain "Picking via Deck's onClick/onHover" (backticks stripped).
const engPlan = read(join(specDir, "engineering_plan.md")).replace(/[`*]/g, "");
if (/Picking via Deck's onClick\/onHover/i.test(engPlan)) {
  fail("5", "engineering_plan.md: still contains \"Picking via Deck's onClick/onHover\"");
}

// Gate 6 — hub Audit closure must list every gap id + low gap.
const hub = read(join(specDir, "t090_091_map_terrain_program.md"));
const gapIds = [
  "GAP-001", "GAP-002", "GAP-003", "GAP-004", "GAP-005",
  "GAP-H1", "GAP-H2", "GAP-H3", "GAP-H4", "GAP-H5", "GAP-H6", "GAP-H7", "GAP-H8",
  "GAP-M1", "GAP-M2", "GAP-M3", "GAP-M4", "GAP-M5", "GAP-M6", "GAP-M7",
];
for (const id of gapIds) {
  if (!hub.includes(id)) fail("6", `t090_091_map_terrain_program.md: audit closure missing ${id}`);
}
for (const low of ["L1", "L2", "L3", "L4", "L5"]) {
  if (!new RegExp(`\\b${low}\\b`).test(hub)) fail("6", `t090_091_map_terrain_program.md: audit closure missing ${low}`);
}

// Gate 7 — every `make <hyphenated-target>` and `npm run <script>` referenced in the corpus exists.
const makefile = read(join(repoRoot, "Makefile"));
const makeTargets = new Set([...makefile.matchAll(/^([A-Za-z0-9_-]+):/gm)].map((m) => m[1]));
const pkg = JSON.parse(read(join(repoRoot, "packages", "tbd-schema", "package.json")));
const npmScripts = new Set(Object.keys(pkg.scripts ?? {}));
for (const { name, text } of corpus) {
  for (const m of text.matchAll(/\bmake\s+([a-z0-9]+(?:-[a-z0-9]+)+)/g)) {
    if (!makeTargets.has(m[1])) fail("7", `${name}: referenced \`make ${m[1]}\` not defined in root Makefile`);
  }
  for (const m of text.matchAll(/\bnpm run ([a-z0-9:_-]+)/g)) {
    const lineStart = text.lastIndexOf("\n", m.index) + 1;
    const lineEnd = text.indexOf("\n", m.index);
    const line = text.slice(lineStart, lineEnd < 0 ? undefined : lineEnd);
    if (/apps\/website\/frontend/.test(line)) continue;
    if (!npmScripts.has(m[1])) fail("7", `${name}: referenced \`npm run ${m[1]}\` not in packages/tbd-schema/package.json scripts`);
  }
}

// Gate 8 — single source of truth: no doc may claim T-090.1 is the active slice (active = T-090.3.0;
// T-090.1 is queued). Scan = t090 corpus + authority docs; exclude the audit artifact + generated
// TICKET_* docs. A line is OK if it also names T-090.3.0, says "queued", or uses the feature term
// "active basemap" (not a slice claim).
const authorityDocs = [
  join(repoRoot, "CLAUDE.md"),
  join(specDir, "ROADMAP.md"),
  join(specDir, "agent_execution.md"),
  join(specDir, "engineering_plan.md"),
  join(repoRoot, "docs", "website", "frontend", "ROADMAP.md"),
  join(repoRoot, "docs", "website", "frontend", "INDEX.md"),
  join(repoRoot, "docs", "website", "frontend", "pages", "mission-editor.md"),
  join(repoRoot, "apps", "website", "frontend", "docs", "INDEX.md"),
  join(repoRoot, "apps", "website", "frontend", "docs", "pages", "mission-editor.md"),
  join(repoRoot, "docs", "mod", "CLAUDE-CODE-START.md"),
];
const gate8Files = [
  ...corpus,
  ...authorityDocs.map((p) => ({ name: relative(repoRoot, p), text: existsSync(p) ? read(p) : "" })),
];
const hasT0901 = /T-090\.1(?!\.\d)/; // T-090.1 but not T-090.1.1
for (const { name, text } of gate8Files) {
  for (const line of text.split("\n")) {
    if (!hasT0901.test(line) || !/\bactive\b/i.test(line)) continue;
    if (/T-090\.3\.0/.test(line) || /\bqueued\b/i.test(line) || /active\s+basemap/i.test(line)) continue;
    fail("8", `${name}: claims T-090.1 active — "${line.trim().slice(0, 90)}"`);
  }
}

// Gate 9 — the Eden AI schema must not describe MC mutation of world objects (read-only context).
if (/move\/delete this object/i.test(read(join(specDir, "t090_eden_ai_world_object_schema.md")))) {
  fail("9", "t090_eden_ai_world_object_schema.md: still says \"move/delete this object\" (mutation is Workbench-only)");
}

// Gate 10 — hub header (first ~800 chars) must name the registry active slice for T-090.
let activeSlice = "T-090.1.2.5";
try {
  const reg = JSON.parse(read(join(repoRoot, ".ai", "tickets", "registry.json")));
  const t090 = reg.tickets?.find((t) => t.id === "T-090");
  if (t090?.active_slice) activeSlice = t090.active_slice;
} catch {
  /* fallback */
}
if (!hub.slice(0, 800).includes(activeSlice)) {
  fail("10", `t090_091_map_terrain_program.md: header does not name ${activeSlice} as the active slice`);
}

// Gate 11 — type-inventory spec must not publish order-of-magnitude Everon ranges (exact integers only).
const inventorySpec = read(join(specDir, "t090_world_object_type_inventory.md"));
const rangeRe = /800k|900k|1\.2M|2k–20k|400k–900k|order-of-magnitude \(Everon/i;
for (const line of inventorySpec.split("\n")) {
  if (rangeRe.test(line) && !/\bnever\b|forbidden|not a substitute|PENDING|hard-coded|no hard-/i.test(line)) {
    fail(
      "11",
      `t090_world_object_type_inventory.md: Everon estimate range — "${line.trim().slice(0, 90)}"`,
    );
  }
}
if (!/censusStatus/.test(inventorySpec) || !/pending_export/.test(inventorySpec)) {
  fail("11", "t090_world_object_type_inventory.md: must document censusStatus pending_export baseline");
}

if (failures.length) {
  console.error(`verify-t090-specs: FAIL (${failures.length})`);
  for (const f of failures) console.error(`  ${f}`);
  process.exit(1);
}
console.log(`verify-t090-specs: OK (${t090Files.length} spec files + authority docs, all 11 gates pass)`);
