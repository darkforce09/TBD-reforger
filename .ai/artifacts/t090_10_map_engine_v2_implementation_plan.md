# T-090.10.1 — Map Engine v2: implementation plan

**Slice:** T-090.10.1 (plan only — no code) · **Authority:** [`t090_10_map_engine_v2.md`](../../docs/specs/Mission_Creator_Architecture/t090_10_map_engine_v2.md) · [`t090_legacy_raster_pipeline.md`](../../docs/specs/Mission_Creator_Architecture/t090_legacy_raster_pipeline.md) · T-144.1 report [`t144_arma3_map_architecture_report.md`](t144_arma3_map_architecture_report.md) @ `b1949182`
**Consumers:** T-090.3 (export) → T-090.5 (render) → T-090.8 (forest) → T-090.9 (interaction); Cursor locks §5 into `t090_render_lod_contract.md` **v2** on doc sync.
**Reference policy:** Arma 3 = engine ground truth (T-144.1). **Reforger = script/UX reference only — no Enfusion C++ source exists in scope and none may be introduced**; permissible references are our own mod scripts (`apps/mod/tbd-framework`), Workbench MCP behavior, and shipped-game UX observation.

---

## 1. Executive summary

T-144.1 proved Arma 3's editor map is **structured world data drawn live as vectors over one terrain texture** — no readability tile pyramid, no offline compose, density-gate LOD, no world-object clustering. Map Engine v2 rebuilds our basemap on that structure within our real constraint (no engine at runtime): the **frozen `tbd-sat` unified texture stays as the photo field** (A3 `DrawField` analogue, T-090.1.2.8), and everything readable — sea band, contours, land-cover, roads, buildings, forest mass, tree/prop glyphs — becomes **data exported once from Workbench (T-090.3) and rendered as Deck.gl vector/instance layers (T-090.5/.8/.9)** with A3-style per-class density gates and per-type importance. The raster-compose pipeline (dual pyramid, land-cover tints, road bakes) is cancelled/frozen per `t090_legacy_raster_pipeline.md`; Satellite|Map becomes **one style control driving sat opacity + vector emphasis**, not two pipelines. **Done =** Mission Creator boots to sat field + hillshade + grid unchanged; enabling Map Engine v2 layers shows roads/buildings/forests/sea/contours streamed from `objects/*` chunks at ≥55 fps under the N11 budget envelope, `tiles/map/` unmounted, and every layer covered by schema + vitest + manual-zoom gates. **Presentation bar (operator): beat A3/Reforger readability** — authored SVG glyph set per `{kind}-{class}`, alpha/width ladders per zoom, importance declutter, per-class layer toggles, a11y shape+label (never color-only).

---

## 2. A3 → Deck mapping table

Every `CStaticMap::DrawBackground` layer (T-144.1 §3) → v2 implementation. Normative order = `t090_10_map_engine_v2.md` §Layer stack (12 slots). Insertion point for all new layers: the single ordered array in `TacticalMap.tsx:350-358`, after `...basemapLayers` / hillshade, before grid + slot layers.

