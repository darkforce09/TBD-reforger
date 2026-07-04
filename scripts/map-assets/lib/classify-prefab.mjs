#!/usr/bin/env node
// T-090.3.0 — single shared prefab classifier.
//
// ONE source of truth for K1 (verify-spike-k1.mjs) and K1b (census-spike.mjs) so the two gates can
// never disagree (micro-nit 1). Wraps packages/tbd-schema/rules/prefab-classify.json: the first rule
// whose `match.resourceNameContains` substring appears in the resourceName wins (rule order = priority);
// otherwise the rule file's `fallback` (kind=prop, class=unknown). Substring match is case-sensitive —
// resourceName paths preserve PascalCase (e.g. "…/House_01.et"), and the rule needles are authored to match.
import { createReadStream, readFileSync } from "node:fs";
import { createInterface } from "node:readline";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
const RULES_PATH = join(repoRoot, "packages", "tbd-schema", "rules", "prefab-classify.json");

let _rules = null;

/** Load (and cache) the shared classification rule file. */
export function loadRules() {
  if (!_rules) _rules = JSON.parse(readFileSync(RULES_PATH, "utf8"));
  return _rules;
}

/**
 * Classify a single resourceName.
 * @param {string} resourceName e.g. "{GUID}Prefabs/.../House_01.et"
 * @returns {{ kind: string, class: string, matched: boolean, rule: object }}
 *   `matched` is false when the fallback was used (needsReview territory).
 */
export function classifyResourceName(resourceName) {
  const rules = loadRules();
  const name = typeof resourceName === "string" ? resourceName : "";
  for (const rule of rules.rules ?? []) {
    const needles = rule.match?.resourceNameContains ?? [];
    if (needles.some((n) => n && name.includes(n))) {
      return { kind: rule.kind, class: rule.class, matched: true, rule };
    }
  }
  const fb = rules.fallback ?? { kind: "prop", class: "unknown" };
  return { kind: fb.kind, class: fb.class, matched: false, rule: fb };
}

/**
 * T-090.3.1 — memoized classifier for full-map streams. classifyResourceName is O(rules × needles)
 * per call; a full Everon raw export is ~1M rows over only a few-k unique resourceNames, so the
 * memo turns 10^8+ substring scans into hash hits. One memo per call site (rules are cached
 * process-wide anyway).
 * @returns {(resourceName: string) => { kind: string, class: string, matched: boolean, rule: object }}
 */
export function createClassifier() {
  const memo = new Map();
  return (resourceName) => {
    const name = typeof resourceName === "string" ? resourceName : "";
    let hit = memo.get(name);
    if (!hit) {
      hit = classifyResourceName(name);
      memo.set(name, hit);
    }
    return hit;
  };
}

/**
 * T-090.3.1 — stream a raw-entities.jsonl of any size (full Everon ≈ 250-300 MB; readFileSync+split
 * would ~2.5× that in heap). Calls onRow(row, lineNumber) per parsed line.
 *
 * Parse errors are FATAL by design (throws on the first bad line): the full pipeline's count gates
 * (G11/I1) are exact-integer identities, and a truncated copy must never survive to a census.
 * Spike-sized callers keep using classifyRawEntitiesJsonl (collect-and-skip semantics).
 *
 * @param {string} path absolute path to the jsonl
 * @param {(row: object, lineNumber: number) => void} onRow
 * @returns {Promise<{ lineCount: number }>}
 */
export async function streamRawEntities(path, onRow) {
  const rl = createInterface({ input: createReadStream(path, { encoding: "utf8" }), crlfDelay: Infinity });
  let lineNumber = 0;
  let lineCount = 0;
  for await (const raw of rl) {
    lineNumber++;
    const line = raw.trim();
    if (!line) continue;
    let row;
    try {
      row = JSON.parse(line);
    } catch (e) {
      throw new Error(`streamRawEntities: parse error at ${path}:${lineNumber} — ${e}`);
    }
    lineCount++;
    onRow(row, lineNumber);
  }
  return { lineCount };
}

/**
 * Classify every row of a raw-entities.jsonl file.
 * @param {string} path absolute path to the jsonl
 * @returns {{ entries: Array<{ row: object, kind: string, class: string, matched: boolean }>,
 *            errors: Array<{ line: number, error: string }> }}
 */
export function classifyRawEntitiesJsonl(path) {
  const text = readFileSync(path, "utf8");
  const entries = [];
  const errors = [];
  const lines = text.split("\n");
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i].trim();
    if (!line) continue;
    let row;
    try {
      row = JSON.parse(line);
    } catch (e) {
      errors.push({ line: i + 1, error: String(e) });
      continue;
    }
    const cls = classifyResourceName(row.resourceName);
    entries.push({ row, kind: cls.kind, class: cls.class, matched: cls.matched });
  }
  return { entries, errors };
}
