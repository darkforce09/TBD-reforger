# T-090.2 — Map object taxonomy + schema

**Ticket:** T-090 · **Slice:** T-090.2  
**Status:** **SHIPPED** @ `691d9b26` (tag **T-090.2**) — S1–S10 goldens + `verify-map-object-golden.mjs` + classify rules + Everon `objects` stub  
**Blocked for:** visual render verify (**T-090.5**); Workbench export counts (**T-090.3**) — not blocking schema/golden ship  
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md)  
**Handoff:** [`.ai/artifacts/t090_2_claude_code_handoff.md`](../../../.ai/artifacts/t090_2_claude_code_handoff.md)

---

## Bootstrap @ T-090.0.2 (shipped — Claude Code must not redo)

**T-090.0.2** (cursor-docs) landed the schema scaffolding and partial goldens. **T-090.2** completes **acceptance S1–S10**, not a second schema rewrite.

| Delivered @ bootstrap | Path |
|----------------------|------|
| Enum single-source | `schema/map-object-enums.schema.json` |
| Prefab / instance / region / roads / catalog / resolved / type-inventory schemas | `schema/map-object-*.schema.json` |
| AJV wiring | `packages/tbd-schema/scripts/validate.mjs` |
| Enum drift gate (S10) | `scripts/verify-map-object-enums.mjs` |
| Glyph gate | `scripts/verify-map-glyphs-manifest.mjs` |
| Partial goldens | `golden/map-objects/*` |
| Classify rules v1 | `rules/prefab-classify.json` |
| Census stub | `scripts/map-assets/census-types.mjs` + `everon/objects/type-inventory.json` (`pending_export`) |

**Baseline:** `make schema-validate` exit 0 @ `0418d952`.

---

## In one sentence

Define a **machine-readable taxonomy** for **any terrain’s** world objects (roads, trees, buildings, props) so exports, AI agents, and the editor render layer speak one vocabulary — including **AI metadata on every prefab**, **road/tree/building subtypes**, deduplicated **catalog v1** storage @ **1M+**, and **zero per-map rework** via the [`t090_terrain_export_pipeline.md`](t090_terrain_export_pipeline.md) one-command pipeline.

---

## Problem

T-090.1 adds pretty tiles. Mission makers still cannot see **individual world props** (roads, structures, trees) or filter them. Without a typed schema, export scripts and Claude Code cannot reliably label “this is a paved highway” vs “dirt track” vs “oak tree cluster.”

---

## Goal

1. Publish **`map-object-catalog.schema.json`**, **`map-object-prefab.schema.json`**, **`map-object-resolved.schema.json`**, **`map-object-enums.schema.json`** in `packages/tbd-schema/schema/`.
2. Lock **closed enums** for all **`buildingClass`** (+ road/tree/prop/…) — §Taxonomy + [`t090_world_object_type_inventory.md`](t090_world_object_type_inventory.md).
3. Lock **Eden AI field contract** — [`t090_eden_ai_world_object_schema.md`](t090_eden_ai_world_object_schema.md).
4. Ship **golden fixture** with ≥1 example **per `buildingClass`** and per other kind class.
5. **`type-inventory.json`** census script — discover exact prefab/instance counts @ export.
6. Extend **`terrain-manifest.schema.json`** with optional `objects` block.
7. Update **`prefab-classify.json`** rules to emit structured `gameplay` + correct `class` enum + **`render.iconKey`**.
8. **Glyph catalog** — SVG per class + atlas build — [`t090_world_object_glyphs.md`](t090_world_object_glyphs.md) (T-090.5).
9. **No render code** — T-090.5. **No editor AI API** — T-090.7.

---

## Taxonomy (normative)

### Top-level `kind` (required on every instance)

| `kind` | Description | Eden analogue |
|--------|-------------|---------------|
| `road` | Trajectory or polyline segment | Road network |
| `building` | Structure with footprint | House, barn, bunker shell |
| `tree` | Single tree or stump | Vegetation entity |
| `vegetation` | Bush, grass patch, cluster proxy | Non-tree flora |
| `rock` | Static rock / cliff prop | Terrain decoration |
| `prop` | Generic static object (fence, sign, wire) | Misc map object |
| `utility` | Powerline, lamp post, antenna | Infrastructure |
| `water` | Pier, dock, river volume marker | Hydro (optional) |
| `forest` | **Region** — derived/engine forest area (polygon) | Vegetation mass |
| `field` | **Region** — open field area (polygon) | Land cover |
| `waterBody` | **Region** — lake/sea polygon | Hydro area |