| # | A3 layer (uiMap.cpp) | v2 slot | Deck layer | Data source | Repo path | New/reuse |
|---|---|---|---|---|---|---|
| 1 | Paper texture (`:1299`) | 1b paper tint | none v1 — flat clear-color/tint under sat | style constant | `worldmap/styleModes.ts` | new (trivial) |
| 2 | `DrawField` sat quads + mip + α crossfade (`:2005–2175`) | 1 satellite-field | existing unified `BitmapLayer` + **new `opacity` prop** | `everon-sat.tbd-sat` (LFS, 152.7 MB) | `layers/useTerrainBasemapLayer.ts:324-333` (+ `satelliteUnified.ts` unchanged) | reuse + prop |
| 3 | `DrawSea` ±5 m gradient (`:2179`) | 2 sea-band | `PolygonLayer` (ocean poly + shore gradient band) computed in worker from DEM | T-091 DEM (`dem/everon-dem-16bit.png`, 6400²) | `worldmap/seaBand.ts` (pure) + worker | new |
| 4 | `DrawScale` hypsometric (`:2253`) | 3 hillshade | existing `dem-hillshade` BitmapLayer + slider | DEM | `layers/useDemLayer.ts` | reuse as-is |
| 5 | `DrawCountlines` live iso (`:3131`) | 5 contours | `PathLayer`, interval = f(zoom), worker-contoured + cached per interval | DEM | `worldmap/contours.ts` (pure marching squares over DEM) + worker | new |
| 6 | Forest/rock marching squares (`:2390–2738`) | 8 forest-mass (+ rock-mass) | `PolygonLayer` fill + outline `PathLayer` | **`objects/density/{cx}_{cy}.bin`** + `objects/forest-regions.json.gz` | `worldmap/forestMass.ts` + worker (T-090.8) | new |
| 7 | Land-cover context (A3: fields texture itself) | 4 land-cover-regions | `PolygonLayer`, coarse field/waterBody/urban regions | `objects/forest-regions.json.gz` region kinds (N5: forest/field/waterBody) | `worldmap/landCoverRegions.ts` | new |
| 8 | `DrawRoads` class quads (`:2929`) | 6 roads | `PathLayer` + `PathStyleExtension` dashing, width/color by `roadClass` | `objects/roads.json.gz` | `worldmap/roadLayer.ts` | new |
| 9 | `DrawObjects` footprints (`:2739`, `DrawBuilding`) | 7 buildings | `PolygonLayer` OBB rects from `spatial.halfExtentsM` + `rotationDeg` (N6; footprint rings when export proves them) + badge `IconLayer` | `objects/chunks/{cx}_{cy}.json.gz` | `worldmap/buildingLayer.ts` | new |
| 10 | `DrawObjects` icons, per-type `importance ≥ scale` (`mapTypes.hpp:90`) | 9 trees/props | `IconLayer` on glyph atlas, `getAngle` from `rotationDeg`, size `baseSizePx·2^(z−REF_ZOOM)` | chunks + `glyphs/atlas/world-glyphs.webp` | `worldmap/treePropLayer.ts` + `layers/worldGlyphAtlas.ts` | new |
| 11 | Airports/runways (`:1617`) | 6 roads (`runway` class) | same road `PathLayer`, class `runway` | `roads.json.gz` | `worldmap/roadLayer.ts` | new |
| 12 | Names/mounts declutter (`:1663–1808`) | (T-090.9, later) | `TextLayer` + importance-distance precompute | export `locations` (future) | deferred | later |
| 13 | `DrawGrid` zoom-stepped (`:3167`) | 10 grid | existing `useBaseMapLayer` LineLayer (zoom-stepped labels = backlog) | procedural | `layers/useBaseMapLayer.ts` | reuse |
| 14 | `DrawExt` mission overlay | 11 slots + 12 selection | existing slot Icon/cluster/selection layers | ydoc | existing | reuse untouched |

FE reality anchors (agent-verified): view = `OrthographicView` zoom −6…+6, default −2, world units = meters (`view/useOrthographicView.ts:12-33`); basemap view state = `state/basemapView.ts` localStorage singleton (`tbd-mc-basemap-view`), not `useMapStore`; hillshade/grid persist in ydoc `meta.environment` (`state/schema.ts:13-20`); no world-object consumer exists anywhere yet (confirmed).

---

## 3. Data contract (T-090.3 rescope)

### 3.1 Current reality (delta base)

- `TBD_TerrainWorldExportPlugin.c` (274 L) = **subregion spike only**: densest 512 m building cell → `TBD_WorldExport_subregion.jsonl` (one entity/line: `resourceName`, `className`, transform, world-AABB) + `_meta.json`. Full-world iterate does not exist.
- Manifest `objects` block **already reserves** `chunksPath`, `chunkSizeM`, `roadsPath`, `regionsPath`, `importPhaseMax`, `importPhaseShipped[]` (`terrain-manifest.schema.json:158-180`) — unused. Everon manifest today: `objects: { format: catalog-v1, typeInventoryPath }` only; census `pending_export`, all counts null.
- `make map-export / map-export-all / map-export-validate / map-verify-phase / map-glyphs-build / map-render-verify` = **exit-1 stubs** (root `Makefile:121-136`).
- Classification: `packages/tbd-schema/rules/prefab-classify.json` — 16 append-only rules (`match.resourceNameContains[]` → kind/class/ai/spatial OBB/render.iconKey).

### 3.2 Export artifacts (v2 normative set)

| Artifact | Format | Content | Status |
|---|---|---|---|
| `objects/prefabs.json.gz` | catalog-v1 | deduped prefab rows: `ai`, `spatial` (OBB), `gameplay`, `render` — **schema bump adds `render.importanceZoom`** (see 3.4) | spec'd (t090_2 shipped schema + bump) |
| `objects/chunks/{cx}_{cy}.json.gz` | catalog-v1 instances | `chunkSizeM: 512`; `(cx,cy) = floor(x/512), floor(y/512)`; compact instance rows `[prefabId, x, y, z, rotationDeg]` | spec'd, unimplemented |
| `objects/roads.json.gz` | map-object-roads schema | polylines + `roadClass` (highway_paved/road_paved/road_dirt/track/path/runway) — **single file, not chunked** (Everon-scale road graph is small) | spec'd |
| **`objects/density/{cx}_{cy}.bin`** | **new binary** (3.3) | per-chunk tree/rock corner-density grids — A3 `MapObjectForest` analogue | **new in v2** |
| `objects/forest-regions.json.gz` | map-object-region schema | derived polygons (t090_8 path B: 32 m bins → components → hull) + region kinds field/waterBody | spec'd (F1–F6) |
| `objects/type-inventory.json` | type-inventory schema | exact integers per kind/class (census I1–I8 *(inventory)*) — **bump adds per-class `importanceZoom` defaults** | shipped stub → real |
| `objects/z-audit.json`, `z-audit-geometry.json` | audit schemas | T-090.4/.6 inputs | spec'd |
| `manifest.json` `objects.*` | terrain-manifest bump | fills reserved fields + **new `densityPath`, `densityCellM: 32`, `lod` block** (gate constants snapshot for cache-busting) | bump |
| ~~`tiles/map/` pass A2~~ | — | **DROPPED — no dual-pyramid pass on any new export** (t090_3 §A2 already marked cancelled; pipeline steps 2b/2c die) | cancelled |

