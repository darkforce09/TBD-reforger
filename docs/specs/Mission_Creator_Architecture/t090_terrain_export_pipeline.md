# T-090 — Automated terrain export pipeline (all maps, one command)

**Ticket:** T-090 · **Slice:** T-090.3 (automation contract)  
**Status:** Spec ready  
**Executor:** **claude-code** (scripts + Workbench plugin) · human only if Workbench GUI must be focused  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)  
**Data contract:** [`t090_2_map_object_taxonomy.md`](t090_2_map_object_taxonomy.md) · **AI pack:** §AI-readable metadata below

---

## In one sentence

**One standardized command** exports **every asset class** (DEM, tiles, catalog v1 objects, audits, manifest) for **any registered terrain** into `packages/map-assets/{terrainId}/` — fully validated, AI-ingestible, **no per-map bespoke scripts**.

---

## Product requirement (2026-06)

| Requirement | Solution |
|-------------|----------|
| “I shouldn’t have to fuck around redoing this per map” | Same pipeline + same folder layout for **every** terrain |
| “AI should press a button and it’s done” | `make map-export TERRAIN=everon` (or CI job) — idempotent, logged |
| “AI must understand exactly what each object is” | Prefab **AI metadata block** + machine taxonomy + ops log |
| Eden ~1M objects | catalog v1 dedup + spatial chunks (not one fat JSON) |
| Everon today, Arland tomorrow | `terrain-registry.json` — add row, run same command |
| **One type at a time** | **Phased import P1→P10** — [`t090_phased_object_import.md`](t090_phased_object_import.md) |

**Human involvement target:** zero after Workbench plugin is installed. **No phase advance without `make map-verify-phase` exit 0** — see phased import spec.

---

## One command (normative)

```bash
# Full export — BLOCKED for Everon until import phase P10 (see phased import spec)
make map-export TERRAIN=everon PHASE=P10_full

# Normal development: one phase at a time (cumulative)
make map-export TERRAIN=everon PHASE=P1_buildings
make map-verify-phase TERRAIN=everon PHASE=P1_buildings   # MUST exit 0 before P2

# Equivalent
./scripts/map-assets/export-terrain.sh everon --phase P1_buildings
```

### What `--all` runs (in order)

| Step | Script / plugin | Output |
|------|-----------------|--------|
| 1 | `TBD_TerrainExportPlugin.c` (DEM) | `dem/*-dem-16bit.png` |
| 2a | `TBD_TerrainWorldExportPlugin.c` — **Satellite** pass | `tiles/satellite/`, `export/raw-entities.jsonl` |
| 2b | Same plugin — **Map** cartographic pass | `tiles/map/` |
| 2c | `build-tile-pyramid.sh` (×2) | WebP z/x/y under both pyramids |
| 3 | `classify-prefab.ts` + `packages/tbd-schema/rules/prefab-classify.json` | `objects/prefabs.json.gz` |
| 4 | `build-catalog-v1.ts` | `objects/chunks/*.json.gz`, `objects/roads.json.gz` |
| 5 | `run-z-audit.ts` | `objects/z-audit.json` |
| 6 | `run-geometry-audit.ts` | `objects/z-audit-geometry.json` |
| 7 | `validate-manifest.sh` + `make schema-validate` | exit 0 or fail fast |
| 8 | `write-export-ops-log.ts` | `.ai/artifacts/map_export_{terrainId}.json` |

**Fail fast:** any step non-zero → abort; partial artifacts marked stale in ops log.

---

## Multi-terrain standard (never fork per map)

### Directory layout (locked)

```text
packages/map-assets/
  terrain-registry.json          # master list — single source of truth
  everon/
    manifest.json                # terrain-manifest.schema.json
    dem/
    tiles/satellite/{z}/{x}/{y}.webp
    tiles/map/{z}/{x}/{y}.webp
    objects/prefabs.json.gz
    objects/chunks/...
    objects/roads.json.gz
    anchors/verification.json
  arland/
    ... same layout ...
```

**Rule:** Adding a map = **(1)** add `terrains.ts` bounds row, **(2)** add `terrain-registry.json` entry, **(3)** `make map-export TERRAIN=arland`. No new export scripts.

### `terrain-registry.json`