### Region kinds (N5 — first-class areas)

`forest`, `field`, `waterBody` are **region** kinds: polygons, not prefab instances. They live in
`objects/forest-regions.json.gz` against
[`map-object-region.schema.json`](../../../packages/tbd-schema/schema/map-object-region.schema.json) and
render as fills at low/mid deckZoom (see [`t090_8_forest_vegetation_regions.md`](t090_8_forest_vegetation_regions.md),
[`t090_render_lod_contract.md`](t090_render_lod_contract.md) §N3). Forests carry `treeCount`,
`dominantSpeciesClass` (`forestClass`), `densityPerHa`, `areaHa`, `coverType`, `source`. This is the
answer to "900k trees must read as forest at the default view" (GAP-001).

### Enums are single-source (N8/GAP-M5)

Every `kind`/`class` value is a member of [`map-object-enums.schema.json`](../../../packages/tbd-schema/schema/map-object-enums.schema.json);
prefab rows, the glyph manifest and `prefab-classify.json` are checked against it by
`make map-object-enums-verify`. Do not redeclare enum values in this doc.

### Building geometry (N6)

**Normative shipped geometry:** oriented bounding **rectangle** from `spatial.halfExtentsM` +
`rotationDeg`. Real **footprint polygon rings** are populated only when T-090.3.0 proves Enfusion
footprint export; when present, polygons supersede OBB rectangles for render.

### `road` — required subtype (`roadClass`)

| `roadClass` | Label | Typical width (m) | Render hint |
|-------------|-------|-------------------|-------------|
| `highway_paved` | Main paved highway | 8–14 | Wide light gray stroke |
| `road_paved` | Secondary paved | 5–8 | Medium gray |
| `road_dirt` | Dirt / gravel | 4–6 | Brown dashed |
| `track` | Field track | 2–4 | Narrow brown |
| `path` | Footpath | 1–2 | Dotted |
| `runway` | Airfield runway segment | 20+ | High-contrast white |
| `unknown` | Unclassified | — | Gray; flag for manual review |

Optional: `surface`, `lanes`, `bridge`, `tunnel` booleans.

### `tree` / `vegetation` — required `speciesClass`

| `speciesClass` | Notes |
|----------------|-------|
| `conifer` | Pine/spruce clusters |
| `deciduous` | Broadleaf |
| `palm` | Coastal |
| `bush` | Low vegetation |
| `dead` | Burned / winter |
| `unknown` | Exporter could not classify |

Optional: `canopyRadiusM`, `heightM` (from export or estimated).

### `building` — required `buildingClass` (closed enum)

**JSON Schema:** `class` when `kind=building` **must** be one of **`buildingClass`** below (`map-object-enums.schema.json`). Eden AI filters by this field — e.g. *“list all `military` buildings in bbox”*.

| `buildingClass` | Description | Typical `gameplay.cover` |
|-----------------|-------------|---------------------------|
| `residential` | Houses, apartments, cottages | hard |
| `commercial` | Shops, offices, restaurants | hard |
| `industrial` | Factories, large warehouses | hard |
| `agricultural` | Barns, farm sheds, greenhouses | hard |
| `civic` | Churches, town halls, schools | hard |
| `military` | Barracks, FOB buildings, checkpoints | hard |
| `bunker` | Pillboxes, hardened bunkers | hard |
| `hangar` | Aircraft / vehicle hangars | hard |
| `garage` | Vehicle garages, workshops | hard |
| `tower` | Watchtowers, built-up masts | hard |
| `ruin` | Destroyed / damaged shells | soft–hard |
| `bridge` | Road/rail bridges (footprint) | hard |
| `castle` | Castle / fort structures | hard |
| `lighthouse` | Lighthouses, beacons | hard |
| `shed` | Small sheds, outbuildings | hard |
| `container` | Shipping containers, crates (static) | hard |
| `tent` | Tents, canvas shelters (soft cover) | soft |
| `wall` | Compound walls, perimeter (footprint) | hard |
| `generic` | Building without finer rule match | hard |
| `unknown` | Unclassified — **needs review** | none until ruled |