Satellite is **never re-exported** by this pipeline — `everon-sat.tbd-sat` frozen.

### 3.3 Density grid binary (`objects/density/{cx}_{cy}.bin`)

A3 stores per-square corner densities (`MapObjectForest {_tl,_tr,_bl,_br}`, `mapObject.hpp:61-83`) and contours them at draw time (`DrawForestsNew`, marching squares, `uiMap.cpp:2390`). v2 protocol:

- Cell size **32 m** (aligns t090_8 path B binning; 512 m chunk = 16×16 cells → **17×17 corner grid**).
- Layout: little-endian, header `u32 magic 'TBDD'`, `u16 version=1`, `u16 cellM=32`, `u16 cols=17`, `u16 rows=17`, `u8 channelCount=2` (`tree`, `rock`), pad; then per channel `u16[17·17]` counts (trees whose base falls in the 32 m cell around each corner, A3-style overlap acceptable).
- Produced host-side by the classify/post-process step (not the Workbench plugin) from raw instance stream — keeps plugin dumb.
- Consumers: T-090.8 marching squares (iso threshold default `1.0` trees/cell, configurable) for forest/rock mass; **replaces `build-landcover-mask.mjs`** as land-cover authority.
- Size: 17·17·2·2 B ≈ 1.2 KB/chunk raw (~625 chunks Everon ≈ 0.7 MB total) — commit plain (no LFS needed).

### 3.4 Schema bumps (versioned, additive)

1. `map-object-prefab` / catalog-v1: `render` gains **`importanceZoom?: number`** — minimum deckZoom at which instances of this prefab draw, overriding its class gate (A3 per-type `importance ≥ _scaleX`, `mapTypes.hpp:90-96`). Default: class gate (§5). High-importance landmark classes (lighthouse, transmitter, watertower, military) get negative offsets in `prefab-classify.json` rules.
2. `map-object-type-inventory`: per-class row gains **`importanceZoom` default** so the LOD table is data-driven from census (t090_10 LOD rule 2). *(Note: shipped T-090.2 schemas have neither field — confirmed gap; additive bump keeps goldens valid.)*
3. `terrain-manifest`: `objects` += `densityPath`, `densityCellM`, `lod { schemaVersion, refZoom, gates: {class: minZoom} }`.
4. `mapType` naming: our L1/L2 (`kind`/`class`) **is** the A3 `mapType` analogue — no new field; `iconKey = {kind}-{class}` stays the glyph key (glyphs spec). The plan uses "mapType" to mean `kind-class` pair; no schema rename.

### 3.5 Plugin + pipeline delta (`TBD_TerrainWorldExportPlugin.c`)

| Step | Current (spike) | v2 target |
|---|---|---|
| Iterate | densest 512 m cell only | full-world entity query, streamed in 512 m chunk passes (bounded memory), JSONL per chunk to `$profile:` |
| Fields | resourceName/className/transform/AABB | + entity yaw → `rotationDeg` (L2 handedness measured by T-090.3.0 S6/K6 rule), water-body hint if cheap |
| Post-process (host, `scripts/map-assets/export-terrain.sh` + Node) | none | classify (rules) → prefabs dedupe → chunks gz → roads extraction → density accumulation (3.3) → forest regions (t090_8 B) → census → z-audits → manifest patch → ops log `.ai/artifacts/map_export_everon.json` |
| Map raster passes 2b/2c | in pipeline spec | **deleted** |
| Precedent | — | `raw-u16-to-dem-png.mjs` host-side post-process pattern (DEM pipeline) |

LFS/commit policy (extends t090_3 table): chunks gz **LFS if aggregate >10 MB** (expected: yes at P2+), `prefabs.json.gz` LFS if >1 MB, `roads.json.gz` plain (expected <2 MB), `density/*.bin` plain (~0.7 MB total), regions gz plain unless >5 MB. `tiles/` stays gitignored (never was committed — agent-verified).

---

## 4. Render spine (frontend)

### 4.1 Module layout (new `worldmap/` under `features/tactical-map/`)

