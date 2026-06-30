# T-090 — Eden AI world object schema (exact field contract)

**Status:** Spec ready — **authority for AI-in-Editor integration**  
**Ticket:** T-090 · **Schema slice:** T-090.2 · **Runtime slice:** T-090.7 (queued)  
**Audience:** AI agents embedded in Mission Creator + export/classification pipeline  
**Storage:** [`t090_2_map_object_taxonomy.md`](t090_2_map_object_taxonomy.md) catalog v1 · **Join:** prefab + instance → **`ResolvedWorldObject`**

---

## Purpose

Mission Creator will expose **AI inside the Eden-style editor**. The AI must read the **world base layer** (1M+ map objects) with the same certainty a human gets from selecting an entity in Workbench:

- **What** is it? (type, label, plain summary)
- **Where** is it? (x/y/z, rotation, terrain cell)
- **How big** is it? (bounds, footprint, height/width)
- **What does it do tactically?** (cover soft/hard/none, blocks movement, blocks LOS, destructible, …)
- **Can I trust placement?** (buried/floating flags from audit)

This doc defines the **exact fields** — required, typed, validated — not loose tags alone.

---

## Canonical shape: `ResolvedWorldObject`

Every AI tool receives this shape (computed at runtime or materialized in ops-log samples). **Do not** invent parallel field names in frontend AI code.

```typescript
/** Join of catalog instance + prefab + optional audit overlay */
type ResolvedWorldObject = {
  // ── Identity (required) ──
  id: string;
  prefabId: number;
  resourceName: string;
  kind: WorldObjectKind;
  class: string; // roadClass | speciesClass | buildingClass | …
  label: string;
  taxonomyPath: string; // e.g. vegetation/tree/conifer
  summary: string; // plain language, 1–2 sentences

  // ── Placement (required on instances) ──
  x: number; // world m, editor x
  y: number; // world m, editor y (= mod z)
  z: number | null; // ASL m; null if unknown
  rotationDeg: number; // yaw 0–360
  pitchDeg?: number;
  rollDeg?: number;

  // ── Spatial / size (required unless kind exempt — see matrix) ──
  spatial: WorldObjectSpatial;

  // ── Gameplay semantics (required — use kind defaults if unknown) ──
  gameplay: WorldObjectGameplay;

  // ── Geometry extensions (kind-specific) ──
  geometry?: LineGeometry | PolygonGeometry;

  // ── Placement quality (from T-090.4 / T-090.6 when available) ──
  placement?: PlacementQuality;

  tags: string[];
};
```

---

## Layer 1 — Identity (all objects)

| Field | Type | Required | Source | AI use |
|-------|------|----------|--------|--------|
| `id` | string | yes | instance | Stable reference for **read-only** inspect / AI context (mutation is **Workbench-only** — never edited in MC) |
| `prefabId` | integer | yes | instance | Join key |
| `resourceName` | string | yes | prefab | Mod prefab path |
| `kind` | enum | yes | prefab | Top-level filter |
| `class` | string | yes | prefab | **Closed enum per `kind`** — e.g. `buildingClass` (14 values). See [`t090_world_object_type_inventory.md`](t090_world_object_type_inventory.md) |
| `label` | string | yes | prefab | UI + NL (“Pine tree medium”) |
| `taxonomyPath` | string | yes | prefab.`ai` | Hierarchical reasoning |
| `summary` | string | yes | prefab.`ai` | NL context window |

---

## Layer 2 — Placement (all instances)

| Field | Type | Required | Precision | Notes |
|-------|------|----------|-----------|-------|
| `x` | number | yes | 0.001 m | |
| `y` | number | yes | 0.001 m | |
| `z` | number \| null | yes* | 0.001 m | *Required after export; null only pre-audit |
| `rotationDeg` | number | yes | 0.1° | Yaw; default `0` |
| `pitchDeg` | number | no | 0.1° | From instance or prefab default |
| `rollDeg` | number | no | 0.1° | |

**Chunk index for queries:** `cx = floor(x / chunkSizeM)`, `cy = floor(y / chunkSizeM)`.

---

## Layer 3 — Spatial / size (`spatial` block)

Stored primarily on **prefab**; instance rotation applied at resolve time for OBB.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `spatial.model` | `aabb` \| `obb` \| `line` \| `polygon` | yes | `line` = roads; `polygon` = building footprint |
| `spatial.pivot` | `base` \| `center` | yes | Where x,y,z attach |
| `spatial.halfExtentsM` | `{ x, y, z }` | yes* | *Except `line` — use widthM |
| `spatial.widthM` | number | roads | Carriageway width |
| `spatial.heightM` | number | yes | Total vertical extent (AI: “tall enough for cover?”) |
| `spatial.footprintM2` | number | buildings | Horizontal area |
| `spatial.footprint` | polygon ring | buildings | Local or world — document in `geometry` if world |

