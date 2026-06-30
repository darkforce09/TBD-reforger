// T-090.2 enum single-source gate (GAP-M5). Every `kind`/`class` used by the golden prefabs, the
// prefab-classify rules, and the glyph manifest MUST be a member of the matching map-object-enums
// $def. Fails (exit 1) on any drift so the closed enums stay the one source of truth.
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = resolve(root, "..", "..");
const readJSON = (p) => JSON.parse(readFileSync(p, "utf8"));

const enums = readJSON(join(root, "schema", "map-object-enums.schema.json")).$defs;
const set = (name) => new Set(enums[name].enum);
const sets = {
  kind: set("kind"),
  buildingClass: set("buildingClass"),
  roadClass: set("roadClass"),
  speciesClass: set("speciesClass"),
  forestClass: set("forestClass"),
  rockClass: set("rockClass"),
  propClass: set("propClass"),
  utilityClass: set("utilityClass"),
  waterClass: set("waterClass"),
};

// Instance/prefab kind -> the closed enum its `class` must come from.
const classEnumForKind = {
  building: "buildingClass",
  road: "roadClass",
  tree: "speciesClass",
  vegetation: "speciesClass",
  rock: "rockClass",
  prop: "propClass",
  utility: "utilityClass",
  water: "waterClass",
};

const errors = [];
const checkRow = (src, kind, klass) => {
  if (!sets.kind.has(kind)) {
    errors.push(`${src}: kind '${kind}' not in map-object-enums#/$defs/kind`);
    return;
  }
  const enumName = classEnumForKind[kind];
  if (!enumName) {
    errors.push(`${src}: kind '${kind}' has no class-enum mapping (regions carry no prefab class)`);
    return;
  }
  if (klass !== undefined && !sets[enumName].has(klass)) {
    errors.push(`${src}: class '${klass}' not in ${enumName} (kind=${kind})`);
  }
};

// 1. Golden prefab rows.
const prefabs = readJSON(join(root, "golden", "map-objects", "map-object-prefabs-sample.json"));
for (const p of prefabs) checkRow(`golden prefab ${p.prefabId}`, p.kind, p.class);

// 2. prefab-classify rules + fallback.
const classify = readJSON(join(root, "rules", "prefab-classify.json"));
for (const [i, r] of (classify.rules ?? []).entries()) checkRow(`prefab-classify rule[${i}]`, r.kind, r.class);
if (classify.fallback) checkRow("prefab-classify fallback", classify.fallback.kind, classify.fallback.class);

// 3. Golden region dominantSpeciesClass must be a forestClass.
const regions = readJSON(join(root, "golden", "map-objects", "map-object-regions-everon-sample.json"));
for (const reg of regions) {
  if (!sets.kind.has(reg.kind)) errors.push(`region ${reg.id}: kind '${reg.kind}' not in kind enum`);
  if (reg.dominantSpeciesClass !== undefined && !sets.forestClass.has(reg.dominantSpeciesClass)) {
    errors.push(`region ${reg.id}: dominantSpeciesClass '${reg.dominantSpeciesClass}' not in forestClass`);
  }
}

// 4. Glyph manifest iconKeys: the leading {kind} token must be a valid kind (badge/class suffix is free).
const glyphs = readJSON(join(repoRoot, "packages", "map-assets", "glyphs", "manifest.json")).glyphs;
for (const key of Object.keys(glyphs)) {
  const kindTok = key.split("-")[0];
  if (!sets.kind.has(kindTok)) errors.push(`glyph '${key}': kind prefix '${kindTok}' not in kind enum`);
}

if (errors.length) {
  console.error("verify-map-object-enums: FAIL");
  for (const e of errors) console.error(`  ${e}`);
  process.exit(1);
}
console.log(`verify-map-object-enums: OK (${prefabs.length} prefabs, ${Object.keys(glyphs).length} glyphs, enums single-source)`);