```text
features/tactical-map/
  worldmap/                      # NEW — Map Engine v2 (all decision logic = pure, node-testable)
    styleModes.ts                # mapStyle → {satOpacity, paperTint, vectorEmphasis} (pure)
    lodGates.ts                  # §5 table as data + classVisible(class, zoom), instanceBudgetCheck (pure)
    chunkMath.ts                 # bbox→chunk ids, border ring, oversized radius (pure)
    chunkStore.ts                # main-thread cache/LRU/refcount of hydrated chunk payloads
    seaBand.ts                   # DEM → ocean/shore polygons (pure; runs in worker)
    contours.ts                  # DEM → iso polylines per interval (pure; runs in worker)
    forestMass.ts                # density grids → marching-squares polys (pure; worker) [T-090.8]
    roadLayer.ts / buildingLayer.ts / treePropLayer.ts / landCoverRegions.ts / seaBandLayer.ts / contourLayer.ts
                                 # thin Deck layer builders over typed arrays
    useWorldMapLayers.ts         # hook: settings + viewport → ordered Deck layer array (slots 2,4–9)
  workers/
    worldObjects.worker.ts       # Comlink API (t090_world_objects_worker.md, W-gates v2)
    worldObjectsClient.ts        # lazy Worker + Comlink.wrap (pattern: mission-creator/compiler/compilerClient.ts:14-31)
  state/
    worldSpatialIndex.ts         # separate rbush (GAP-H3) — worker-side authoritative copy
    worldLayerPrefs.ts           # per-class toggles + mapStyle (localStorage tbd-mc-world-layers / tbd-mc-basemap-view)
  layers/
    useTerrainBasemapLayer.ts    # REFACTOR: + opacity prop on unified BitmapLayer (:324-333); map-view branches retired in T-090.10.2
    worldGlyphAtlas.ts           # NEW — atlas load-once (glyphs spec deliverable 4)
```

**Testability rule (hard):** vitest = node env, no DOM/GPU (`vitest.config.ts:13-14`). Every gate/geometry/style decision lives in the pure modules; Deck layer builders stay thin and untested beyond construction smoke.

### 4.2 Layer order + ids (normative)

Slots 1–12 per scaffold; ids: `satellite-field` (rename of `basemap-satellite-unified` when opacity lands — keep old id until T-090.10.2 to avoid test churn), `world-sea`, `dem-hillshade` (existing), `world-landcover`, `world-contours`, `world-roads`, `world-buildings` + `world-building-badges`, `world-forest` + `world-forest-outline`, `world-trees`, `world-props`, then existing `grid`/slot/selection ids. One Deck layer per **class group**, never per chunk (data arrays composited in the store — draw-call budget).

### 4.3 Style control (replaces Satellite|Map radio)

- `mapStyle: 'satellite' | 'hybrid' | 'map'` in `worldLayerPrefs.ts`; persisted value migration: existing `tbd-mc-basemap-view` `'satellite'→'satellite'`, `'map'→'map'`.
- Mapping (pure, `styleModes.ts`): satellite → sat opacity **1.0**, no paper, vector overlays per toggles; hybrid → sat **0.55**, vectors full; map → sat **0** (+ paper tint), full cartographic emphasis (A3 `_showScale` OFF analogue).
- **No automatic zoom crossfade v1** (differs from A3 `alphaFadeStart/EndScale` — deliberate, §10 Q4); the machinery (opacity as f(zoom) hook input) exists for a later flag.
- Mission Settings dialog: radio (`MissionSettingsDialog.tsx:113-118`) → 3-way segmented control; hillshade slider + grid toggle unchanged (ydoc `meta.environment`).
- **Interim Map mode:** until T-090.5.4 + T-090.8 reach visual parity, `mapStyle:'map'` keeps rendering the **legacy `tiles/map/` pyramid fallback** (existing branch untouched); parity flip + branch deletion = T-090.10.2 (§8).

### 4.4 Presentation bar (operator addition — beat A3/Reforger)

Target: clearer than A3 2012 and Reforger's map at every zoom. (Reforger reference = shipped-game UX observation + our mod scripts only — no Enfusion C++ source.)

1. **Glyphs:** authored SVG set per `{kind}-{class}` (~40–80 files, `packages/map-assets/glyphs/svg/`), Aegis-styled, `tintable` + `defaultColor`, bottom-center anchor for ground props, north-up + `getAngle` rotation (glyphs spec N4/L2; baseSizePx table adopted). Atlas via `make map-glyphs-build` (`build-glyph-atlas.mjs`, ≤4096² power-of-two — budget L1) + `map-glyphs-verify` (GL-G1–G6).
2. **Declutter:** per-class alpha/width ladders (§5), min-px clamps (road widths ≥1 px, glyph sizeMin), per-type `importanceZoom`, forest mass replaces individual trees below glyph band (A3 lesson: things *vanish* below their band — no soup), labels later via importance-distance precompute (T-144 `_nearestMoreImportant` pattern).
3. **Layer toggles:** `WorldLayerToggles.tsx` per class (roads/buildings/forest/trees/props/contours/sea), persisted `tbd-mc-world-layers` (N8), Eden-parity but per-class not lump.
4. **A11y:** class = shape + label, never color alone (L4/I6); color ramps from dataviz-safe Aegis tokens.

---

## 5. LOD table — v2 (rewrites `t090_render_lod_contract.md` N2/N3; Cursor locks on doc sync)