Optional: `buildingSubClass` (string) — finer label e.g. `"apartment_block"`; `footprintM2`, `floorCount` (T-129), `heightM`.

**Ground truth counts** (how many of each class on Everon): **`type-inventory.json`** — see [`t090_world_object_type_inventory.md`](t090_world_object_type_inventory.md). Counts are **unknown until first export**; schema defines **allowed** types only.

### `rock` — required `rockClass`

| `rockClass` | Notes |
|-------------|-------|
| `boulder` | Large standalone rock |
| `cliff` | Cliff / rock face prop |
| `pebble` | Small ground rock |
| `scree` | Rock scatter / field |
| `unknown` | Unclassified |

### `prop` — required `propClass`

| `propClass` | Notes |
|-------------|-------|
| `fence` | Wire, chain, wooden fence |
| `barrier` | Jersey barrier, roadblock |
| `sign` | Signs, posts |
| `furniture` | Benches, tables (static) |
| `debris` | Rubble, wreckage |
| `buildingpart` | Composite-building part soup (off-map until props ship) |
| `pavement` | Paved surfaces, slabs |
| `rail` | Railway track props |
| `monument` | Cemeteries, calvaries, memorials |
| `composition` | Whole-POI composition prefabs (`World/Locations/*` — never rendered) |
| `pebble` | Small prop / stone |
| `unknown` | Unclassified |

### `utility` — required `utilityClass`

| `utilityClass` | Notes |
|----------------|-------|
| `powerline` | Poles, wires |
| `lamp` | Street lamps |
| `antenna` | Radio masts (non-tower) |
| `pipeline` | Pipes above ground |
| `unknown` | Unclassified |

### `water` — required `waterClass`

| `waterClass` | Notes |
|--------------|-------|
| `pier` | Pier / dock structure |
| `dock` | Mooring dock |
| `buoy` | Buoy marker |
| `unknown` | Unclassified |

### `vegetation` — extend `speciesClass`

| `speciesClass` | Notes |
|----------------|-------|
| `bush` | Low shrub |
| `grass` | Grass patch |
| `fern` | Fern / undergrowth |
| `dead` | Dead vegetation |
| `unknown` | Unclassified |

*(Tree `speciesClass` table unchanged above.)*

---

## AI-readable metadata (mandatory on prefabs)

**Requirement:** Any AI agent must understand **what a prefab is** without opening Enfusion or guessing from GUID paths. Type metadata lives on **`prefabs[]` only** (deduplicated); instances stay compact.

### `ai` block (required on every prefab @ export)

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `summary` | string | yes | 1–2 sentences — plain language identity + role on map |
| `taxonomyPath` | string | yes | Hierarchical path: `vegetation/tree/conifer`, `infrastructure/road/highway_paved`, … |
| `edenEquivalent` | string | no | Eden/Workbench category label for UX parity |
| `gameplayTags` | string[] | no | **Legacy** — prefer structured `gameplay` block (§below) |
| `classificationSource` | string | yes | `rules-v1/prefab-name` \| `override` \| `export-metadata` |
| `confidence` | number | yes | 0–1; rule match strength |
| `needsReview` | boolean | yes | `true` if unmatched or confidence < 0.7 |

Populated by [`prefab-classify.json`](../../../packages/tbd-schema/rules/prefab-classify.json) + overrides — see [`t090_terrain_export_pipeline.md`](t090_terrain_export_pipeline.md). **New maps reuse the same rules file**; only new prefab patterns get new rule rows.

### Example prefab row (normative shape)

```json
{
  "prefabId": 0,
  "resourceName": "{GUID}Prefabs/...",
  "kind": "tree",
  "class": "conifer",
  "label": "Pine tree (medium)",
  "ai": {
    "summary": "Medium conifer pine; static vegetation; soft cover.",
    "taxonomyPath": "vegetation/tree/conifer",
    "classificationSource": "rules-v1/prefab-name",
    "confidence": 0.92,
    "needsReview": false
  },
  "bounds": { "model": "obb", "pivot": "base", "halfExtentsM": { "x": 1, "y": 1, "z": 6 } }
}
```

