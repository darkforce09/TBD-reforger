# T-090 — World objects worker (chunk parse + spatial index off the main thread)

**Status:** **shipped** @ `155651b9` (T-090.5.3) — pick hover UI ships **T-090.9**
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md) · render [`t090_render_lod_contract.md`](t090_render_lod_contract.md)
**Pattern source:** `apps/website/frontend/src/features/tactical-map/compiler/compiler.worker.ts` (T-066, Comlink)

---

## In one sentence

A dedicated Web Worker fetches, gunzips and parses world-object chunks and builds the **world** rbush
spatial index off the main thread, so a 50–500 MB catalog never blocks the UI and the editor holds ≥55 fps (N11).

**v2 (T-090.10.1):** no world supercluster — `visibleInstances(bbox, zoom)` respects `lodGates.ts` density bands; returns typed arrays via transferables.

---

## Why a worker (GAP-H4, GAP-H3)

- The full Everon catalog is **50–500 MB** gzip — parsing on the main thread freezes the tab.
- World picking must mirror `state/slotSpatialIndex.ts` but over a **different, much larger** dataset.
  It uses a **separate world** rbush — the slot index is a single-mounted-doc singleton owned by
  authored slots and must not be shared.
- **v2:** density-gate visibility (`lodGates.classVisible`) replaces tree clustering; forest mass from density grids (T-090.8.1).

**Roads exception:** `roads.json.gz` stays a **main-thread one-shot** (`worldData.loadWorldRoads`) — `parseRoadsPayload` lives in `roadLayer.ts` (deck.gl import) and must not enter the worker bundle (~18.8 KB worker, zero deck.gl).

---

## Module layout

```text
features/tactical-map/workers/
  worldObjectsCore.ts      # pure factory (node-testable): parse, rbush, LRU, visibleInstances, pick
  worldObjects.worker.ts   # thin Comlink shell + fetchBytes + transferable marking
  worldObjectsClient.ts    # typed RPC + worldPickRadiusM + terminateWorldObjects()
worldmap/
  chunkStore.ts            # main-thread LRU, refcount pin, ≤4 ms/frame apply, useSyncExternalStore
state/
  worldSpatialIndex.ts     # createWorldSpatialIndex() factory rbush (W3 — not slot singleton)
```

Mission unmount: `resetWorldStream()` + `terminateWorldObjects()` (mirror `terminateCompiler`).

---

## Comlink API (main thread → worker)

```ts
interface WorldObjectsWorker {
  loadManifest(terrainId: string): Promise<WorldManifestLite>;
  loadChunksInBbox(
    bbox: Bbox,
    marginCells: number,
    opts?: { deckZoom: number; classes?: string[]; ids?: string[]; excludeIds?: string[] },
  ): Promise<ChunkLoadResult>;
  pickNearest(worldXY: [number, number], radiusM: number, deckZoom?: number): Promise<string | null>;
  pickRect(bbox: Bbox, deckZoom?: number): Promise<string[]>;
  visibleInstances(bbox: Bbox, deckZoom: number): Promise<VisibleSet>;
  resolve(id: string): Promise<ResolvedWorldObject>;
  unload(): Promise<void>;
  getStatus(): { ready: boolean }; // ready:true after manifest loaded
}
```

- **`WorldManifestLite`:** `prefabRows` (clone-safe subset for `buildingPrefabLookup`), `cells` (chunk index), `hasOversized`, `roadsPath`, `chunkSizeM`.
- **`loadChunksInBbox` opts:** `classes ∩ classVisible(cls, deckZoom)` gates boundary transfer; `ids` = exact chunkMath set from main store (no dual-computation drift); `excludeIds` suppresses re-delivery.
- **Instance ids:** `${chunkId}:${rowIndex}` (stable per export); `resolve(id)` joins prefab for T-090.9.
- **Pick zoom gate:** optional `deckZoom` on picks — only classes visible at that zoom are pickable (N4). Client helper: `worldPickRadiusM(deckZoom)`.
- **Transferables:** chunk payloads decode to SoA typed arrays (`Float32Array` positions, etc.) — never postMessage 1M JS objects.
- **Prefabs loaded once** in worker; instances stream per chunk. Main-thread `chunkStore` composites for Deck layers.

## Loading strategy (N11 budgets)

| Step | Where | Budget |
|------|-------|--------|
| Fetch + gunzip prefabs | worker | included in P-phase load ms |
| Parse chunk (viewport + border halo) | worker | <500 ms per chunk batch |
| Build/extend world rbush | worker | incremental |
| Main-thread apply | chunkStore | **≤4 ms/frame** (rAF slice) |
| LRU | both | max(64, 3× viewport chunks); pinned/visible never evicted |
| Skip-when-invisible | chunkStore | no chunk requests when all instance classes gated off (e.g. @ −3) |

## Interaction contract (read-only — see T-090.9)

- `pickNearest` / `pickRect` wired @ T-090.5.3; hover UI ships T-090.9.
- Deck GPU pick on world layers **forbidden** (N4); `getCursor` stays constant per T-057/T-063.
- World objects are **read-only** — no mutation RPCs.

## Verification

| ID | Check | Pass |
|----|-------|------|
| W1 | Worker parses golden chunk without main-thread parse | vitest @ `155651b9` |
| W2 | `pickNearest` === brute-force on golden | vitest |
| W3 | World rbush separate from `slotSpatialIndex` | vitest + review |
| W4 | `visibleInstances` respects `lodGates` — no supercluster | vitest |
| W5 | Pan ≥55 fps with world layers on | FpsCounter (operator) |

Verify log: [`.ai/artifacts/t090_5_3_verify_log.md`](../../../.ai/artifacts/t090_5_3_verify_log.md)

## Out of scope

- World object editing/mutation (Workbench only).
- T-110 binary residency compile.

## Related

- [`t090_9_world_object_interaction.md`](t090_9_world_object_interaction.md)
- [`t090_5_map_object_render_layer.md`](t090_5_map_object_render_layer.md)
- [`t063_spatial_index.md`](t063_spatial_index.md) · [`t066_worker_compile.md`](t066_worker_compile.md)
