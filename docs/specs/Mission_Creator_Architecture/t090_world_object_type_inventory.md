# T-090 — World object type inventory (counts + taxonomy census)

**Status:** Spec ready — runs on **first raw export** and after **every phased import**  
**Authority:** [`t090_2_map_object_taxonomy.md`](t090_2_map_object_taxonomy.md) · [`t090_phased_object_import.md`](t090_phased_object_import.md)

---

## In one sentence

Discover and publish **exact counts** of world objects at three levels — **`kind`**, **`class` subtype**, and **unique prefab** — so AI and humans know how many building types (and everything else) exist on each map **before** claiming import complete.

---

## Problem

We do **not** yet know how many distinct object types Everon (or Arland) contains until Workbench export runs. The schema defines **allowed** `buildingClass` / `roadClass` values, but **ground truth counts** (`prefabTypes`, `instances` per class) come only from a **prefab census** artifact — not guesswork.

**Eden AI** must answer: *“How many military buildings are in this sector?”* → requires inventory + instances.

---

## Three levels of “type” (do not conflate)

| Level | Field | Cardinality | Example | Schema |
|-------|-------|-------------|---------|--------|
| **L1 — Kind** | `kind` | **8 fixed values** | `building`, `tree`, `road` | Closed enum in `map-object-prefab.schema.json` |
| **L2 — Class** | `class` | **Closed enum per kind** | `buildingClass=residential` | See §Subtype enums below |
| **L3 — Prefab type** | `resourceName` | **Discovered @ export** | `{GUID}Prefabs/.../House_01.et` | Thousands on Everon |
| **L4 — Instance** | `id` + placement | **Discovered @ export** | One pine at (5120, 4800) | exact count in `type-inventory.json` |

**AI reads:** L1+L2 from schema rules; L3+L4 from **`type-inventory.json`** + streamed chunks.

---

## Subtype enums (L2 — normative, JSON Schema)

`class` **must** be one of the values for its `kind`. Unknown export rows → `unknown` + `needsReview: true`.

### `building` → `buildingClass`

| `buildingClass` | Description | Typical gameplay |
|-----------------|-------------|------------------|
| `residential` | Houses, apartments, cottages | hard cover, enterable |
| `commercial` | Shops, offices, restaurants | hard cover, enterable |
| `industrial` | Factories, warehouses, silos | hard cover |
| `agricultural` | Barns, farm sheds, greenhouses | hard cover |
| `civic` | Churches, town halls, schools | hard cover, enterable |
| `military` | Barracks, FOB structures, checkpoints | hard cover |
| `bunker` | Hardened bunkers, pillboxes | hard cover |
| `hangar` | Aircraft / vehicle hangars | hard cover, large footprint |
| `garage` | Vehicle garages, workshops | hard cover, enterable |
| `tower` | Watchtowers, radio masts (built-up) | hard cover, climbable |
| `ruin` | Destroyed / damaged shells | partial cover |
| `wall` | Compound walls, perimeter segments (footprint) | hard cover, not enterable |
| `generic` | Classified building, no finer rule | hard cover default |
| `unknown` | Unmatched — needs `prefab-classify` rule | flagged |

Optional finer grain: `buildingSubClass` (free string, e.g. `"single_family"`, `"apartment_block"`) — **not** required for P1; AI may use when rules add it.

### Other kinds → `class` field

| `kind` | `class` enum name | Values |
|--------|-------------------|--------|
| `road` | `roadClass` | `highway_paved`, `road_paved`, `road_dirt`, `track`, `path`, `runway`, `unknown` |
| `tree` | `speciesClass` | `conifer`, `deciduous`, `palm`, `dead`, `unknown` |
| `vegetation` | `speciesClass` | `bush`, `grass`, `fern`, `dead`, `unknown` |
| `rock` | `rockClass` | `boulder`, `cliff`, `pebble`, `scree`, `unknown` |
| `prop` | `propClass` | `fence`, `barrier`, `sign`, `furniture`, `debris`, `pebble`, `unknown` |
| `utility` | `utilityClass` | `powerline`, `lamp`, `antenna`, `pipeline`, `unknown` |
| `water` | `waterClass` | `pier`, `dock`, `buoy`, `unknown` |

Full tables: [`t090_2_map_object_taxonomy.md`](t090_2_map_object_taxonomy.md) §Taxonomy.

**Schema file:** `packages/tbd-schema/schema/map-object-enums.schema.json` (referenced by prefab + resolved schemas) — **closed enums**, extensible only via schema version bump + rule update.

