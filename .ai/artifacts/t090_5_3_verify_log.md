# T-090.5.3 — Map Engine v2 worker chunk streaming @ scale · verify log

**Slice:** T-090.5.3 · **Executor:** claude-code · **Date:** 2026-07-05
**Spec:** [`t090_5_map_object_render_layer.md`](../../docs/specs/Mission_Creator_Architecture/t090_5_map_object_render_layer.md) · plan §6/§7 [`t090_10_map_engine_v2_implementation_plan.md`](t090_10_map_engine_v2_implementation_plan.md) · worker spec [`t090_world_objects_worker.md`](../../docs/specs/Mission_Creator_Architecture/t090_world_objects_worker.md) · LOD [`t090_render_lod_contract.md`](../../docs/specs/Mission_Creator_Architecture/t090_render_lod_contract.md)

## What shipped

The T-090.5.2 main-thread bulk loader (fetch-all 275 chunks, distill buildings) is replaced by
viewport-driven streaming: the worker owns fetch + gunzip (`DecompressionStream`) + parse +
the **world rbush**, returning **transferable typed arrays** per chunk; the main-thread
`chunkStore` pins the visible set, applies payloads under a **4 ms/frame** budget, LRU-evicts
beyond **max(64, 3× viewport chunks)**, and feeds the unchanged road/building layer builders.
**No new Deck layers; ids and visuals identical to T-090.5.2.2.** Flag off = zero change.

| File | Change |
|---|---|
| `workers/worldObjectsCore.ts` | **NEW** — whole worker brain as a pure factory (node-testable): manifest/prefab/chunk-index load, chunk parse → SoA typed arrays, world rbush, worker-side LRU, `visibleInstances`/`pickNearest`/`pickRect`/`resolve`, budget cap |
| `workers/worldObjects.worker.ts` | rewritten thin: `Comlink.expose(core)` + HTTP `fetchBytes` + transferable marking |
| `workers/worldObjectsClient.ts` | full typed RPC surface (`loadWorldManifest`, `loadWorldChunksInBbox`, `worldVisibleInstances`, `pickWorldNearest`, `pickWorldRect`, `resolveWorldObject`, `unloadWorldObjects`, `worldPickRadiusM`) + existing lifecycle |
| `state/worldSpatialIndex.ts` | **NEW** — `createWorldSpatialIndex()` factory rbush (chunk-granular insert/remove, world-meter radius, class filter). Deliberately not the slot singleton (W3) |
| `worldmap/chunkStore.ts` | **NEW** — main-thread streaming session: skip-when-invisible, chunkMath preload set (+1 oversized ring), request diff/dedupe, refcount pin, LRU, ≤4 ms/frame apply queue, hydrate stats, `useSyncExternalStore` surface |
| `worldmap/worldData.ts` | **shrunk** to manifest gate + roads one-shot (`loadWorldRoads`); all chunk machinery deleted |
| `worldmap/useWorldMapLayers.ts` | rewired: buildings via chunkStore snapshot, roads via worldData, viewport effect → `setWorldViewport` |
| `TacticalMap.tsx` | passes `viewBounds` (existing basemap bounds) into the hook |
| `MissionCreatorPage.tsx` | unmount: `resetWorldStream()` + `terminateWorldObjects()` (mirror of `terminateCompiler`) |
| `index.ts` (barrel) | exports the two teardown fns |
| `worldmap/buildingLayer.ts` | **one-line lint annotation only** (see §Pre-existing findings) — no logic change |

Tests: `workers/worldObjectsCore.test.ts`, `worldmap/chunkStore.test.ts`,
`state/worldSpatialIndex.test.ts` (all new).

## Gate results