### Agent read order

1. `packages/map-assets/terrain-registry.json` — which terrains exist  
2. `packages/map-assets/{terrainId}/manifest.json` — paths + counts  
3. `.ai/artifacts/map_export_{terrainId}.json` — last export ops log  
4. `objects/prefabs.json.gz` — full type + **ai** metadata (load once)  
5. `objects/chunks/{cx}_{cy}.json.gz` — stream instances; join via `prefabId`

**Never** require an agent to load all instance chunks at once.

### Structured `gameplay` + `spatial` (required — Eden AI)

Loose `gameplayTags` are **not sufficient** for in-editor AI. Every prefab **must** include:

| Block | Required fields | Spec |
|-------|-----------------|------|
| `spatial` | `model`, `pivot`, `heightM`, `halfExtentsM` (or road `widthM`) | [`t090_eden_ai_world_object_schema.md`](t090_eden_ai_world_object_schema.md) §Layer 3 |
| `gameplay` | `cover.type`, `cover.standing/prone/crouch`, `cover.heightM`, `movement.*`, `lineOfSight`, capability flags | Same doc §Layer 4 |

At runtime, **`ResolvedWorldObject`** = join(prefab, instance, optional audit). That is what the Eden AI integration reads.

---

## Storage architecture (normative — catalog v1)

**Problem:** Eden-scale worlds have **~1M map objects**. A naive JSON array of fat rows (repeating `resourceName`, `kind`, `class`, `bounds`, labels on every tree) explodes to **hundreds of MB** and cannot be `JSON.parse`’d in the browser.

**Solution:** Split **type data** (save once per prefab) from **placement data** (save per instance). Same pattern as game engines (archetype + transform) and T-110’s future binary base — catalog v1 is the **JSON interchange** format; T-110 may compile it to TypedArray later.

### Mental model

```text
prefabs[prefabId]  —  “what is this thing?” (shared, categorized, rich metadata)
instances[]        —  “where is it?” (compact transform + prefabId reference)
roadSegments[]     —  “where is the road?” (polyline geometry — cannot dedupe by prefab alone)
```

```mermaid
flowchart LR
  subgraph once [Saved once per prefab type]
    P[prefabs id]
    P --> kind
    P --> class
    P --> resourceName
    P --> bounds
    P --> render
  end
  subgraph many [Saved per placement]
    I[instances]
    I --> prefabId
    I --> x y z rotation
  end
  P -->|prefabId index| I
```

### On-disk layout (T-090 era)

| File | Contents | When |
|------|----------|------|
| `objects/prefabs.json.gz` | Full **prefab table** (~2k–20k unique types on Everon) | Always |
| `objects/instances/{cx}_{cy}.json.gz` | **Compact instances** for one spatial cell | When total > ~50k OR gzip > 30 MB |
| `objects/roads.json.gz` | **Road network** polylines | If roads exported separately |
| `objects/catalog.json.gz` | **Single bundle** `{ prefabs, instances, roadSegments }` | Small maps / golden fixtures only |

Manifest `objects` block records paths + counts (see [`t090_3_map_asset_export.md`](t090_3_map_asset_export.md)).

**Rule:** Production Everon export uses **prefabs + chunked instances**. Single monolithic `everon-objects.json.gz` is acceptable only for **golden samples** and CI.

### Top-level catalog document (`schemaVersion: "1.0.0"`)

```json
{
  "schemaVersion": "1.0.0",
  "terrainId": "everon",
  "prefabs": [ "..." ],
  "instances": [ "..." ],
  "roadSegments": [ "..." ]
}
```

All three arrays may be present; `roadSegments` optional if roads live in `instances` with inline `geometry`.

---

### Prefab record (type table — rich, deduplicated)