```json
{
  "schemaVersion": "1.0.0",
  "terrains": [
    {
      "terrainId": "everon",
      "displayName": "Everon",
      "worldBoundsM": [0, 0, 12800, 12800],
      "manifestPath": "everon/manifest.json",
      "workbenchWorld": "Worlds/Everon/Everon.ent",
      "exportProfile": "default",
      "status": "active"
    },
    {
      "terrainId": "arland",
      "displayName": "Arland",
      "worldBoundsM": [0, 0, 4096, 4096],
      "manifestPath": "arland/manifest.json",
      "workbenchWorld": "Worlds/Arland/Arland.ent",
      "exportProfile": "default",
      "status": "queued"
    }
  ]
}
```

Export script reads registry — **never** hard-code Everon paths in logic.

---

## Workbench plugin (one button inside WE)

**Plugin:** `TBD_TerrainWorldExportPlugin.c` (ships with T-090.3)

| UI action | Effect |
|-----------|--------|
| **TBD → Export terrain assets** | Writes raw export bundle to configured output dir (gitignored staging) |
| Sub-options (optional) | `[x] DEM  [x] Satellite tiles  [x] Map tiles  [x] World objects` — default all on |

Headless path for CI/agents when Workbench MCP available:

```bash
./scripts/map-assets/workbench-export.sh everon
# invokes MCP / CLI wrapper → same raw bundle as GUI button
```

After raw bundle lands, **Node steps 3–8** run with no Workbench — repeatable on any machine with the staging files.

---

## Classification rules (learn once, apply everywhere)

**File:** `packages/tbd-schema/rules/prefab-classify.json`

- Glob / regex rules on `resourceName` → `{ kind, class, aiSummary, taxonomyPath }`
- Versioned; grows when new prefabs appear — **not** duplicated per terrain
- Unmatched prefabs → `kind: prop`, `class: unknown`, `needsReview: true` in export report

**AI agent workflow for new prefabs:**

1. Export flags `needsReview` prefabs in ops log.
2. Agent adds rule row to `prefab-classify.json` (one line per prefab pattern).
3. Re-run `make map-export TERRAIN=*` — no manual re-tagging of 1M instances.

Human override file (rare): `packages/tbd-schema/rules/prefab-overrides.json` keyed by exact `resourceName`.

---

## AI-readable metadata (mandatory on every prefab)

Every row in `prefabs.json.gz` **must** include an `ai` block so LLM agents can reason without guessing:

```json
{
  "prefabId": 42,
  "resourceName": "{ABC}Prefabs/Vegetation/Tree_Pinus_spp_M_01.et",
  "kind": "tree",
  "class": "conifer",
  "label": "Pine tree (medium)",
  "ai": {
    "summary": "Medium conifer pine tree; static map decoration; blocks soft cover.",
    "taxonomyPath": "vegetation/tree/conifer",
    "edenEquivalent": "Nature tree (simple)",
    "gameplayTags": ["cover-soft", "destructible"],
    "classificationSource": "rules-v1/prefab-name",
    "confidence": 0.92,
    "needsReview": false
  },
  "bounds": { "model": "obb", "halfExtentsM": { "x": 1, "y": 1, "z": 6 }, "pivot": "base" },
  "render": { "iconKey": "tree-conifer", "lodMinZoom": 3 }
}
```

| Field | Purpose for AI |
|-------|----------------|
| `ai.summary` | Plain-language identity (1–2 sentences) |
| `ai.taxonomyPath` | Hierarchical path for filters/tools |
| `ai.edenEquivalent` | UX parity reference |
| `ai.gameplayTags` | Optional semantics (cover, ladder, door, …) |
| `ai.classificationSource` | Traceability (`rules-v1`, `override`, `llm-batch`) |
| `ai.confidence` | 0–1; `< 0.7` → `needsReview: true` |
| `ai.needsReview` | Export report queue for rule fixes |

**Resolved instance for AI** (document in ops log sample):

```json
{
  "id": "ent-99102",
  "prefabId": 42,
  "x": 5120.5,
  "y": 4800.0,
  "z": 138.2,
  "rotationDeg": 90,
  "_resolved": {
    "kind": "tree",
    "class": "conifer",
    "label": "Pine tree (medium)",
    "aiSummary": "Medium conifer pine tree; ..."
  }
}
```

`_resolved` is **documentation / ops-log only** — not stored per instance in production files.