| Gate | Check | Result |
|---|---|---|
| **W1** | Golden chunk (`map-object-chunk-sample.json`, gzipped like production) parses in the worker core → building group counts/positions/rotations/z match; mixed chunks deliver only building+pier rows; tree-only chunks deliver hydrated-empty | **PASS** (vitest) |
| **W2** | `pickNearest` === brute-force scan over all fixture instances (7 probes × 3 radii + zoom-gated variants); unclassified kinds unpickable; radius-miss → null | **PASS** (vitest) |
| **W3** | World rbush = factory instance inside the worker core; slot `slotSpatialIndex` singleton untouched by world inserts | **PASS** (vitest + code review) |
| **W4** | `visibleInstances` respects `lodGates.classVisible` only: @−2 buildings drawn/trees hidden; @−3 nothing; tree band opens @0; veg/rock/prop at their gates; hard cap at `INSTANCE_BUDGET` | **PASS** (vitest + headless browser) |
| **W5 / R5** | Pan ≥55 fps with world layers on | **operator browser pass pending** (see §Manual); all per-frame work measured ≪ budget |
| **INSTANCE_BUDGET** | Census-driven from committed `type-inventory.json`: building 4,131 + water 2,299 = 6,430 ≤ 150,000 at every band boundary where buildings draw; trees (501,861 > budget) provably outside the hydrate set below zoom 0 | **PASS** (vitest, no hard-coded counts) |
| **Hydrate ≤4 ms/frame** | Instrumented (`getWorldStreamStats`: `maxApplyMs`, `framesOverBudget`, over-budget `console.warn`); slow-clock vitest proves the slicer (1 chunk/frame under a 3 ms-per-call clock, queue drains across frames) | **PASS** — real numbers below |
| **LRU** | Cap max(64, 3× viewport), pinned/visible never evicted; revisit-under-cap = zero refetch; far sweep evicts oldest → revisit refetches | **PASS** (vitest, fake client call log) |
| **Skip-when-invisible** | @−3 zero chunk requests; zoom back in re-pins from cache with zero requests | **PASS** (vitest) |
| **Regression bar** | Streamed `BuildingInstance[]` deep-equals `buildingsFromChunkInstances` output on identical rows (same `obbCorners`/`badgeIconKey`/`buildingPrefabLookup` code paths — geometry byte-identical) | **PASS** (vitest) |

## Verify commands (all exit 0 unless noted)

| Command | Result |
|---|---|
| `npm run test -- --run worldObjects chunkStore worldSpatialIndex lodGates` | **4 files, 53 tests PASS** |
| `npm run test -- --run` (full suite) | **16 files, 150 tests PASS** (was 107 @ .5.2.2 + 43 new) |
| `npm run build` | PASS (tsc -b + vite) |
| `npm run lint` | PASS |
| `make schema-validate` | **FAIL on `verify-n6` only — pre-existing at clean HEAD** (see §Pre-existing findings); every other step re-run individually: `validate.mjs`, `verify-map-object-enums`, `verify-map-object-golden`, `verify-map-glyphs`, `verify-type-inventory`, `verify-t090-specs`, `verify-n10` — **all PASS** |

## Hydrate timing (real Everon data, plan §6 budget claim)

Bench: replay of the `applyChunk` math (building filter + OBB corners) over all 275 committed
chunks (`hydrate-bench`, node):

- Main-thread apply: **worst chunk 0.65 ms** (`10_11`: 1,607 rows → 30 buildings); **all 275
  chunks total 16.0 ms**; 6,430 building rows.
- Worker-side gunzip+parse: worst chunk **1.6 ms** — off the main thread by construction.
- Headless-browser store run (16-chunk viewport): `maxApplyMs 0.3`, `framesOverBudget 0`.

The 4 ms/frame budget is ~6× headroom above the worst real chunk; the budget machinery is
vitest-proven for the P2+ future where per-chunk work grows (trees @ T-090.5.5).

## Headless browser smoke (real Worker + Comlink + transferables through Vite)

`VITE_WORLDMAP_ENABLED=1` Vite + chromium headless-shell via raw CDP (temp harness page,
deleted before commit):

```
ping     "world-objects-worker"
manifest {"cells":275,"chunkSizeM":512,"prefabs":391,"hasOversized":false}
chunks   {"n":9,"buildings":119,"isF32":true,"ms":23}            ← transferables intact post-hop
visible  {"atMinus2":119,"atMinus3":0}                            ← gates live in-browser
pick     {"id":null}                                              ← probe was a tree @ −2 → correctly unpickable (N4)
store    {"status":"ready","buildings":196,"stats":{"chunksApplied":16,"applyFrames":1,"maxApplyMs":0.3,"framesOverBudget":0}}
teardown {"status":"idle"}                                        ← reset + terminate path (S4 analogue)
```