**One row per unique Enfusion prefab** (`resourceName` or stable hash). Expected **~2k–20k rows** on Everon vs **~1M instances**.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `prefabId` | integer | yes | **Stable index** into `prefabs[]` (0-based). Export sorts by `resourceName` for deterministic diffs. |
| `resourceName` | string | yes | `{GUID}Prefabs/...` — canonical key |
| `kind` | enum | yes | §Taxonomy top-level `kind` |
| `class` | string | yes | `roadClass` / `speciesClass` / `buildingClass` / … |
| `label` | string | no | Human name (“Pine tree medium”) |
| `ai` | object | **yes** | §AI-readable metadata — **required @ export** |
| `spatial` | object | **yes** | Size, bounds, height — Eden AI + T-090.6 |
| `gameplay` | object | **yes** | Cover, LOS, movement block — Eden AI tactical reasoning |
| `bounds` | object | no | **Deprecated name** — use `spatial` (keep alias until export ships) |
| `render` | object | yes* | *Required when kind uses IconLayer. See [`t090_world_object_glyphs.md`](t090_world_object_glyphs.md): `iconKey`, `baseSizePx`, `anchor`, `lodMinZoom` |
| `tags` | string[] | no | Default tags for all instances of this prefab |
| `footprint` | object | no | Building polygon template in **local space** (if identical for prefab) |

**Categorization lives on the prefab**, not repeated per instance. Filter UI (“show only `conifer`”) = filter instances whose `prefabId` resolves to matching `class`.

---

### Instance record (placement — compact)

**One row per placed object** (tree, building, prop, …). Reference prefab by index.

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `id` | string | yes | Stable world entity id (Enfusion id or export hash) — for audit + delta |
| `prefabId` | integer | yes | Index into `prefabs[]` |
| `x` | number | yes | World meters |
| `y` | number | yes | World meters |
| `z` | number | no | ASL m; omit → T-090.4 |
| `rotationDeg` | number | no | Yaw default; pitch/roll only if ≠ prefab default |
| `tags` | string[] | no | **Instance-only** overrides (e.g. `bridge`, `partial_burial_ok`) |
| `geometry` | object | no | Only when placement-specific (non-road); usually omit |

**Do not** duplicate `kind`, `class`, `resourceName`, or `bounds` on instances — resolve via `prefabs[prefabId]`.

**Compact wire option (T-110 input):** arrays `[id, prefabId, x, y, z?, rotationDeg?]` documented in schema as `oneOf` — golden uses expanded objects for readability.

---

### Road segments (geometry-heavy — separate array)

Roads share **roadClass** but each segment has **unique polyline**. Do not force into prefab/instance dedup unless segment shares exact same point list.

| Field | Type | Required |
|-------|------|----------|
| `id` | string | yes |
| `roadClass` | enum | yes |
| `points` | `[[x,y],…]` or `[[x,y,z],…]` | yes |
| `prefabId` | integer | no | If segment uses a standard road prefab template |
| `tags` | string[] | no |

Road count ~5k–20k on Everon — small enough for one `roads.json.gz`.

---

### Spatial chunking (runtime + export)

When `instances.length` > **50k** (or full gzip > **30 MB**):

```text
objects/chunks/manifest.json     # { chunkSizeM: 512, cells: [{ cx, cy, path, instanceCount }] }
objects/chunks/12_8.json.gz      # { instances: [...] } — prefabs loaded from shared prefabs.json.gz
```

- Chunk key `(cx, cy) = floor(x / chunkSizeM), floor(y / chunkSizeM)`.
- Each chunk file contains **instances only**; **prefabs are global** (loaded once, ~1–5 MB gzip).
- Editor (**T-090.5**) fetches chunks intersecting viewport + 1-cell margin.
- Audit (**T-090.4/.6**) streams chunks sequentially — never holds 1M instances in RAM.

---

### Size estimates (Everon order-of-magnitude)

| Format | ~1M trees/props | Notes |
|--------|-----------------|-------|
| **Naive denormalized JSON** | 200–500 MB raw | ❌ Forbidden for production |
| **catalog v1 deduped gzip** | 15–40 MB instances + 1–3 MB prefabs | ✅ T-090 target |
| **Chunked + gzip** | Same total; 512 KB–2 MB per chunk fetch | ✅ Editor load |
| **T-110 binary** | ~24–40 MB `ArrayBuffer` | Future compile from catalog v1 |