**AI rules of thumb (documented defaults when export missing):**

| kind | Default heightM | Default cover inference |
|------|-----------------|-------------------------|
| `tree` | 6–12 m from bounds | soft if heightM ≥ 2 |
| `building` | from bounds | hard if footprintM2 ≥ 20 |
| `rock` | 1–3 m | hard if heightM ≥ 1 |
| `prop` | from bounds | from `gameplay.cover` only |
| `road` | 0 | none |

---

## Layer 4 — Gameplay semantics (`gameplay` block)

**Required on every prefab** after classification. AI uses this for tactical reasoning — not optional string tags alone.

```json
{
  "gameplay": {
    "cover": {
      "type": "soft",
      "standing": true,
      "prone": true,
      "crouch": true,
      "heightM": 2.5
    },
    "movement": {
      "blocksInfantry": false,
      "blocksVehicle": false
    },
    "lineOfSight": "partial",
    "destructible": true,
    "climbable": false,
    "enterable": false,
    "ladder": false,
    "door": false,
    "bridge": false,
    "runway": false
  }
}
```

### `cover.type` (required enum)

| Value | Meaning | AI prompt hint |
|-------|---------|----------------|
| `none` | No combat cover | Open ground, thin props |
| `soft` | Concealment / vegetation / light materials | Bushes, small trees, fences |
| `hard` | Ballistic cover | Walls, rocks, bunkers, large trunks |

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `cover.type` | `none` \| `soft` \| `hard` | **yes** | Primary tactical field |
| `cover.standing` | boolean | yes | Standing soldier protected |
| `cover.prone` | boolean | yes | |
| `cover.crouch` | boolean | yes | |
| `cover.heightM` | number | yes | Effective cover height @ this prefab |

### `movement` + `lineOfSight`

| Field | Type | Required |
|-------|------|----------|
| `movement.blocksInfantry` | boolean | yes |
| `movement.blocksVehicle` | boolean | yes |
| `lineOfSight` | `none` \| `partial` \| `full` | yes — `full` = opaque barrier |

### Capability flags (required booleans — default false)

`destructible`, `climbable`, `enterable`, `ladder`, `door`, `bridge`, `runway`

Populate from rules + Enfusion export metadata when available; else kind defaults below.

---

## Kind-specific required field matrix

| Field / block | tree | building | rock | prop | road | utility | vegetation |
|---------------|------|----------|------|------|------|---------|------------|
| `summary` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ |
| `spatial.halfExtentsM` | ✓ | ✓ | ✓ | ✓ | — | ✓ | ✓ |
| `spatial.heightM` | ✓ | ✓ | ✓ | ✓ | 0 | ✓ | ✓ |
| `spatial.footprintM2` | — | ✓ | — | — | — | — | — |
| `geometry` line | — | — | — | — | ✓ | optional | — |
| `gameplay.cover` | ✓ | ✓ | ✓ | ✓ | none | ✓ | ✓ |
| `gameplay.enterable` | — | ✓ | — | — | — | — | — |

---

## Kind defaults (`gameplay` when rules unmatched)

| kind | `cover.type` | `blocksInfantry` | `lineOfSight` | Notes |
|------|--------------|------------------|---------------|-------|
| `tree` | `soft` | false | `partial` | `cover.heightM` ≈ min(6, spatial.heightM) |
| `building` | `hard` | true | `full` | `enterable: true` if door heuristic |
| `rock` | `hard` | true | `full` | |
| `prop` | `none` | false | `none` | Rule override common |
| `road` | `none` | false | `none` | `runway` flag on runway class |
| `utility` | `none` | false | `partial` | Power poles: soft if tall |

**Unknown prefab:** `cover.type: none`, all flags false, `needsReview: true` on prefab.`ai`.

---

## Layer 5 — Placement quality (`placement` block)

Attached at resolve time from audit artifacts (not stored per instance in catalog).

| Field | Type | Source |
|-------|------|--------|
| `placement.zDeltaM` | number | T-090.4 |
| `placement.severity` | `ok` \| `warn` \| `fail` | T-090.4 / T-090.6 |
| `placement.visibleAboveGroundPct` | number | T-090.6 |
| `placement.maxBurialM` | number | T-090.6 |

AI should **warn** when suggesting spawn near `severity: fail` objects.

---

## Storage mapping (catalog v1 → resolved)

| ResolvedWorldObject field | Prefab | Instance | Audit |
|---------------------------|--------|----------|-------|
| Identity | ✓ | id only | — |
| Placement | defaults | ✓ | — |
| `spatial` | ✓ | rotation applied | — |
| `gameplay` | ✓ | tag overrides only | — |
| `geometry` (roads) | template | world points | — |
| `placement` | — | — | ✓ |

**Prefab must include `gameplay` object** (not only `ai.gameplayTags`). Tags are **deprecated** for AI logic — migrate to structured `gameplay`.

