# T-090.8 — Forest & vegetation regions (first-class areas)

**Ticket:** T-090 · **Slice:** T-090.8
**Status:** Spec ready (depends on **T-090.2** taxonomy + **T-090.3** export)
**Executor:** **claude-code**
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md) · render [`t090_render_lod_contract.md`](t090_render_lod_contract.md) · N5

---

## In one sentence

Forests (and fields, water bodies) are **first-class queryable areas** — exported as polygon regions,
rendered as translucent fills at low/mid Deck zoom and dissolving to per-tree glyphs only when zoomed in
— so Everon's 400k–900k trees read as forest at a glance (simpler than Eden) yet stay typed, counted and
AI-queryable (deeper than Eden). **Not optional, not cluster-only.**

---

## Problem (GAP-001)

Trees as 1M points give two bad defaults at the opening view (Deck zoom −2): either nothing (glyphs
gated above) or ~900k stacked icons (perf death + visual mud). A vegetation mass must read as an **area**.

---

## Region type (N5 — taxonomy + enums)

`kind` gains **`forest`**, **`field`**, **`waterBody`** (`map-object-enums.schema.json`). Regions are not
prefab instances; they live in their own artifact and schema
([`map-object-region.schema.json`](../../../packages/tbd-schema/schema/map-object-region.schema.json)).

| Field | Required | Notes |
|-------|----------|-------|
| `id` | yes | stable region id |
| `kind` | yes | `forest` \| `field` \| `waterBody` |
| `polygon` | yes | one or more rings `[[x,y],…]` in world meters |
| `treeCount` | forest | trees inside the region |
| `dominantSpeciesClass` | forest | `forestClass` enum (conifer/deciduous/mixed/palm/dead/unknown) |
| `densityPerHa` | forest | trees per hectare |
| `areaHa` | yes (forest/field) | polygon area in hectares |
| `coverType` | yes | `none` \| `soft` \| `hard` |
| `source` | yes | `engine-mask` \| `derived-hull` |

Golden: [`packages/tbd-schema/golden/map-objects/map-object-regions-everon-sample.json`](../../../packages/tbd-schema/golden/map-objects/map-object-regions-everon-sample.json).

---

## Export (two normative paths — N5/D2)

Artifact: **`objects/forest-regions.json.gz`** (manifest `objects.regionsPath`) — **required** after the
tree phase (P2 → P2b).

- **Path A — engine mask (`source: engine-mask`):** if the **T-090.3.0** spike proves Reforger Workbench
  exposes a vegetation/forest generator layer or foliage mask, ingest its polygons directly (truer +
  cheaper). The spike documents presence/absence per terrain.
- **Path B — derived hull (`source: derived-hull`, mandatory fallback):** always specified so forests
  work without engine support:
  1. Bin `kind=tree` instances into a grid (default **32 m** cells); mark cells with density ≥ threshold.
  2. Connected-component the dense cells; for each component compute a **concave/alpha hull** ring.
  3. Aggregate `treeCount`, `dominantSpeciesClass` (mode of member species), `densityPerHa`, `areaHa`.

### Reconciliation (verifiable — exact at ship)

**Ship gate:** `byRegionKind.forest.treeCount + unassignedTrees = byKind.tree.instances` (exact integer equality from `type-inventory.json`).

During P2 **development**, hull assignment may temporarily miss up to **±2%** of trees before `unassignedTrees` is populated — that tolerance is **not** a substitute for exact totals in docs or verify gates. Every tree must end in either a region or `unassignedTrees`.

---

## Render (Deck zoom — see N3 master table)

Forests follow the canonical ladder in [`t090_render_lod_contract.md`](t090_render_lod_contract.md) §N3;
do not restate the numbers. Summary by Deck orthographic zoom: at deckZoom ≤ `FOREST_REGION_MAX_ZOOM`
(1) forests are translucent **`PolygonLayer`** fills (`rgba(34,120,60,α)`); above +1 the fill fades to
context and per-tree glyphs take over (deckZoom ≥ `TREE_GLYPH_MIN_ZOOM` per LOD v2).
Fields render as a lighter hatch; water bodies as a blue fill.

---

## Interaction (read-only — see T-090.9)

- **Hover** a forest polygon → tooltip, exact template:
  `"Mixed {dominantSpeciesClass} forest · ~{treeCount} trees · {areaHa} ha · {coverType} cover"`.
- **Click** → inspect panel: species breakdown + density + area + **"Ask AI about this area"** (feeds the
  region summary, **not** 12k individual trees).
- Picking uses the **separate world** spatial index in the worker; never a Deck GPU pick.

---

## AI (region summaries only — GAP-M6)

`queryWorldObjectsInRect` / `getAiContextPack` return **region summaries** for forest areas, never an
enumeration of the member trees. A bbox over a forest yields one region row (≤ a few hundred bytes), so
the 256 KB / 50-object context-pack budget (T-090.7) holds at 1M scale.

---

## Per-phase budget (N11 — P2b)

| Phase | ~instances | max load ms | max resident MB | min fps @ deckZoom −2 | eviction |
|-------|------------|-------------|-----------------|----------------------|----------|
| P2 trees | `byKind.tree.instances` from inventory | 8000 | 180 | 55 | forest regions required; chunk LRU 8 |
| **P2b forest regions** | derived | 3000 | +20 | 55 | region index pinned |

Forest regions are **required before P2 ships** — trees alone may not be the default low-zoom render.

---

## Verification

| ID | Check | Pass |
|----|-------|------|
| F1 | Region golden validates `map-object-region.schema.json` | `make schema-validate` |
| F2 | `forest.treeCount + unassignedTrees = byKind.tree.instances` (exact) | `verify-type-inventory.mjs` |
| F3 | At deckZoom −2 a forest renders as a filled polygon (no per-tree icons drawn inside) | vitest (T-090.5) |
| F4 | Hover tooltip matches the exact template string | vitest |
| F5 | `dominantSpeciesClass ∈ forestClass` enum | `make map-object-enums-verify` |
| F6 | Path B derivation reproducible (same instances → same rings) | unit |

---

## Out of scope
- Editing/moving trees or forest boundaries (Workbench only).
- 3D canopy. Per-tree species at low zoom (regions summarize).

## Related
- [`t090_2_map_object_taxonomy.md`](t090_2_map_object_taxonomy.md) · [`t090_3_map_asset_export.md`](t090_3_map_asset_export.md)
- [`t090_render_lod_contract.md`](t090_render_lod_contract.md) · [`t090_9_world_object_interaction.md`](t090_9_world_object_interaction.md)
- [`t090_world_object_type_inventory.md`](t090_world_object_type_inventory.md)
