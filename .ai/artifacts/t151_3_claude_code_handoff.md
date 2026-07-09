# T-151.3 — Claude Code handoff (chunk residency + spatial index + first world instances)

**Shipped:** @ `32bf5ac5` (tag **T-151.3**, 2026-07-09) — verify log
[`t151_3_verify_log.md`](t151_3_verify_log.md). Cursor doc-sync pass follows.

**Spec (wins on conflict):**
[`t151_3_world_residency.md`](../../docs/specs/Mission_Creator_Architecture/t151_3_world_residency.md)
· **Program hub:**
[`t151_wgpu_engine_program.md`](../../docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md)
· **Working tree:** the standing worktree at `tbd-reforger-wgpu-spike/` (absolute:
`/var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike`) @ `a51e9dcb` (tag **T-151.2**)
or later — **never `main`**. Do **not** run `./scripts/ticket run`. Do **not** create or
checkout slice branches — commit linearly on the worktree's current HEAD.

## Operator report

T-151.2 shipped @ `a51e9dcb` (verify log
[`t151_2_verify_log.md`](t151_2_verify_log.md)): `world/` parse byte-exact on **275** chunks,
census **391 / 508 291 / 888 / 36 / 625**, vitest **343**, wasm **3,858,591 B**. Parse-only — no
streaming, no GPU world draws.

W3 is the **first visible world-object slice on wgpu**: building OBB fills + outlines when
`?engine=wgpu` and `deckZoom ≥ −2.5`. The Deck mount and Comlink worker must remain unchanged.

## What you are building

Five deliverables (Piece 2 opening from
[`t145_world_zerocopy_kickoff.md`](t145_world_zerocopy_kickoff.md)):

1. **`WorldResidency` (Rust):** multi-chunk registry, viewport-driven chunk-id requests, LRU
   eviction, 4 ms/frame ingest budget, wasm `set_viewport` + `ingest_chunk_gz`.
2. **World spatial index (Rust):** chunk-keyed `PointIndex` with `pick_nearest` / `pick_rect`
   returning `${chunkId}:${row}` ids — Class **S** vs rbush on ≥10k probes.
3. **JS fetch shim:** `wgpuWorldLoader.ts` (12 concurrent HTTP fetches) + `useWgpuWorldResidency.ts`
   hook driving viewport updates from `WgpuTacticalMap` camera.
4. **GPU building lanes:** instanced OBB fill + polyline outline batches on `RenderEngine`; readback
   probe for byte-exact center pixel.
5. **Parity harnesses:** `world.residency.parity.test.ts` (chunk sets + eviction order vs
   `createChunkStore`); keep `world.parity.test.ts` green.

## Do not

- Edit `docs/**`, `.ai/tickets/registry.json`, generated ticket views, CLAUDE sync markers.
- Retire the worker, flip Deck `chunkStore`, or change `useWorldMapLayers` / Deck building layers.
- Draw trees, roads, sea, forest, slots, or wire editor world picks (later slices).
- Break W2 `world.parity.test.ts`, T-151.0 spike self-check, or T-151.1 basemap lanes.
- `git checkout -b`, create `ticket/T-151.x` branches, or run `./scripts/ticket run`.

## Execution order (strict)

1. Port chunk-id math → native tests + optional TS cross-check.
2. `WorldResidency` + LRU + budget → wasm bindings.
3. `WorldSpatialIndex` chunk insert/remove + pick wasm API → pick parity test.
4. Building instance struct + GPU upload/draw on `RenderEngine`.
5. JS loader + `useWgpuWorldResidency` + wire `WgpuTacticalMap` viewport.
6. Residency parity test vs `chunkStore` harness.
7. Full verify + readback probe; `.ai/artifacts/t151_3_verify_log.md`; commit `T-151.3:`; tag `T-151.3`.

## Preflight

