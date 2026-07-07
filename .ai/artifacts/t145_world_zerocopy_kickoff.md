# T-145 — World objects → zero-copy Rust (kickoff for a fresh session)

Standalone brief to continue the Rust/wasm port. **Goal:** move the last two big JS systems — the
**world-object parser** and the **world spatial index + chunk streamer** — into the Rust core, so all
~500k static map objects (trees, roads, buildings) live in **zero-copy wasm memory** and Deck.gl reads
them straight from that buffer. Combined with the already-shipped zero-copy slot doc-core, the editor
then holds a locked frame rate regardless of object count (target: ~165 fps @ 1M). Read this +
`.ai/artifacts/t145_write_swap_kickoff.md` (the doc-core flip that just finished) + memory
`t145-wasm-port.md` + `[[wasm-react-lifecycle]]` first.

## Where things stand (branch `t-145-rust-rewrite`, tree clean, tip `a228ed98`)

- **Backend** Go→Rust (Axum + sqlx): done, Go deleted (`[[t145-rust-backend]]`).
- **map-engine wasm port** (`[[t145-wasm-port]]`): Phase 1 (DEM + geometry leaf math), Phase 2 (mission
  compile crate), Phase 3.0–3.2 (**yrs doc-core flip, F1→F4 complete** — the authored **slots** doc is a
  Rust `yrs` doc behind `WasmMissionDoc`, zero-copy SoA proven @ criterion 6; `yjs`/`y-indexeddb` gone).
- **Still JS (this work):** the **world-object** pipeline (the 391 prefabs / ~508k instances / 275
  chunks + roads + forest) is parsed, indexed, and streamed in JavaScript. Only its DEM/geometry *leaf*
  math already crosses into wasm (`seaBand`/`contours`/`forestMass`/`tbdd`). The **parse, the rbush, and
  the chunk LRU are still JS** — that's what this port finishes.
- Branch is **43 commits ahead of main, 0 behind** (clean FF whenever the operator decides to merge).

## The two pieces (the operator's framing — do them in this order)

### Piece 1 — World parsing → Rust
Port the chunk **parse** into `map-engine-core` so Rust reads the massive JSON/binary exports directly
into a zero-copy world-object **SoA** in wasm memory, off the JS main thread. Today `workers/
worldObjectsCore.ts` does: fetch manifest + prefab tables + chunk index → fetch chunk → gunzip
(`DecompressionStream`) → **parse to SoA typed arrays** → return transferables. The parse + the SoA
build move to Rust; a wasm `WorldStore`/`WorldChunk` handle owns the columns (x/y/prefabId/class/… as
`Vec<f32>`/`Vec<u32>`) with zero-copy `*_ptr`/`len` views, exactly like `doc::soa::SlotSoa`.

### Piece 2 — World spatial index + chunk streamer → Rust
Once parsed, port the **streaming + picking** into Rust. Today: `state/worldSpatialIndex.ts` (an `rbush`
per worker, chunk-granular `insertChunk`/`removeChunk`, class-filtered `pickNearest`/`pickRect`) +
`worldmap/chunkStore.ts` (viewport diff → worker fetch → ≤4 ms/frame apply queue → refcount-pinned
visible set + LRU eviction; `getWorldBuildings()` revision-bumped for pan-stability). Port the index to
the existing Rust grid (`spatial::point_index`, already proven set-equal to rbush) and the chunk
LRU/eviction into the Rust `WorldStore`, so Deck points a `Float32Array` **directly at the wasm chunk
buffers** ("draw these chunks") — bypassing JS GC. This is the same zero-copy mechanism the slot render
proved at criterion 6.

**End state:** static world objects + dynamic slots both in one zero-copy Rust environment → the 165 fps
ceiling.

## Current JS architecture — what to port (files)

- **Parse (worker):** `workers/worldObjectsCore.ts` (the whole streaming brain as a pure factory —
  manifest/prefab/chunk-index fetch, chunk fetch+gunzip+parse→SoA, worker rbush, worker LRU, query API
  `visibleInstances`/`pickNearest`/`pickRect`/`resolve`; returns typed arrays only, never per-instance JS
  objects) · `workers/worldObjects.worker.ts` (thin Comlink shell) · `workers/worldObjectsClient.ts`
  (main-thread client).