---

## AI ops log (after every export)

**Path:** `.ai/artifacts/map_export_{terrainId}.json`

```json
{
  "terrainId": "everon",
  "exportedAt": "2026-06-26T18:00:00Z",
  "command": "make map-export TERRAIN=everon",
  "steps": [
    { "name": "dem", "status": "ok", "durationMs": 120000 },
    { "name": "tiles_satellite", "status": "ok", "durationMs": 180000 },
    { "name": "tiles_map", "status": "ok", "durationMs": 180000 },
    { "name": "catalog", "status": "ok", "prefabCount": 8420, "instanceCount": 1048576 },
    { "name": "z-audit", "status": "ok", "failCount": 16576 },
    { "name": "validate", "status": "ok" }
  ],
  "needsReviewPrefabs": 12,
  "artifactPaths": {
    "manifest": "packages/map-assets/everon/manifest.json",
    "prefabs": "packages/map-assets/everon/objects/prefabs.json.gz"
  },
  "aiInstructions": "Read manifest.json + prefabs.json.gz ai blocks. Stream instance chunks by cx,cy. Do not load full 1M array."
}
```

Agents **start here** when working on map data.

---

## Makefile targets (to implement @ T-090.3)

```makefile
map-export: ## Export all map assets for TERRAIN=everon|arland|...
	./scripts/map-assets/export-terrain.sh $(TERRAIN) --all

map-export-all: ## Export all terrains in terrain-registry.json
	./scripts/map-assets/export-all-terrains.sh

map-export-validate: ## Validate manifests + golden only (CI)
	./scripts/map-assets/validate-all-terrains.sh
```

---

## Verification (pipeline-level)

| ID | Check | Pass |
|----|-------|------|
| P1 | `make map-export TERRAIN=everon` exit 0 on machine with Workbench staging | ops log |
| P2 | Arland row in registry exports with **same** steps (smaller bounds) | script |
| P3 | Every prefab has `ai.summary` + `ai.taxonomyPath` | schema script |
| P4 | Re-run export → deterministic prefabId ordering (git diff only counts) | diff |
| P5 | Agent reads ops log + resolves 3 sample instances without repo grep | manual |

---

## Forest regions + tile cache + multi-terrain (N5 / N10 / L5)

- **Forest derivation step** (after `build-catalog-v1` trees): `derive-forest-regions.ts` writes
  `objects/forest-regions.json.gz` — engine mask if the T-090.3.0 spike proved one
  (`source: engine-mask`), else grid-density + concave/alpha hull (`source: derived-hull`, mandatory
  fallback). Reconcile `Σ treeCount` ±2 % vs `type-inventory.json`; remainder → `unassignedTrees`.
- **Tile cache & storage (N10 — single source, identical to [`t090_basemap_dual_view.md`](t090_basemap_dual_view.md)):**

  | Item | Value |
  |------|-------|
  | One pyramid (Everon tile zoom 0–5 WebP q≈80) | 200–400 MB LFS |
  | Dual pyramid (Satellite + Map) | 400–800 MB LFS |
  | Max concurrent tile fetches | 6 |
  | Tile LRU cache | 512 tiles (~32 MB) |
  | Cold first paint | grid + hillshade ≤500 ms; tiles stream; forest regions ≤3 s @ deckZoom −2 |
  | Runtime | only one basemap pyramid mounted at a time |

- **L5 — Arland / multi-terrain:** identical pipeline + folder layout for any registry terrain. A terrain
  with no `objects/` export (Arland today) shows the editor "world data not exported yet" state
  (T-090.9), not an error.

## Out of scope

- Mission-authored entities (slots) — Y.Doc / compiler
- T-110 binary compile — future consumer of catalog v1
- LLM auto-classify unknown prefabs in loop — optional follow-on; rules file is v1

---

## Related

- [`t090_3_map_asset_export.md`](t090_3_map_asset_export.md) — artifact detail
- [`t090_2_map_object_taxonomy.md`](t090_2_map_object_taxonomy.md) — catalog v1 schema
- [`t091_0_dem_tile_export.md`](t091_0_dem_tile_export.md) — DEM plugin pattern
- [`t121_terrain_dem_export_automation.md`](t121_terrain_dem_export_automation.md) — DEM refresh only