Per instance compact JSON ≈ `{ id, prefabId, x, y, z?, r? }` → **~40–80 bytes** serialized vs **~300+ bytes** denormalized.

---

### Resolve helper (normative for consumers)

```typescript
function resolveWorldObject(
  prefabs: Prefab[],
  inst: Instance,
  audit?: AuditIndex
): ResolvedWorldObject {
  const p = prefabs[inst.prefabId];
  if (!p) throw new Error(`Unknown prefabId ${inst.prefabId}`);
  return {
    id: inst.id,
    prefabId: inst.prefabId,
    resourceName: p.resourceName,
    kind: p.kind,
    class: p.class,
    label: p.label ?? p.ai.summary,
    taxonomyPath: p.ai.taxonomyPath,
    summary: p.ai.summary,
    x: inst.x,
    y: inst.y,
    z: inst.z ?? null,
    rotationDeg: inst.rotationDeg ?? 0,
    spatial: p.spatial,
    gameplay: p.gameplay,
    tags: [...(p.tags ?? []), ...(inst.tags ?? [])],
    placement: audit?.get(inst.id),
  };
}
```

Normative field list: [`t090_eden_ai_world_object_schema.md`](t090_eden_ai_world_object_schema.md). Editor API: **T-090.7**.

---

## Field contract (legacy flat row — deprecated)

The table below describes the **resolved** view after joining prefab + instance. **Do not** emit one fat JSON object per map object in export files.

| Field | Source |
|-------|--------|
| `id`, `x`, `y`, `z`, `rotationDeg`, instance `tags` | **instance** |
| `kind`, `class`, `resourceName`, `label`, `bounds`, `render` | **prefab** |
| `geometry` (roads) | **roadSegments** or instance override |

### `bounds` (T-090.6 geometry audit)

Export from Workbench prefab bbox when available. Used for automated buried/floating detection @ 1M scale — **not** a render mesh.

| Field | Type | Notes |
|-------|------|-------|
| `model` | `"aabb"` \| `"obb"` | Prefer `obb` |
| `pivot` | `"base"` \| `"center"` | Where `x,y,z` attaches |
| `halfExtentsM` | `{x,y,z}` | Half-size in local space (m) |
| `rotationDeg` | `{yaw,pitch?,roll?}` | Defaults `{yaw: rotationDeg}` on instance |

If absent, T-090.6 uses **kind defaults** (see `t090_6_geometry_placement_audit.md`).

---

## Z placement problem (document now, fix later)

**Known issue:** Exported objects may sit **below terrain** (buried) or **float** above ground. Causes: export-time sampling vs DEM grid, building pivots at corner vs center, bridge decks, trees on slopes.

| Flag (T-090.4 output) | Meaning |
|-----------------------|---------|
| `zDeltaM = objectZ - demZ` | Positive = floating; negative = buried |
| `severity: ok \| warn \| fail` | `fail` if \|zDeltaM\| > threshold (default **2 m** trees, **0.5 m** roads, **1 m** buildings) |
| **T-090.6** | OBB corner samples → `visibleAboveGroundPct`, `maxBurialM` — catches tilted/large props point audit misses |

**T-090.2** defines `z` optional + `bounds` optional. **T-090.4** = fast pivot screen (all objects). **T-090.6** = geometry visibility (1M automated).

---

## Cost model (AI planning)

| Asset | Everon order-of-magnitude | Git LFS | Runtime (editor) |
|-------|---------------------------|---------|------------------|
| Tile pyramid (tile zoom 0–5) WebP | per N10 tile-cache table (basemap dual-view) | yes | GPU textures; lazy load |
| Full object catalog (catalog v1) | 15–40 MB instances + 1–3 MB prefabs gzip | optional LFS | **Stream/chunk** — never full parse in browser |
| Sample fixture (T-090.2) | <1 MB | no | Vitest + schema validate |
| Render @ 100k instances | — | — | Requires LOD/cluster (**T-090.5**); naive IconLayer **fails** |

**Rule:** Full-world catalog is a **build artifact**, not a single `JSON.parse` in the browser.

---

## Deliverables (T-090.2 ship — post bootstrap)