- **Stream + index (main + worker):** `worldmap/chunkStore.ts` (viewport cache + apply queue + LRU) ·
  `state/worldSpatialIndex.ts` (rbush factory) · `worldmap/chunkMath.ts` (chunk ids/rects) ·
  `worldmap/lodGates.ts` (`WorldRenderClass` visibility per zoom — `classVisible`, `INSTANCE_BUDGET`).
- **Loader + layers (stay JS / partial):** `worldmap/worldData.ts` (manifest gate + **roads one-shot**,
  main-thread — roads import deck.gl so they must NOT ride the worker) · `worldmap/{building,road,
  treeProp,forestMass}Layer.ts` (Deck layers — the render targets) · `worldmap/useWorldMapLayers.ts`
  (the render hook) · `worldmap/{tree,forestMass}Store.ts`.
- **Export format** (LFS/gitignored under the terrain's asset dir; T-090.3.x): a manifest JSON with an
  `objects` block (prefab table, chunk index, `roadsPath`, density grids) · per-chunk gzipped JSON (or
  the binary `objects/density/{cx}_{cy}.bin` TBDD grids) · `roads.json.gz` (~888 segments) ·
  `forest-regions.json.gz` (36 regions). Read `worldData.ts` + `worldObjectsCore.ts` for the exact wire
  shapes before porting.

## The reusable Rust/wasm template (don't reinvent — mirror the doc-core port)

`crates/map-engine-core/src/` — add a `world/` module alongside `doc/`, `spatial/`, `geometry/`:
- **SoA columns + zero-copy views:** copy `doc/soa.rs` (`SlotSoa`) + the wasm getters/`*_ptr` pattern in
  `map-engine-wasm/src/lib.rs` (`MissionDoc` slot columns + `slot_xy_ptr`). A `WorldChunk`/`WorldStore`
  holds parallel columns; the wasm shim exposes a `Float32Array` view onto them.
- **Spatial index:** `spatial::point_index::PointIndex` (the wasm `SlotIndex`) already replaces rbush and
  is pinned set-equal (`features/_wasm/slotIndex.parity.test.ts`). `worldSpatialIndex` → chunk-keyed
  `PointIndex`(es) with the class filter; `spatial::cluster::ClusterIndex` is the LOD template.
- **Parity discipline (the flip's rulebook):** keep the JS parser/rbush/LRU as the **oracle** and prove
  the Rust twin byte/set-equal via a `features/_wasm/*.parity.test.ts` **before** flipping the app onto
  it — Class **R** (rational → bit-identical `as f32`), **T** (transcendental like `obbCorners` → ≤1 ULP),
  **S** (structural: parse result-set, chunk membership, pick set). Harness: `features/_wasm/parity.ts`.
  Only delete the JS oracle once the app is flipped + green (how F1→F4 removed yjs).

## Gotchas (carry these in)

- **wasm handles in React hooks are effect-local, never `useMemo`** — StrictMode double-frees; `.free()`
  isn't idempotent (`[[wasm-react-lifecycle]]`). The world store lives in a **worker**, so the lifecycle
  is the worker's, but any main-thread wasm handle (the zero-copy view owner) follows this rule.
- **Worker purity:** `worldObjectsCore` must not import deck.gl (bloats the worker bundle) — the world
  wasm module must stay deck-free too. Roads stay a main-thread one-shot for this reason.
- **Worker + wasm build:** `vite.config` needs `worker.format:'es'` + `worker.plugins:[wasm,tsconfigPaths]`
  (workers import wasm → top-level await, illegal in IIFE) — already wired for the DEM/geometry wasm;
  confirm the world module loads the same way.
- **Zero-copy across worker→main:** the criterion-6 slot view aliases wasm memory on the **main** thread.
  For world objects owned by the **worker**, either (a) transfer the parsed typed arrays (current W-rule,
  a copy) or (b) share the worker's `wasm.memory` via **SharedArrayBuffer** (COOP/COEP **credentialless**
  already set for cross-origin Discord avatars) so the main-thread deck view aliases it with no copy —
  decide this early; it's the crux of "165 fps no matter what."
- **Chunk eviction invalidates views:** LRU free/realloc of a chunk's SoA grows/moves wasm memory →
  detaches every `Float32Array` view (the doc-core `refresh()` gotcha). The layer must rebuild views
  after an evict/apply, keyed on a revision (mirror `chunkStore`'s revision + `getWorldBuildings` pan
  stability).
- **⚠️ Asset re-export dependency (blocks the binary-chunk wire):** moving chunks from gzipped-JSON to a
  Rust-native **binary** format needs the mod/Workbench to re-export chunks in that format. Per CLAUDE.md
  the executor gate: **Claude Code cannot run Workbench/mod exports** (`executor: workbench`/`human`). So
  either (a) port the parser to read the **current** JSON-gz wire first (no re-export — do this), and
  defer the binary wire to a later slice that a human runs the export for, or (b) get the operator to
  re-export. Plan piece 1 around the existing wire to stay unblocked.

## Gate (per the flip's discipline)

`make wasm` (rebuild the gitignored pkg) **before** any frontend build/test · `make wasm-ci` (fmt +
clippy `--all-features -D warnings` + core tests) · `cargo test -p map-engine-core --all-features` ·
frontend `npm run test` (the new `world*.parity` oracle green) + `npm run build` + `npm run lint` +
`npx prettier --write` touched files (skip `worldmap/*` pre-existing prettier debt) · `make rust-test-it
> /tmp/it.log 2>&1; echo $?` (never `| tail`). Operator browser gate: fps @ 1M world objects + pan
smoothness + correct pick/LOD (the perceptual half — needs the operator's GPU).

## Sequencing + risk

1. **Piece 1 (parse):** new `world/` core module + wasm `WorldStore` reading the **current JSON-gz wire**
   → SoA; `worldObjectsCore` parse path flips onto it behind a `world*.parity` oracle. Reversible.
2. **Piece 2 (index + streamer):** `worldSpatialIndex` → chunk-keyed `PointIndex`; `chunkStore` LRU →
   Rust `WorldStore` eviction; deck reads the wasm chunk views (zero-copy, SharedArrayBuffer decision).
3. Binary-chunk wire + full asset re-export: **deferred**, needs a human Workbench export (executor gate).
The big-risk item is the worker↔main zero-copy path (SharedArrayBuffer) — spike it early; if it's not
viable, the transfer-typed-arrays fallback still moves parse+index off the main thread (most of the win).

## Opening prompt for the new session

> Continue T-145 — port the **world-object parser** + **world spatial index / chunk streamer** into the
> Rust/wasm core for zero-copy world objects (target 165 fps). Read
> `.ai/artifacts/t145_world_zerocopy_kickoff.md` + memory `t145-wasm-port.md` + `wasm-react-lifecycle.md`
> first. The doc-core flip (F1→F4) is done; slots are already a zero-copy `yrs` wasm doc. Now do **Piece
> 1** (port `workers/worldObjectsCore.ts`'s chunk parse → a Rust `world/` SoA in `map-engine-core`,
> reading the current JSON-gz wire, behind a `features/_wasm/world*.parity` oracle) then **Piece 2**
> (`state/worldSpatialIndex.ts` rbush → chunk-keyed `spatial::point_index`; `worldmap/chunkStore.ts` LRU →
> Rust `WorldStore` eviction; deck reads wasm chunk views). Mirror the doc-core port pattern (SoA +
> `*_ptr` zero-copy view + Class R/T/S byte-parity, keep the JS oracle until flipped). Mind the executor
> gate (binary-chunk re-export needs a human Workbench run — stay on the JSON wire) and the worker↔main
> zero-copy decision (SharedArrayBuffer; COOP/COEP already set). Plan it as its own staged checkpoint.
