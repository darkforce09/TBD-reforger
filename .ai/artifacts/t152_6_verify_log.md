# T-152.6 verify log — Locations export (`locations.json`)

**Slice:** T-152.6  
**Branch:** `ticket/T-152`  
**Worktree:** `/home/Samuel/Projects/TBD-Reforger/.ai/artifacts/worktrees/TBD-T-152`

## Summary

Everon **`locations.json`** shipped: **60** named places (19 town anchors from `World/Locations/Eden/{Town}.et` + Saint Philippe urban anchor + 37 peaks/hills + 4 CfgWorlds Names crosswalk rows). Schema, golden sample, export/verify scripts, manifest pointer, Path **B** spike JSON, and Path **A** Workbench plugin (`TBD_LocationsExportPlugin.c`) for operator re-export.

## Gate table

| ID | Predicate | Result | Evidence |
|----|-----------|--------|----------|
| **G1** | `make schema-validate` exit 0 including locations schema + sample | **PASS** | `node scripts/validate.mjs` — `locations-everon-sample.json` + `map-assets/everon/locations.json` + `everon/manifest.json` (locations block) **PASS**; `All contracts valid.` |
| **G2** | `locations.json` validates against schema (Ajv) | **PASS** | `node scripts/map-assets/verify-locations.mjs` G2 |
| **G3** | `count(locations) ≥ N_MIN` | **PASS** | **60** rows; **N_MIN = 60** (bumped from interim 10 per L2) |
| **G4** | `REQUIRED_EVERON_TOWNS ⊆ {loc.name}` | **PASS** | All 8 required towns present (case/space normalized) |
| **G5** | `∀ loc: name.length ≥ 2 ∧ finite(x,y)` | **PASS** | verify-locations.mjs |
| **G6** | `∀ loc: ¬/location composition/i.test(name)` | **PASS** | verify-locations.mjs |
| **G7** | Spike JSON documents export path A, B, or C with evidence | **PASS** | Path **B** — [`.ai/artifacts/t152_6_locations_spike.json`](t152_6_locations_spike.json); Path A plugin authored for operator |
| **G8** | T-152.5 verify PASS; `make map-export-validate` still PASS | **PASS** | T-152.5 ALL PASS; `make map-export-validate` exit 0 |

## Pinned numbers

| Quantity | Value |
|----------|-------|
| Location rows | **60** |
| Town anchors (Eden/*.et) | **19** |
| World/Locations rows in raw export | **170** |
| **N_MIN** (new floor) | **60** |
| Required town coords (x, y m) | Morton **5135.24, 4011.78** · Gorey **4844.906, 8088.995** · Highstone **4950, 8550** · Raccoon Rock **1280, 6400** · Saint Philippe **4502.76, 10772** · Levie **7464.44, 4738.91** · Montignac **4773.46, 7094.57** · Kermovan **6359.376, 9668.684** |

## Importance table (capital / large ≥ 0.7)

| Name | importance | kind |
|------|------------|------|
| Montignac | 0.85 | town |
| Saint Philippe | 0.78 | town |
| Levie | 0.74 | town |
| Chotain | 0.72 | town |
| Morton | 0.70 | village |
| Gorey | 0.62 | village |
| Kermovan | 0.58 | village |
| Raccoon Rock | 0.52 | natural |
| Highstone | 0.48 | peak |

## Export path (G7)

**Path B (primary):** `node scripts/map-assets/export-locations.mjs --terrain everon` reads staged `raw-entities.jsonl`, filters `World/Locations/*`, derives display names from prefab basenames, merges CfgWorlds crosswalk for Gorey / Highstone / Raccoon Rock / Kermovan.

**Path A (operator):** `TBD_LocationsExportPlugin.c` → `$profile:TBD_LocationsExport.json` (MCP blocked in CI sandbox).

## Automated commands

```text
node scripts/map-assets/export-locations.mjs --terrain everon  → 0 (60 rows)
node scripts/map-assets/verify-locations.mjs TERRAIN=everon  → 0
cd packages/tbd-schema && node scripts/validate.mjs  → 0 (locations + manifest)
make map-export-validate  → 0
```

## Manual (operator)

| ID | Check | Pass |
|----|-------|------|
| M1 | Gorey + Morton coords vs cartographic cursor ±100 m | ☐ |
| M2 | No row displays "Location composition" | ☑ (G6 automated) |

## Prior slices

| Slice | Result |
|-------|--------|
| T-152.0–.5 | PASS per respective verify logs |

## Ready for

**T-152.7** height markers · **T-152.8** town labels (consumer: `locations.json` + manifest pointer)