| # | Artifact | Path | T-090.2 action |
|---|----------|------|----------------|
| 1–4 | Schemas | `packages/tbd-schema/schema/map-object-*.schema.json` | **Verify only** — extend enums only if S9 requires |
| 5 | Golden fixtures | `packages/tbd-schema/golden/map-objects/*` | **Expand** — full S9 enum coverage |
| 6 | Semantic verifier | `packages/tbd-schema/scripts/verify-map-object-golden.mjs` | **Create** — S2–S9 gates |
| 7 | Classify rules | `packages/tbd-schema/rules/prefab-classify.json` | **Extend** — new class patterns |
| 8 | Census script | `scripts/map-assets/census-types.mjs` | **Stub only** — full compute waits for T-090.3 |
| 9 | Verify log | `.ai/artifacts/t090_2_verify_log.md` | **Fill on ship** |
| 10 | Manifest stub (optional) | `packages/map-assets/everon/manifest.json` `objects` block | Schema-valid paths to goldens |

---

## S9 gap audit (@ bootstrap — T-090.2 must close)

| Surface | Missing coverage |
|---------|------------------|
| `map-object-prefabs-sample.json` | tree `dead`/`unknown`; vegetation `grass`/`fern`/`dead`/`unknown`; rock `cliff`/`pebble`/`scree`/`unknown`; prop `barrier`/`sign`/`furniture`/`debris`/`pebble`/`unknown`; utility `lamp`/`antenna`/`pipeline`/`unknown`; water `dock`/`buoy`/`unknown`; road prefab all classes except `road_paved` |
| `map-object-roads-sample.json` | `road_paved`, `path`, `runway`, `unknown` |
| `map-object-regions-everon-sample.json` | `waterBody` region kind |
| `buildingClass` | **Complete** (20/20) @ T-090.3.3 |

---

## Verification

```bash
make schema-validate
make map-object-enums-verify
make map-census TERRAIN=everon
cd packages/tbd-schema && npm run verify-map-object-golden   # after T-090.2 lands script
```

| ID | Check | Pass |
|----|-------|------|
| S1 | Schema validates golden sample | exit 0 |
| S2 | Every row has `kind` + `class` | script |
| S3 | ≥1 example per `kind` in golden | manual |
| S4 | Road rows use `roadClass` enum | script |
| S5 | Golden uses **prefab dedup** — no duplicated `resourceName` on instances | script |
| S6 | `prefabId` on every instance resolves | script |
| S7 | Every prefab has `ai.summary`, `ai.taxonomyPath`, `gameplay.cover.type`, `spatial.heightM` | script |
| S8 | Materialized resolved samples validate `map-object-resolved.schema.json` | script |
| S9 | Golden includes **every `buildingClass`** enum + every `kind` | script |
| S10 | `class` values ⊆ `map-object-enums.schema.json` | script |

---

## Locked decisions

| Decision | Rationale |
|----------|-----------|
| Do not re-scaffold T-090.0.2 schemas | Bootstrap shipped; T-090.2 = goldens + semantic gates |
| `verify-map-object-golden.mjs` owns S2–S9 | S1/S10 stay in `validate.mjs` + `verify-map-object-enums.mjs` |
| Golden paths stay under `golden/map-objects/` | No monolithic `map-objects-everon-sample.json` rename |
| Census stays `pending_export` until T-090.3 | `census-types.mjs` validate-only path must keep exit 0 |
| No satellite / ortho edits | Parallel stream A (`.2.5.1`) owns SAP water |
| No render / FE code | T-090.5 |

---

## Out of scope

- Workbench export (**T-090.3**)
- Z audit script (**T-090.4**)
- Deck.gl layers (**T-090.5**)
- Building floor picker (**T-129**)
- Docs/registry edits during Claude Code run (Cursor sync after merge)

---

## Ship

Tag **`T-090.2`** @ `691d9b26` · merged to `main`  
Handoff: [`.ai/artifacts/t090_2_claude_code_handoff.md`](../../../.ai/artifacts/t090_2_claude_code_handoff.md)  
Verify log: [`.ai/artifacts/t090_2_verify_log.md`](../../../.ai/artifacts/t090_2_verify_log.md) — S1–S10 PASS, 52 prefabs, `verify-map-object-golden.mjs` in `make schema-validate`

