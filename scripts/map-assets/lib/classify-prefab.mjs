#!/usr/bin/env node
// T-090.3.0 — single shared prefab classifier.
//
// ONE source of truth for K1 (verify-spike-k1.mjs) and K1b (census-spike.mjs) so the two gates can
// never disagree (micro-nit 1). Wraps packages/tbd-schema/rules/prefab-classify.json: the first rule
// whose `match.resourceNameContains` substring appears in the resourceName wins (rule order = priority);
// otherwise the rule file's `fallback` (kind=prop, class=unknown). Substring match is case-sensitive —
// resourceName paths preserve PascalCase (e.g. "…/House_01.et"), and the rule needles are authored to match.
import { readFileSync } from "node:fs";
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