Asset serving smoke (Vite dev): manifest / chunk-index / chunk / prefabs / roads all **200**.
Worker bundle: `dist/assets/worldObjects.worker-*.js` **18.8 KB, zero deck.gl** (roads stay
main-thread precisely to keep deck.gl out of the worker graph — see §Doc-sync notes a).
`hasOversized:false` for the current Everon export (no prefab half-extent ≥ 64 m) — the +1
ring path is vitest-covered and self-arms if a future export ships larger prefabs.

## Manual checklist (operator, `VITE_WORLDMAP_ENABLED=1 make web`, hard refresh)

| ID | Check | Status |
|---|---|---|
| W5/R5 | Pan/zoom Everon — roads+buildings visually identical to T-090.5.2.2, FpsCounter ≥55 | **pending operator** |
| S1 | Pan across island — no freeze at chunk boundaries | **pending operator** (headless streaming pass + 0.65 ms worst apply say yes) |
| S2 | Zoom −6→+3 — buildings at −2.5 band; zero tree icons anywhere | **pending operator** (vitest + headless: −3→0, −2→buildings) |
| S3 | Flag OFF — zero regression vs T-090.5.1 (no worker in DevTools) | **pending operator** (hook returns `[]` before any work; store/worker never touched) |
| S4 | Mission unmount/remount — no duplicate workers | **pending operator** (headless teardown → idle; remount respawns lazily) |

## Pre-existing findings (not this slice; surfaced by its verify)

1. **`make schema-validate` fails at `verify-n6` on clean HEAD `346a31c9`** — the canonical N6
   building-geometry sentence drifted in
   `docs/specs/Mission_Creator_Architecture/t090_5_map_object_render_layer.md` (its Buildings
   section now reads "…oriented bounding **rectangle** from measured `spatial.halfExtentsM`…
   supersede OBB when export provides them", vs the gate's verbatim "…Real footprint polygon
   rings are populated only when T-090.3.0 proves Enfusion footprint export; when present,
   polygons supersede OBB rectangles for render."). **Cursor doc sync: restore the canonical
   sentence (or bump the gate) — docs are outside this slice's write scope.**
2. **`buildingPrefabLookup` lint complexity 18 > 15 at clean HEAD** (the T-090.5.2.2 pier/dock
   branch pushed it over; lint was reported clean then). Annotated with a justified
   `eslint-disable-next-line complexity` (repo precedent: MissionCreatorPage keyboard router) —
   zero logic change to the locked module.

## Doc-sync notes for Cursor (API deltas vs `t090_world_objects_worker.md`)

a. **Roads stay a main-thread one-shot** (`worldData.loadWorldRoads`) rather than riding the
   worker: `parseRoadsPayload` lives in `roadLayer.ts`, which imports deck.gl at module scope —
   importing it from the worker would drag deck.gl into the worker bundle. Sanctioned by the
   slice brief ("roads may stay one-shot fetch"); worker stays 18.8 KB.
b. `loadChunksInBbox(bbox, marginCells)` gained a third opts arg
   `{ deckZoom, classes, ids?, excludeIds? }`: `classes ∩ classVisible(cls, deckZoom)` gates
   what crosses the boundary (trees never transfer this slice), `ids` lets the main store pass
   its exact chunkMath set (no dual-computation drift), `excludeIds` suppresses re-delivery.
c. `pickNearest` / `pickRect` take an optional trailing `deckZoom` — when present, only classes
   visible at that zoom are pickable (N4); omitted = unfiltered (T-090.9 decides its default).
d. `loadManifest` returns `WorldManifestLite` incl. `prefabRows` (clone-safe subset the main
   thread feeds to `buildingPrefabLookup`), `cells` (chunk index), `hasOversized`, `roadsPath`.
e. Worker-side chunk LRU mirrors the main formula (`max(64, 3× last request)`, most-recent
   request set never evicted); main-thread `chunkStore` owns the render-cache LRU + pins.
f. `getStatus().ready` now true once a manifest is loaded (5.1 stub said false-until-5.3).
g. Instance ids are `${chunkId}:${rowIndex}` (stable per export); `resolve(id)` joins prefab
   identity for T-090.9.
h. The N4 px→m conversion helper is `worldObjectsClient.worldPickRadiusM(deckZoom)`.

## Commit

- Commit: `T-090.5.3: worker chunk streaming + chunkStore LRU/budget + pick wiring` — tag **T-090.5.3** (tag is the authoritative pointer; sha in the return note).
- `apps/mod/tbd-framework/resourceDatabase.rdb` (pre-existing dirty) excluded.