---

## Exact-count policy (locked)

**Verification uses exact integers only.** Examples: `1_100_112` total instances, `512_086` trees, `47_218` buildings — never "about 900k", "roughly 500k", or ±2% on totals.

| Rule | Detail |
|------|--------|
| **Before export** | `censusStatus: "pending_export"` — all counts are **`null`** in `type-inventory.json`. Docs cite **PENDING**, not ranges. |
| **After export** | `make map-census TERRAIN=<id>` writes **exact** non-negative integers. Re-census without re-export → **byte-identical** JSON (I6). |
| **Phase ship** | `make map-verify-phase` compares export row counts to inventory — **integer equality** (G11). |
| **UI / legend** | Filter chips show inventory integers (e.g. `Buildings · military (12)`), not extrapolations. |
| **Manifest mirror** | `manifest.json` → `objects.instanceCount` / `objects.prefabCount` **must equal** inventory `levels.*` for the shipped phase. |
| **Human baseline** | [`.ai/artifacts/everon_object_count_baseline.md`](../../../.ai/artifacts/everon_object_count_baseline.md) — updated when Everon census first lands. |

**Forest regions:** at ship time `byRegionKind.forest.treeCount + unassignedTrees = byKind.tree.instances` (**exact**). A ±2% tolerance applies **only** to provisional hull assignment during P2 **development** — it is **not** a verification gate for totals.

---

## Type inventory artifact (ground truth counts)

Generated by **`scripts/map-assets/census-types.mjs`** after raw export + after each phased import.

**Path:** `packages/map-assets/{terrainId}/objects/type-inventory.json`  
**Schema:** `packages/tbd-schema/schema/map-object-type-inventory.schema.json`  
**Also:** `.ai/artifacts/type_inventory_{terrainId}.json` (copy for agents)

**Everon today:** committed stub @ `packages/map-assets/everon/objects/type-inventory.json` with `censusStatus: "pending_export"` (all counts `null`).

```json
{
  "schemaVersion": "1.0.0",
  "terrainId": "everon",
  "censusStatus": "pending_export",
  "generatedAt": null,
  "importPhaseMax": null,
  "sourceExportPath": null,
  "levels": {
    "uniquePrefabs": null,
    "totalInstances": null
  },
  "byKind": {
    "building": { "prefabTypes": null, "instances": null },
    "tree": { "prefabTypes": null, "instances": null },
    "road": { "prefabTypes": null, "instances": null, "segments": null }
  },
  "byBuildingClass": {},
  "needsReview": { "prefabTypes": null, "prefabs": [] }
}
```

**After first census** (`censusStatus: "partial"` or `"complete"`), every populated field is an **exact integer**:

```json
{
  "schemaVersion": "1.0.0",
  "terrainId": "everon",
  "censusStatus": "partial",
  "generatedAt": "2026-06-26T20:00:00Z",
  "importPhaseMax": "P1_buildings",
  "sourceExportPath": "staging/spike/raw-entities.jsonl",
  "levels": {
    "uniquePrefabs": 8420,
    "totalInstances": 1100112
  },
  "byKind": {
    "building": { "prefabTypes": 312, "instances": 47218 },
    "tree": { "prefabTypes": null, "instances": null },
    "road": { "prefabTypes": null, "instances": null, "segments": null }
  },
  "byBuildingClass": {
    "residential": { "prefabTypes": 142, "instances": 28104 },
    "military": { "prefabTypes": 12, "instances": 847 }
  }
}
```

### Field definitions

| Field | Meaning |
|-------|---------|
| `prefabTypes` | Count of **distinct `resourceName`** (L3) in bucket |
| `instances` | Count of **placements** (L4) referencing those prefabs |
| `segments` | Road polylines only |
| `prefabs[]` | Optional manifest: `{ resourceName, prefabId, instanceCount, class }` — capped at 500 rows in summary file; full list in `type-inventory-prefabs.jsonl` |

---

## Discovery workflow (when counts are unknown)

```text
1. First Workbench raw export (even partial / P1 filter)
      → export/raw-entities.jsonl

2. make map-census TERRAIN=everon
      → census-types.mjs: scan raw + classified prefabs
      → type-inventory.json (counts still 0 until classified)

3. After classify + build-catalog-v1 for phase Pn
      → re-run map-census (counts populate)

4. Compare inventory to prior phase
      → Δ prefabTypes / instances must match G11 in phased verify
```