### Normative prefab excerpt

```json
{
  "prefabId": 42,
  "resourceName": "{GUID}Prefabs/Vegetation/Tree_Pinus_M.et",
  "kind": "tree",
  "class": "conifer",
  "label": "Pine tree (medium)",
  "ai": {
    "summary": "Medium conifer; soft cover to ~4 m standing.",
    "taxonomyPath": "vegetation/tree/conifer",
    "classificationSource": "rules-v1/prefab-name",
    "confidence": 0.92,
    "needsReview": false
  },
  "spatial": {
    "model": "obb",
    "pivot": "base",
    "halfExtentsM": { "x": 1.2, "y": 1.2, "z": 6 },
    "heightM": 12
  },
  "gameplay": {
    "cover": { "type": "soft", "standing": true, "prone": true, "crouch": true, "heightM": 4 },
    "movement": { "blocksInfantry": false, "blocksVehicle": false },
    "lineOfSight": "partial",
    "destructible": true,
    "climbable": false,
    "enterable": false,
    "ladder": false,
    "door": false,
    "bridge": false,
    "runway": false
  }
}
```

---

## Editor runtime API (T-090.7 — AI read path)

Implemented in Mission Creator frontend; **reads same resolved shape**.

| API | Returns | Use |
|-----|---------|-----|
| `resolveWorldObject(id)` | `ResolvedWorldObject` | Single selection context |
| `queryWorldObjectsInRect(bbox, filter?)` | `ResolvedWorldObject[]` | “What cover is near this squad?” |
| `queryWorldObjectsNear(x, y, radiusM, filter?)` | `ResolvedWorldObject[]` | Radius search |
| `getAiContextPack(ids[])` | `{ objects, terrainId, chunkRefs }` | LLM tool payload |
| `filterByCover(type)` | ids[] | Tactical queries |

**Filter object:**

```typescript
type WorldObjectFilter = {
  kinds?: WorldObjectKind[];
  classes?: string[];
  coverType?: 'none' | 'soft' | 'hard';
  blocksInfantry?: boolean;
  maxPlacementSeverity?: 'ok' | 'warn' | 'fail';
};
```

Loads: global `prefabs` + streamed **chunks** only — never full 1M array.

### Region queries + context-pack cap (N5 / GAP-M6)

For area kinds (`forest`/`field`/`waterBody`), `queryWorldObjectsInRect` / `getAiContextPack` return
**region summaries** — never an enumeration of the member trees. A bbox over a forest yields one region
row, so the **256 KB / ≤50-object** budget (A5) holds at 1M scale. New query `queryRegionsInRect(bbox,
kinds?)` → region rows. The **same** resolver feeds the human inspect panel + "Ask AI about this
object/area" in [`t090_9_world_object_interaction.md`](t090_9_world_object_interaction.md) — one shared
`WorldObjectFilter`, one path for AI and UI.

---

## JSON Schema deliverables (T-090.2)

| File | Contents |
|------|----------|
| `map-object-catalog.schema.json` | prefabs + instances + roads |
| `map-object-prefab.schema.json` | **`spatial` + `gameplay` + `ai` required** |
| **`map-object-enums.schema.json`** | **All closed enums: `buildingClass`, `roadClass`, `speciesClass`, `propClass`, …** |
| `map-object-resolved.schema.json` | **`ResolvedWorldObject`** — AI + T-090.7 contract |
| `packages/tbd-schema/golden/map-objects-everon-sample.json` | ≥1 row per **`buildingClass`** + per kind |
| `packages/tbd-schema/rules/prefab-classify.json` | Rules emit `gameplay` + valid `class` |
| **`{terrainId}/objects/type-inventory.json`** | **Ground truth prefab/instance counts** — [`t090_world_object_type_inventory.md`](t090_world_object_type_inventory.md) |

---

## Verification

| ID | Check |
|----|-------|
| A1 | Golden validates against `map-object-resolved.schema.json` (materialized samples) |
| A2 | Every prefab in golden has `gameplay.cover.type` |
| A3 | Tree/building/road examples match kind matrix |
| A4 | `resolveWorldObject()` TS types match schema (codegen) |
| A5 | AI context pack ≤ 256 KB for ≤50 objects (perf budget) |

---

## Program placement

```text
T-090.2  schema + golden + classify rules (this contract)
T-090.3  export populates spatial + gameplay on prefabs
T-090.5  render uses spatial/render hints
T-090.7  Eden AI read API in Mission Creator (query + context pack)
```

---

## Related

- [`t090_2_map_object_taxonomy.md`](t090_2_map_object_taxonomy.md)
- [`t090_terrain_export_pipeline.md`](t090_terrain_export_pipeline.md)
- [`t090_eden_map_reference.md`](t090_eden_map_reference.md)
- [`t090_world_object_type_inventory.md`](t090_world_object_type_inventory.md) — enum catalog + **type counts**
