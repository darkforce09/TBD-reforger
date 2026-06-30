# T-090 — World objects worker (chunk parse + spatial index off the main thread)

**Status:** Spec ready — ships with **T-090.5** render + **T-090.9** interaction
**Authority:** [`t090_091_map_terrain_program.md`](t090_091_map_terrain_program.md) · render [`t090_render_lod_contract.md`](t090_render_lod_contract.md)
**Pattern source:** `apps/website/frontend/src/features/tactical-map/compiler/compiler.worker.ts` (T-066, Comlink)

---

## In one sentence

A dedicated Web Worker fetches, gunzips and parses world-object chunks and builds the **world** rbush
spatial index + **world** cluster index off the main thread, so a 50–500 MB catalog never blocks the UI
and the editor holds ≥55 fps (N11).

---

## Why a worker (GAP-H4, GAP-H3)

- The full Everon catalog is **50–500 MB** gzip — parsing on the main thread freezes the tab.
- World picking must mirror `state/slotSpatialIndex.ts` but over a **different, much larger** dataset.
  It uses a **separate world** rbush — the slot index is a single-mounted-doc singleton owned by
  authored slots and must not be shared.
- Tree clustering at scale (`byKind.tree.instances` from `type-inventory.json`) needs `supercluster.load()` (whole-forest rebuild). That is a **separate
  world** cluster index, distinct from the slot `slotClusterIndex` singleton, and its `load()` runs in
  the worker, never on the UI thread.

---

## Module layout

```text
features/tactical-map/workers/
  worldObjects.worker.ts     # Comlink endpoint: fetch + gunzip + parse + rbush + world cluster
  worldObjectsClient.ts      # main-thread Comlink proxy + lifecycle (terminate on mission unmount)
state/
  worldSpatialIndex.ts       # rbush wrapper, SAME API surface as slotSpatialIndex (separate instance)
  worldClusterIndex.ts       # supercluster wrapper for kind=tree (separate from slotClusterIndex)
```

## Comlink API (main thread → worker)

```ts
interface WorldObjectsWorker {
  loadManifest(terrainId: string): Promise<WorldManifest>;          // prefabs + chunk grid + regions
  loadChunksInBbox(bbox: Bbox, marginCells: number): Promise<ChunkLoadResult>; // streamed, deduped
  pickNearest(worldXY: [number, number], radiusM: number): Promise<string | null>;
  pickRect(bbox: Bbox): Promise<string[]>;                          // world ids in box (read-only)
  clusterTrees(deckZoom: number, bbox: Bbox): Promise<ClusterMarker[]>;
  resolve(id: string): Promise<ResolvedWorldObject>;                // join prefab+instance(+audit)
  unload(): Promise<void>;
}
```

- **Transferables:** chunk payloads decode to typed arrays where possible; positions returned as a
  transferable `Float32Array` (`[x,y]` interleaved) + parallel id list, so the worker→main hop copies
  nothing large.
- **Prefabs loaded once** (~1–5 MB gzip) and cached in the worker; instances stream per chunk.
- **Never** return the full 1M instance array across the boundary — only viewport chunks + the rbush
  query results.

## Loading strategy (N11 budgets)

| Step | Where | Budget |
|------|-------|--------|
| Fetch + gunzip prefabs | worker | included in P-phase load ms |
| Parse chunk (viewport + `marginCells` halo) | worker | <500 ms per chunk batch |
| Build/extend world rbush | worker | incremental; mirrors `slotSpatialIndex.insert` |
| `supercluster.load()` for trees | worker | once per dirty set; P2 budget (N11) |
| Eviction | worker | chunk LRU (size per phase, N11); region index pinned |

## Interaction contract (read-only — see T-090.9)

- `pickNearest` / `pickRect` answer hover + click without any Deck GPU pick (Deck pick on world layers
  is **forbidden**; `getCursor` stays constant per T-057/T-063).
- Hover runs on the existing container `pointermove` (rAF-throttled) and calls `pickNearest`; the legacy
  Deck `onHover` path was **removed** in T-057 and is not reintroduced.
- World objects are **read-only context** — the worker exposes no mutation; moving/deleting terrain
  props is Workbench-only.

## Verification

| ID | Check | Pass |
|----|-------|------|
| W1 | Worker parses the golden chunk sample without main-thread parse | unit (Comlink mock) |
| W2 | `pickNearest` returns the same id as a brute-force scan on the golden | vitest |
| W3 | World rbush is a separate instance from `slotSpatialIndex` (no shared singleton) | code review |
| W4 | `clusterTrees` `load()` runs in the worker (no main-thread supercluster import) | code review |
| W5 | Pan ≥55 fps with 50k visible world instances + basemap | FpsCounter (T-090.5) |

## Out of scope
- World object editing/mutation (Workbench only).
- T-110 binary residency compile (consumer of catalog v1; the worker stays the v1 reader).

## Related
- [`t090_9_world_object_interaction.md`](t090_9_world_object_interaction.md)
- [`t090_5_map_object_render_layer.md`](t090_5_map_object_render_layer.md)
- [`t063_spatial_index.md`](t063_spatial_index.md) · [`t065_cluster_lod.md`](t065_cluster_lod.md) · [`t066_worker_compile.md`](t066_worker_compile.md)