**Until first export:** Everon counts are **unknown** — read `censusStatus` on the committed stub. **Never** substitute order-of-magnitude guesses in verify gates, phase budgets, or UI.

**After first census:** docs, ops logs, manifest `objects.*`, and phase verify scripts cite **`type-inventory.json`** integers only.

---

## Forest region census + UI counts (N5 / L3)

The census also counts **regions**: `byRegionKind.forest.count`, total `treeCount`, `unassignedTrees`,
and a `dominantSpeciesClass` histogram. **Ship gate:** `byRegionKind.forest.treeCount + unassignedTrees =
byKind.tree.instances` (exact integer equality). **`type-inventory.json` drives the editor UI (L3):** the legend +
filter panel ([`t090_9_world_object_interaction.md`](t090_9_world_object_interaction.md)) render live
counts — e.g. "Buildings · military (847)", "Forests (8 · 38_412 trees)" — not approximations.

## Commands

```bash
make map-census TERRAIN=everon
make map-census TERRAIN=everon PHASE=P1_buildings   # scoped to phase filter
```

---

## Mathematical verification (inventory)

| ID | Invariant | Pass |
|----|-----------|------|
| I1 | `Σ byKind.instances = levels.totalInstances` | integer eq |
| I2 | `Σ byBuildingClass.instances = byKind.building.instances` (when P1+ shipped) | integer eq |
| I3 | Every `class` in inventory ∈ schema enum for that kind | script |
| I4 | `needsReview.prefabTypes = 0` before phase **ship** (rules complete for phase filter) | integer eq |
| I5 | `prefabTypes` in inventory = `manifest.prefabCount` filtered by phase | integer eq |
| I6 | Re-census without re-export → identical JSON (deterministic) | byte eq |
| I7 | `manifest.objects.instanceCount` = `levels.totalInstances` when objects block present | integer eq |
| I8 | Every doc/spec phase budget cites inventory integers for shipped terrain — no hard-coded ~900k | gate 11 |

Runs as part of `make map-verify-phase` and `make schema-validate` (`verify-type-inventory.mjs`).

---

## Everon baseline (authoritative when census lands)

| Bucket | prefabTypes | instances | Status |
|--------|-------------|-----------|--------|
| **All kinds** | *pending* | *pending* | `pending_export` |
| `building` | *pending* | *pending* | |
| `tree` | *pending* | *pending* | |
| `vegetation` | *pending* | *pending* | |
| `rock` | *pending* | *pending* | |
| `prop` | *pending* | *pending* | |
| `utility` | *pending* | *pending* | |
| `water` | *pending* | *pending* | |
| `road` segments | *pending* | *pending* | |

**Fill this table from `type-inventory.json` after T-090.3.0 spike + first `make map-census TERRAIN=everon`.** Commit the updated JSON; run `./scripts/ticket sync` if registry ops log references counts.

Human-readable mirror: [`.ai/artifacts/everon_object_count_baseline.md`](../../../.ai/artifacts/everon_object_count_baseline.md).

---

## P1 buildings — inventory acceptance

Before **P2 trees**, inventory must show:

| Check | Pass |
|-------|------|
| `byKind.building.instances > 0` | exported |
| Every non-`unknown` `buildingClass` has ≥1 `prefabTypes` **or** documented empty enum | script |
| `byBuildingClass.unknown.instances / byKind.building.instances < 0.5%` | ratio |
| Full prefab list for buildings in `type-inventory-prefabs.jsonl` | file exists |

AI uses **`byBuildingClass`** to list exact building types on map: *“Everon has 47 residential prefab types, 12 military, …”*

---

## Deliverables (T-090.2 / T-090.3)

| # | Artifact |
|---|----------|
| 1 | `packages/tbd-schema/schema/map-object-type-inventory.schema.json` |
| 2 | `scripts/map-assets/census-types.mjs` + `packages/tbd-schema/scripts/verify-type-inventory.mjs` |
| 3 | `make map-census` Makefile target |
| 4 | `type-inventory.json` per terrain @ export |
| 5 | Golden: one row per **`buildingClass`** in `golden/phased/P1-buildings.json` |

---

## Related

- [`t090_2_map_object_taxonomy.md`](t090_2_map_object_taxonomy.md)
- [`t090_eden_ai_world_object_schema.md`](t090_eden_ai_world_object_schema.md)
- [`t090_phased_object_import.md`](t090_phased_object_import.md)