Deck orthographic: `mpp = 2^-zoom` (world units = meters). Band −6…+6, default −2 (4 m/px). A3 translation basis: gate class when its A3 `ptsPerSquare` threshold (px per 50 m cell @ 800 px ref) maps to Deck px density — `zoom ≥ log2(pts / cellM)`.

**Constants v2 (replaces N2):**

| Constant | Value | Provenance |
|---|---|---|
| `REF_ZOOM` | **3** | keep v1 (glyph size anchor) |
| ~~`WORLD_CLUSTER_MAX_ZOOM`~~ | **deleted** | t090_10: no world clustering |
| `TREE_GLYPH_MIN_ZOOM` | **0** | v1 transition band start ∧ A3 icon band (~1 m/px) — convergent |
| `FOREST_FILL_MAX_ZOOM` | **+1** | keep v1 (`FOREST_REGION_MAX_ZOOM`) — fill fades as glyphs take over |
| `FOREST_OUTLINE_MIN_ZOOM` | **−1.5** | A3 `ptsPerSquareForEdge≈15` → log2(15/50)≈−1.7, rounded |
| `BUILDING_FOOTPRINT_MIN_ZOOM` | **−2.5** | A3 `ptsPerSquareObj≈9` → log2(9/50)≈−2.5 (visible at default −2 ✓) |
| `BUILDING_BADGE_MIN_ZOOM` | **+1** | keep v1 |
| `PROP_MIN_ZOOM` | **+3** | keep v1 |
| `VEGETATION_MIN_ZOOM` | **+1.5** | between trees and props |
| `ROCK_LARGE_MIN_ZOOM` | **+1** | landmark value |
| `PICK_RADIUS_PX` | **12** | A3 2 %-viewport analogue, fixed screen feel |
| `INSTANCE_BUDGET` | **150 000** | drawn world instances at any zoom (vitest-assertable from census ints) |

**Master band table v2 (replaces N3):**

| deckZoom | sat (`satellite` style) | sea/land-cover | contour interval | roads visible | buildings | forest | trees/veg/props |
|---|---|---|---|---|---|---|---|
| −6…−4 | 1.0 | fill on | 100 m | highway, paved, runway | — | fill α 0.45 | — |
| −4…−2.5 | 1.0 | fill on | 50 m | + (dirt/track @ −2) | — | fill α 0.45 | — |
| −2.5…0 | 1.0 | fill on | 50→20 m | all but path | OBB rects (thin) | fill α 0.35 + outline @ ≥−1.5 | — |
| 0…+1 | 1.0 | fill on | 20 m | all but path | rects + fill | fill α 0.35, outline | tree glyphs (from 0) |
| +1…+3 | 1.0 | fade fill | 10 m | all but path | + badges @ ≥+1 | fill α 0.12 → outline-only | + vegetation @ ≥+1.5, large rocks @ ≥+1 |
| +3…+6 | 1.0 | off | 10 m | + path @ ≥+4 | + footprint rings (if exported) | outline only | + props/small rocks @ ≥+3; tree size cap 1.5× by `heightM` |