**Unblocks:** **T-090.3** phased Workbench export (classify rules + schema gates live).

---

## Claude Code prompt — T-090.2 (copy-paste)

Extract: `./scripts/ticket prompt T-090 --slice T-090.2` (run from worktree)

```
Read CLAUDE.md first.

Implement **T-090.2** — map object taxonomy ship (S1–S10 goldens + semantic verifier).

═══ PREFLIGHT ═══
  cd .ai/artifacts/worktrees/TBD-T-090-2
  git fetch origin && git rebase main
  make schema-validate
  ./scripts/ticket brief T-090

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t090_2_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t090_2_map_object_taxonomy.md
  3. packages/tbd-schema/golden/map-objects/* (current partial S9)
  4. packages/tbd-schema/scripts/validate.mjs + verify-map-object-enums.mjs

═══ PROBLEM ═══
  T-090.0.2 shipped schemas + partial goldens. Agents and T-090.3 export need closed enum
  coverage (S9) and automated semantic gates (S2–S8) — not just AJV. Building classes are
  complete; tree/vegetation/rock/prop/utility/water/road prefabs and waterBody regions are not.

═══ SHIPPED (do not reopen) ═══
  - T-090.0.2 — map-object schemas + validate.mjs wiring + partial goldens
  - T-090.0 — program hub + verify script scaffolding

═══ LOCKED ═══
  - Expand goldens under golden/map-objects/* — no schema rewrite unless S1 fails
  - Create verify-map-object-golden.mjs for S2–S9; wire npm script + schema-validate
  - prefab-classify.json — extend rules; do not delete bootstrap rules
  - census-types.mjs — validate-only; censusStatus stays pending_export
  - No packages/map-assets/everon/satellite/** or ortho scripts
  - No docs/registry edits

═══ DO ═══
  1. P0 — gap audit vs spec §S9 gap audit; confirm baseline make schema-validate PASS
  2. Expand map-object-prefabs-sample.json for all missing class enum examples
  3. Expand roads + regions (waterBody) + instances + resolved + catalog goldens as needed
  4. Implement packages/tbd-schema/scripts/verify-map-object-golden.mjs (S2–S9)
  5. npm run verify-map-object-golden + fold into make schema-validate
  6. Extend prefab-classify.json for obvious new patterns
  7. Optional: everon manifest objects stub (schema-valid)
  8. .ai/artifacts/t090_2_verify_log.md — S1–S10 PASS table
  9. Tag **T-090.2** · prefix **T-090.2:**

═══ DO NOT ═══
  - Edit docs/**, `.ai/tickets/registry.json`, CLAUDE status markers
  - Touch satellite ortho / water composite (T-090.1.2.5.1 on main)
  - Workbench export or full type-inventory compute (T-090.3)
  - Deck.gl / MC render (T-090.5)

═══ VERIFY (all exit 0) ═══
  make schema-validate
  make map-object-enums-verify
  make map-census TERRAIN=everon
  cd packages/tbd-schema && npm run verify-map-object-golden

═══ MANUAL ═══
  S9-full: verify-map-object-golden reports zero missing enum examples
  S7-spot: one building + one tree prefab — full gameplay + ai blocks

═══ RETURN ═══
  - Commit SHA + tag T-090.2
  - t090_2_verify_log.md with automated output
  - **Ready for Cursor doc sync.**
```

---

## Related

- [`t090_world_object_type_inventory.md`](t090_world_object_type_inventory.md) — **type counts + buildingClass census**
- [`t090_world_object_glyphs.md`](t090_world_object_glyphs.md) — **SVG / atlas, rotatable + scalable**
- [`t090_phased_object_import.md`](t090_phased_object_import.md) — P1→P10 import + mathematical verify
- [`t090_eden_ai_world_object_schema.md`](t090_eden_ai_world_object_schema.md) — **Eden AI exact field contract**
- [`t090_eden_map_reference.md`](t090_eden_map_reference.md)
- [`t090_3_map_asset_export.md`](t090_3_map_asset_export.md)
- [`t090_terrain_export_pipeline.md`](t090_terrain_export_pipeline.md) — one-command multi-map export + AI ops log