```bash
cd /var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike
test "$(git rev-parse --show-toplevel)" = "$(pwd)"
git status --porcelain             # empty @ a51e9dcb+
# Do NOT checkout or create branches; do NOT run ./scripts/ticket run
git lfs pull && make map-assets-link
cd apps/website/frontend && npm ci && cd ../../..
make wasm
```

Toolchain: same as T-151.0–2. Map assets linked — chunk files under
`packages/map-assets/everon/objects/chunks/`.

## Key files (surveyed — trust these locations)

| Concern | Path |
|---|---|
| Main-thread stream oracle | `worldmap/chunkStore.ts` + `chunkStore.test.ts` |
| Chunk id math oracle | `worldmap/chunkMath.ts` |
| Worker LRU oracle | `workers/worldObjectsCore.ts` (`evictBeyondCap`, `ensureChunk`) |
| rbush pick oracle | `state/worldSpatialIndex.ts` |
| Building geometry oracle | `worldmap/buildingLayer.ts` (`obbCorners`, `buildingsFromChunkInstances`) |
| LOD gate | `worldmap/lodGates.ts` (`BUILDING_FOOTPRINT_MIN_ZOOM`, `classVisible`) |
| W2 parse + WorldStore | `crates/map-engine-core/src/world/*`, `map-engine-wasm` `WorldStore` |
| PointIndex (slots) | `crates/map-engine-core/src/spatial/point_index.rs` |
| Slot pick parity pattern | `features/_wasm/slotIndex.parity.test.ts` |
| W2 parser parity (regression) | `features/_wasm/world.parity.test.ts` |
| Engine batch list | `crates/map-engine-render/src/engine.rs` |
| Scene instance layout | `crates/map-engine-render/src/scene.rs` |
| wgpu editor mount | `WgpuTacticalMap.tsx` |
| Basemap hook pattern | `wgpu/useWgpuBasemap.ts`, `wgpu/wgpuBasemap.ts` |
| Deck world layers (do not delete) | `worldmap/useWorldMapLayers.ts` |

## Gotchas

- **Dual mount:** only the wgpu path uses `WorldResidency`; Deck still uses `chunkStore` +
  `worldObjectsClient`. Do not change the Deck code path.
- **Pinned set:** the last `set_viewport` chunk-id set is never evicted — eviction order must match
  `chunkStore` test harness (`chunkStore.test.ts` LRU gate).
- **Building class only:** piers/docks are `building` render class in W2 classify — include them in
  building GPU instances.
- **Anchor-relative GPU coords:** building instances use `scene::ANCHOR` [6400, 6400] like stress
  quads — same contract as T-151.0 calibration.
- **Rotation in instance struct:** W3 documents the layout; W5 pins ≤ 20 B icon layout — do not
  break stress/calibration `QuadInstance` size (32 B) used by spike.
- **Viewport hook:** subscribe to wgpu camera view state (same ortho math as basemap) — do not
  import Deck.
- **world.parity.test.ts:** uses single-chunk `WorldStore` API — keep working while adding
  `WorldResidency` (may coexist as separate wasm types).
- **Readback probe:** pick a building at a pinned camera where OBB center lands on integer pixel
  (spike margin-forced pattern from T-151.1 verify log).
- **Prettier + eslint** on touched TS; `make wasm` before `npm test`.

## Verify commands

Spec §Verify verbatim. Minimum new tests: residency parity, pick parity, W2 regression.
Vitest baseline **343** + new tests — record final count in verify log.

## Return to operator / Cursor

- Commit SHA + tag `T-151.3`
- `.ai/artifacts/t151_3_verify_log.md` — gate outputs, residency script log, pick probe summary,
  readback JSON, wasm byte size
- **Ready for Cursor doc sync.**

## Handoff vs spec vs prompt

Spec = decisions + gates (L1–L16). This handoff = context + file map + gotchas. The prompt
(spec §Claude Code prompt) = the send-off. On conflict: spec wins.