- **Road class table: keep v1 verbatim** (already vector-correct): highway_paved `#c8c8c8`/4 px @ z0/solid/min −6 · road_paved `#a0a0a0`/2.5/solid/−6 · road_dirt `#8b6914`/2/dash/−2 · track `#6b5010`/1.5/dash/−2 · path `#5a4a3a`/1/dash/**+4** · runway `#ffffff`/6/solid/−6; dashing via `PathStyleExtension`.
- Per-type override: `render.importanceZoom` (prefab) < class gate ⇒ landmark visible earlier (lighthouse/transmitter/watertower/military: recommend −4).
- **Gate implementation:** `lodGates.ts` pure table + `classVisible(class, zoom)`; **budget gate** = vitest: for each band boundary, Σ(census instances of visible classes) ≤ `INSTANCE_BUDGET` — data-driven from `type-inventory.json` exact ints once P-phases land (I8-inventory: no hard-coded ~900k).
- **Cluster confirmation:** world layer has **no supercluster** anywhere (delete `worldClusterIndex.ts` + `clusterTrees()` from worker spec; rewrite W4 → `visibleInstances(bbox, zoom)` respects gates; rewrite LOD3 vitest → "at −2: forest = polygons, trees = **hidden**, buildings = rects"). **Slot** cluster untouched: `ZOOM_CLUSTER_MAX = −4`, `CLUSTER_SLOT_THRESHOLD = 500` (`state/constants.ts:12,17`).
- Zoom is continuous (Deck); bands step **feature classes**, exactly A3's model (continuous transform, stepped classes).

---

## 6. Chunk streaming (A3 `landSave` analogue)

A3: per-square spans lazily deserialized under `TCLoadMapObjects` time budget, refcount cache, +5 % border preload, oversized-object rect extension, predictive flyTo preload (T-144 §6). v2:

| Mechanism | v2 design | A3 analogue |
|---|---|---|
| Chunk key | 512 m grid, `chunkMath.ts` `bboxToChunks(viewportBbox)` | land squares |
| Hydrate budget | worker parses freely; **main-thread apply ≤ 4 ms/frame** (rAF slice; queue drains across frames — `yieldToUi` pattern) | `TimeManagerScope(TCLoadMapObjects)` |
| Border preload | viewport bbox + **max(5 % of viewport span, 1 chunk ring)** | `PreloadMapObjects` +5 % (`uiMap.cpp:1528`) |
| Oversized objects | prefab classes flagged `oversizedRadiusM` (runway, pier, powerline span) ⇒ +1 extra ring on affected chunks | `ObjMapRadiusRectangle` (`collisions.cpp:137`) |
| Cache policy | LRU cap = **3× current viewport chunk count, min 64 chunks**, refcount pin on visible set (never evict visible); N11 phase budgets are the memory envelope (P1 40 MB → P10 256 MB) | refcounted `_mapObjects(x,z)` + `MapObjectListRectUsed` lock |
| Predictive | flyTo/`Center(soft)` animations preload target viewport chunks | `PreloadData(predictPos)` (`uiMap.cpp:3443`) |
| Skip-when-invisible | if no class visible at current zoom needs instances (all gates closed), **don't hydrate chunks at all** (density/regions/roads suffice below −2.5) | `ptsLand < min(For,Road,Obj)` skip (`uiMap.cpp:1509-1517`) |

**Worker split** (t090_world_objects_worker.md, W-gates v2): worker owns fetch + gunzip (`DecompressionStream`) + JSON/bin decode + `worldSpatialIndex` (separate rbush — GAP-H3, W3) + density→marching-squares + contour generation; returns **transferable typed arrays** (positions `Float32Array`, prefab indices `Uint16Array`, polygon vertex buffers) — never 1M JS objects (W-transfer rule). Main thread: `chunkStore.ts` composites arrays per class → Deck `data` props ({length, attributes} pattern). Pick: worker `pickNearest/pickRect` (W2 brute-force parity with T-063 semantics); Deck GPU pick stays forbidden (N4); hover via container `pointermove` rAF (T-057 rule). Client harness: copy `compilerClient.ts` lazy-init pattern; `terminateWorldObjects()` on mission unmount.

---

## 7. Phased slices

Gate namespaces (collision-resolved for citation): `PH-P1…P10` = import phases · `PL-P1…P5` = pipeline gates · `INV-I1…I8` = inventory invariants · `IX-I1…I8` = interaction · `GL-G1…G6` = glyph checks · `G1…G12` = global export invariants. (Cursor: consider renaming in specs on doc sync.)

| Slice | Scope | Files touched (primary) | Acceptance gates | Deps |
|---|---|---|---|---|
| **T-090.3.1** — export core (PH-P1 buildings **+ roads pulled forward**) | Plugin full-world iterate + host post-process: classify → prefabs/chunks/roads/census; schema bumps (§3.4); realize `make map-export TERRAIN=everon PHASE=P1` + `map-verify-phase` + `map-export-validate` (stubs → real) | `TBD_TerrainWorldExportPlugin.c`, `scripts/map-assets/export-terrain.sh` + new Node post-process, `packages/tbd-schema/schema/*` bumps + goldens, `rules/prefab-classify.json`, root `Makefile` | E1, E1b(PH-P1-1…6 + G1–G12), E3/E4/E6, `make schema-validate`, `map-census` INV-I1…I8, roads golden validates | T-090.10.1 approved; Workbench operator session |
| **T-090.3.2** — density + trees (PH-P2 + P2b) | density/*.bin (§3.3) + forest-regions (t090_8 B) + PH-P2 tree chunks; census real ints | post-process scripts, `map-object-region` goldens, manifest `objects.density*` | F1/F2/F6 (export side), PH-P2-1…5, density golden + `verify-map-object-golden` ext, INV re-census byte-identical (I6) | 3.1 |
| **T-090.5.1** — render spine scaffold | `worldmap/` skeleton (styleModes/lodGates/chunkMath pure + tests), sat `opacity` prop, `mapStyle` 3-way + prefs migration, worker+client skeleton, feature flag `worldmap.enabled` | `useTerrainBasemapLayer.ts`, `MissionSettingsDialog.tsx`, `worldmap/*`, `workers/*`, `state/worldLayerPrefs.ts`, `TacticalMap.tsx:350-358` | vitest: styleModes mapping, lodGates table, chunkMath, prefs migration (module-singleton test pattern per `basemapView.test.ts`); FE build/lint; manual M1–M3 (sat unchanged @ all styles-off, style switch, no regression w/ flag off) | none (parallel w/ 3.1) |
| **T-090.5.2** — roads + buildings live | roadLayer + buildingLayer + badge glyphs (P1 glyph set: `building-*` per non-empty buildingClass), glyph atlas build | `worldmap/roadLayer.ts`, `buildingLayer.ts`, `layers/worldGlyphAtlas.ts`, `glyphs/svg/*`, `build-glyph-atlas.mjs`, Makefile glyph targets | R1–R4 + R7 (`map-glyphs-verify` GL-G1…G6), LOD vitest (road classes per band, buildings ≥−2.5), manual zoom checklist Z1–Z6, ≥55 fps @ PH-P1 data (R5) | 3.1 + 5.1 |
| **T-090.5.3** — streaming @ scale | full worker (W1–W3, W5 + W4-v2 `visibleInstances`), chunkStore LRU/budget/border/oversized, pick wiring | `workers/worldObjects.worker.ts`, `worldObjectsClient.ts`, `chunkStore.ts`, `state/worldSpatialIndex.ts` | W1–W5(v2); N11 PH-P1/P2 budgets (load ms/MB/fps table); hydrate ≤4 ms/frame instrumented; INSTANCE_BUDGET vitest | 3.2 + 5.2 |
| **T-090.8.1** — forest/rock mass | marching squares from density (worker), fill+outline layers, land-cover regions layer | `worldmap/forestMass.ts`, `landCoverRegions.ts` | F3 (@ −2 polygons, no tree icons), F4/F5, LOD3-v2 vitest, N11 P2b (3000 ms/+20 MB/pinned) | 3.2 + 5.3 |
| **T-090.5.4** — sea-band + contours | DEM→ocean/shore polys + iso contours in worker, cached per interval band | `worldmap/seaBand.ts`, `contours.ts`, layer builders | vitest on pure fns (known DEM fixtures); manual M: shoreline matches water-composite visual; contour interval ladder per §5; perf (contour gen off main thread) | 5.3 (worker) |
| **T-090.5.5** — trees/veg/props glyphs | treePropLayer + PH-P2…P5 glyph sets + `importanceZoom` overrides + heightM size cap | `worldmap/treePropLayer.ts`, `glyphs/svg/*` | R7/R8 (rotation pick), GL-G5 rotation-distinct fixture, INSTANCE_BUDGET @ +1/+3 bands, ≥55 fps @ PH-P2 (N11) | 3.2 + 5.3 + 8.1 |
| **T-090.9.x** — interaction | per existing spec, pick gates aligned to §5 bands (pick only visible classes; `PICK_RADIUS_PX·mpp`) | worker pick + inspect UI | IX-I1…I8 | 5.5 |
| **T-090.10.2** — legacy retirement | delete map-view branches (`useTerrainBasemapLayer.ts:94,220,243-256,306,347`; `BasemapView` type; dialog button), manifest `tiles.map` → deprecated, freeze notes, `basemapView.ts` → shim over worldLayerPrefs | listed + `coords/terrainManifest.ts:43` | FE build/lint + vitest (basemapView tests rewritten); manual: `map` style = pure vector; V6 fallback test retired | visual parity sign-off after 5.4 + 8.1 |

Manual zoom checklist template (per render slice): Z1 whole-island (−6) · Z2 −4 · Z3 default −2 · Z4 0 · Z5 +3 · Z6 +6 — screenshot each, verify band table §5 row-by-row, pan sweep for chunk pop-in, fps overlay on.

Slice IDs are proposals; reconcile against registry at `./scripts/ticket sync` (Cursor).

---

## 8. Legacy migration

| Item | Disposition | When |
|---|---|---|
| `scripts/map-assets/build-map-cartographic.mjs` + `make map-cartographic-everon` | **freeze** (runnable, no features) — legacy Map fallback rebuild only | now (policy, already in legacy doc) |
| `build-landcover-mask.mjs` | freeze → **retired** by density grids | after T-090.3.2 |
| `tiles/map/` | **gitignored local output (never committed — no repo purge needed)**; serves `mapStyle:'map'` fallback | branch deleted @ T-090.10.2 |
| `tiles/satellite/` pyramid | stays as unified-texture **fallback** (`delivery:'pyramid'` path, `computeLod` machinery kept) | indefinite |
| Mission Settings Satellite\|Map radio | → 3-way `mapStyle` select; localStorage value migration (`'map'`→`'map'`) | T-090.5.1 |
| `state/basemapView.ts` | absorbed into `worldLayerPrefs.ts` (shim first, delete @ 10.2) | 5.1 → 10.2 |
| `t090_basemap_dual_view.md` (N8/N9/N10, V1–V7) | **superseded** banner (Cursor): N9 + map half of N10 + V1/V2/V7 die; N8 keys survive re-purposed; V6 retired @ 10.2 | Cursor doc sync |
| `t090_3` legacy §A block + pipeline steps 2b/2c + manifest example w/ `tiles.map` | strip/mark historical (Cursor) | doc sync |
| Shipped T-090.1.1 / T-090.1.1.1 raster artifacts | frozen; no rebuilds; superseded visually by 5.4 + 8.1 | — |
| `verify-t090-spec-consistency.mjs` | needs v2 gate updates (cluster asserts → density-gate asserts) — **code change, assigned to T-090.5.1**, not this slice | 5.1 |
| Cluster machinery in specs (`WORLD_CLUSTER_MAX_ZOOM`, N3 cluster rows, LOD3, `worldClusterIndex.ts`, `clusterTrees`, W4, glyphs-spec cluster note) | delete/rewrite per §5 (Cursor doc sync; code never existed) | doc sync |

---

## 9. Risk register

| # | Risk | Impact | Mitigation |
|---|---|---|---|
| R1 | 1M instances memory (P10) | tab OOM | typed-array SoA end-to-end (worker transferables → Deck `{length, attributes}`); prefabs deduped once (~1–5 MB); N11 envelope 256 MB @ P10 is the gate; LRU eviction + skip-hydrate below gates |
| R2 | Draw calls / layer churn | fps collapse | one layer per class (composite arrays), never per chunk; Deck `updateTriggers` keyed on store revision; `pickable:false` on mass layers (pick in worker) |
| R3 | First paint regression | editor feels slower | sat field + hillshade + grid render exactly as today before any chunk arrives; world layers hydrate progressively behind `worldmap.enabled` flag; boot metric M1 in 5.1 |
| R4 | Main-thread jank on hydrate | drag/pan stutter (T-061 regression) | 4 ms/frame apply budget; all parse/geometry in worker; pan coalescing untouched |
| R5 | Layer failure (bad chunk, atlas 404, DEM decode) | blank map | per-layer feature flags + error boundary: failed layer skips + one toast (pattern: `onBasemapDegraded`); sat field never depends on v2 modules |
| R6 | LFS bandwidth/cost (chunks gz) | clone bloat, CI cost | budget table per phase in export ops log; chunks LFS-tracked only above thresholds (§3.5); CI never pulls map-assets LFS (schema+goldens only — existing `ci-local` unaffected) |
| R7 | Glyph atlas limits | texture too big / blurry | ≤4096², power-of-two packing, GL-G4 bounds gate; per-class not per-prefab (~40–80 sprites) |
| R8 | Contour/marching-squares cost on big DEM (6400²) | worker stalls | generate per interval band lazily + cache; downsample DEM per band (A3 steps contour sampling by `ptsPerSquareCLn`); budget test on fixture |
| R9 | Worker transfer overhead | copy storms | transferables only; chunk payloads moved once; no postMessage of raw JSON arrays |
| R10 | Pick correctness across bands | picking invisible things | pick gate = same `lodGates.ts` table (single source); IX vitest asserts pick-band parity |
| R11 | Everon-only data (Arland has none) | broken empty state | I7/L5 empty-state path: no `objects` in manifest → v2 layers disabled cleanly, toggles hidden |
| R12 | Spec drift during doc sync | gates cite dead constants | §7 namespace prefixes; Cursor updates `verify-t090-spec-consistency` asserts in 5.1 |

---

## 10. Open questions (operator decision; defaults chosen so work proceeds if unanswered)

| # | Question | Default recommendation |
|---|---|---|
| Q1 | Pull **roads** forward out of PH-P6…P9 into the first export slice? (readability win; single small artifact) | **Yes** — roads.json.gz exports in T-090.3.1 alongside PH-P1 buildings; phase-gate renumbering = Cursor doc sync |
| Q2 | Chunk payload format v1 | **JSON.gz** now (schema-validatable, debuggable); columnar `.bin` escape hatch only if N11 fails at PH-P2 |
| Q3 | Density resolution | **32 m cells** (t090_8 locked; 17×17 corners per 512 m chunk) |
| Q4 | Automatic sat↔map zoom crossfade (A3 `alphaFadeStart/EndScale`)? | **Off v1** — explicit 3-way style only; revisit as flag after 5.4 (UX simplicity beats A3 parity here) |
| Q5 | `mapStyle:'map'` before vector parity | **Keep legacy `tiles/map/` pyramid fallback** until 5.4 + 8.1 sign-off, then T-090.10.2 deletes |
| Q6 | Contours: precomputed export vs client-side | **Client-side in worker from DEM** (zero new export artifacts, matches A3 live contouring); precompute only if R8 budget fails |
| Q7 | Paper tint in `map` style | **Flat color token** (no texture asset) |
| Q8 | Gate namespace renames in specs (PH-/PL-/INV-/IX-/GL-) | **Adopt prefixes** at Cursor doc sync; code cites prefixed ids from day one |
| Q9 | `importanceZoom` defaults location | **`prefab-classify.json` rules** (append-only, versioned) + census copy; not hard-coded in FE |
| Q10 | N12 citation in hub ("N1–N12") | **N12 does not exist** in specs (N1–N11 confirmed) — Cursor fixes hub reference |

---

*Prepared by Claude Code (T-090.10.1). Inputs: T-144.1 report + t090_10 scaffold + legacy disposition + spec corpus digest + FE/export tooling inventories (3 read-only exploration passes). No code, no docs/registry edits in this slice.*
