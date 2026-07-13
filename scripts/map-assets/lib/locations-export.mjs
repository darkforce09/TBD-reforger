/**
 * T-152.6 — derive locations.json rows from staged Workbench raw-entities JSONL.
 * Path B: World/Locations/* composition prefabs + CfgWorlds Names crosswalk for towns
 * without a dedicated Location .et anchor.
 */
import { readFileSync } from "node:fs";

/** @typedef {{ id: string, name: string, x: number, y: number, importance?: number, kind?: string }} LocationRow */

export const REQUIRED_EVERON_TOWNS = [
  "Morton",
  "Gorey",
  "Highstone",
  "Raccoon Rock",
  "Saint Philippe",
  "Levie",
  "Montignac",
  "Kermovan",
];

/** Interim floor — bumped to count after first export (T-152.6 L2). */
export const N_MIN = 10;

/** Capital / large settlements ≥ 0.7 (verify log operator table). */
export const IMPORTANCE_BY_NAME = {
  Montignac: 0.85,
  "Saint Philippe": 0.78,
  Levie: 0.74,
  Chotain: 0.72,
  Morton: 0.7,
  Gorey: 0.62,
  Kermovan: 0.58,
  "Raccoon Rock": 0.52,
  Highstone: 0.48,
};

const DISPLAY_OVERRIDES = {
  EntreDeux: "Entre Deux",
  Le_Moule: "Le Moule",
  Villeneuf: "Villeneuve",
  StPhilippe_StPhilippe_01: "Saint Philippe",
  Airport: "Airport",
};

const KIND_BY_BASENAME = {
  Airport: "airport",
};

/**
 * Towns present in CfgWorlds Names / cartographic labels but absent as top-level
 * `World/Locations/Eden/{Name}.et` composition prefabs in the full-world export.
 * Coordinates cross-validated: EnfusionMapMaker (APL-SA), operator grid refs, map_export_everon.json.
 */
export const CFGWORLD_NAME_SUPPLEMENT = [
  {
    id: "everon-gorey",
    name: "Gorey",
    x: 4844.906,
    y: 8088.995,
    kind: "village",
    source: "EnfusionMapMaker resource-depot marker + nearest prop @ 1 m",
  },
  {
    id: "everon-highstone",
    name: "Highstone",
    x: 4950,
    y: 8550,
    kind: "peak",
    source: "Operator grid 049,085 (SteamAH GM guide) + nearest prop @ 2 m",
  },
  {
    id: "everon-raccoon-rock",
    name: "Raccoon Rock",
    x: 1280,
    y: 6400,
    kind: "natural",
    source: "map_export_everon.json subregionCellCentreM + nearest cliff @ 4 m",
  },
  {
    id: "everon-kermovan",
    name: "Kermovan",
    x: 6359.376,
    y: 9668.684,
    kind: "village",
    source: "EnfusionMapMaker labeledTownLocations crosswalk + nearest prop @ 1 m",
  },
];

const slug = (terrainId, name) =>
  `${terrainId}-${name
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "")}`;

const basenameFromResource = (resourceName) => {
  const m = resourceName.match(/Locations\/Eden\/(?:[^/]+\/)?([^/]+)\.et$/);
  return m ? m[1] : null;
};

const isDirectTownPrefab = (resourceName) =>
  /Prefabs\/World\/Locations\/Eden\/[^/]+\.et$/.test(resourceName);

const isSaintPhilippeAnchor = (resourceName) =>
  resourceName.includes("StPhilippe_StPhilippe_01.et");

const displayNameFromBasename = (base) => DISPLAY_OVERRIDES[base] ?? base.replace(/_/g, " ");

const defaultImportance = (name) => IMPORTANCE_BY_NAME[name] ?? 0.55;

const round3 = (n) => Math.round(n * 1000) / 1000;

const rejectName = (name) => !name || name.length < 2 || /location composition/i.test(name);

/**
 * @param {string} jsonlPath
 * @param {{ terrainId?: string, includePeaks?: boolean }} [opts]
 * @returns {LocationRow[]}
 */
export function exportLocationsFromJsonl(jsonlPath, opts = {}) {
  const terrainId = opts.terrainId ?? "everon";
  const includePeaks = opts.includePeaks ?? true;
  const text = readFileSync(jsonlPath, "utf8");
  const rows = text
    .trim()
    .split("\n")
    .filter(Boolean)
    .map((line) => JSON.parse(line))
    .filter((r) => r.resourceName?.includes("World/Locations/"));

  /** @type {Map<string, LocationRow>} */
  const byId = new Map();

  for (const r of rows) {
    const base = basenameFromResource(r.resourceName);
    if (!base) continue;

    if (isDirectTownPrefab(r.resourceName)) {
      const name = displayNameFromBasename(base);
      if (rejectName(name)) continue;
      const id = slug(terrainId, name);
      byId.set(id, {
        id,
        name,
        x: round3(r.x),
        y: round3(r.z),
        importance: defaultImportance(name),
        kind: KIND_BY_BASENAME[base] ?? (base === "Airport" ? "airport" : "town"),
      });
      continue;
    }

    if (isSaintPhilippeAnchor(r.resourceName)) {
      const name = "Saint Philippe";
      const id = slug(terrainId, name);
      if (!byId.has(id)) {
        byId.set(id, {
          id,
          name,
          x: round3(r.x),
          y: round3(r.z),
          importance: defaultImportance(name),
          kind: "town",
        });
      }
      continue;
    }

    if (!includePeaks) continue;
    if (!r.resourceName.includes("/Natural/")) continue;
    if (!/(Hill|Mountains|Moutains|Peak|Ridge)/i.test(base)) continue;

    const name = displayNameFromBasename(base);
    if (rejectName(name)) continue;
    const id = slug(terrainId, name.toLowerCase());
    if (byId.has(id)) continue;
    byId.set(id, {
      id,
      name,
      x: round3(r.x),
      y: round3(r.z),
      importance: 0.35,
      kind: /hill/i.test(base) ? "hill" : "peak",
    });
  }

  for (const sup of CFGWORLD_NAME_SUPPLEMENT) {
    if (byId.has(sup.id)) continue;
    byId.set(sup.id, {
      id: sup.id,
      name: sup.name,
      x: round3(sup.x),
      y: round3(sup.y),
      importance: defaultImportance(sup.name),
      kind: sup.kind,
    });
  }

  return [...byId.values()].sort((a, b) => a.name.localeCompare(b.name));
}

/** G3/G4/G5/G6 census gates. */
export function verifyLocationsGates(locs) {
  const errors = [];
  if (locs.length < N_MIN) errors.push(`G3: count ${locs.length} < N_MIN ${N_MIN}`);

  const norm = (s) => s.toLowerCase().replace(/\s+/g, "");
  const names = new Set(locs.map((l) => norm(l.name)));

  for (const town of REQUIRED_EVERON_TOWNS) {
    const k = norm(town);
    const ok = names.has(k) || [...names].some((n) => n.includes(k.slice(0, 6)));
    if (!ok) errors.push(`G4: missing required town "${town}"`);
  }

  for (const loc of locs) {
    if (loc.name.length < 2) errors.push(`G5: name too short id=${loc.id}`);
    if (!Number.isFinite(loc.x) || !Number.isFinite(loc.y)) errors.push(`G5: non-finite coords id=${loc.id}`);
    if (/location composition/i.test(loc.name)) errors.push(`G6: placeholder name id=${loc.id}`);
  }

  return errors;
}
