# T-151.3 — chunk residency + world spatial index + first world instances (W3)

**Status:** **ready** (executor queue) · **Program:**
[`t151_wgpu_engine_program.md`](t151_wgpu_engine_program.md) (W3) · **Executor:** claude-code ·
**Worktree:** `tbd-reforger-wgpu-spike/` (absolute:
`/var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike`; do **not** touch `main`) ·
**Baseline:** `a51e9dcb` (tag **T-151.2** — verify log
[`t151_2_verify_log.md`](../../../.ai/artifacts/t151_2_verify_log.md)).

## In one sentence

Wire Rust chunk residency (viewport LRU + parse budget), a chunk-keyed world `PointIndex`, and
the first wgpu world visuals — building OBB fills + thin outlines — on `WgpuTacticalMap`, with
Class **S** parity against `chunkStore` / worker rbush oracles before W4 vector layers.

## Problem

T-151.2 proved the world **parse** (275 chunks, census exact) but `WorldStore` still holds one
`last_chunk` at a time and nothing draws world objects on wgpu. The live Deck path streams via
`chunkStore.ts` + the Comlink worker (`worldObjectsCore` + rbush). **W3 is Piece 2 kickoff:**
Rust owns residency (which chunks are loaded, LRU eviction, ≤ 4 ms/frame ingest budget), replaces
the worker spatial index with chunk-keyed `PointIndex`, and uploads **building** instances to the
wgpu engine — **buildings only** at `deckZoom ≥ −2.5`. The Deck mount and worker stay the oracle;
the wgpu mount gains the first world layer stack.

## Goal

1. **`WorldResidency` in Rust:** multi-chunk registry; viewport → chunk-id set mirroring
   `chunkMath.ts` (+ 5 % preload margin + oversized +1 ring); LRU `max(64, 3 × pinned)` with
   pinned immunity; ≤ **4 ms**/frame amortized parse/ingest budget; exact chunk-id + eviction-order
   parity vs `chunkStore`.
2. **Thin JS fetch loader:** 12-way concurrent chunk byte fetch (`DecompressionStream` optional —
   Rust gunzip from W2); feeds gz bytes into wasm ingest; no per-frame JS SoA consumer.
3. **Chunk-keyed world index:** class-filtered `pick_nearest` / `pick_rect` returning stable
   `${chunkId}:${rowIndex}` ids — Class **S** vs worker rbush on **10k** scripted probes.
4. **GPU building lane:** per-resident-chunk building instance buffers on `RenderEngine`; rotated
   OBB fill quads + thin outline polylines (dark casing color from Deck); instance layout documents
   rotation field (step toward W5 ≤ 20 B icon layout).
5. **`WgpuTacticalMap` wire-up:** viewport-driven residency hook; draw order basemap → hillshade →
   buildings (fill → outline) → grid; LOD gate `building` only via `classVisible`.

## Out of scope (later slices — do not build)

- Retiring the Comlink worker or flipping Deck `chunkStore` to Rust (Deck path unchanged).
- Trees, props, vegetation, roads, sea, contours, forest, landcover, badges (W4–W5).
- World pick wiring in the editor gesture machine (W7); picks are harness-only this slice.
- Deleting Deck `worldmap/*Layer.ts` (T-151.9).
- Registry/docs edits (Cursor-owned).

## Locked decisions

| # | Decision | Rationale |
|---|---|---|
| L1 | New `world/residency.rs` + `world/index.rs`; evolve `WorldStore` into **`WorldResidency`** holding `HashMap<String, WorldChunk>` + manifest/prefab tables from W2 | Multi-chunk residency; W2 `parse_chunk_gz` logic reused per ingest |
| L2 | Chunk-id set math ports `chunkMath.ts`: `preloadMarginM` = max(5 % span, one chunk ring); `chunkIdsForViewport`; intersect manifest `cells` when present; +1 ring when `has_oversized` | Class **S** gate vs `chunkStore` |
| L3 | LRU cap = `max(LRU_MIN_CHUNKS, 3 × pinned.len())` with **LRU_MIN_CHUNKS = 64**; `pinned` = last requested id set; pinned ids never evicted; evict ascending `last_used` among non-pinned | Mirrors `chunkStore.ts:155–168` + worker `evictBeyondCap` |
| L4 | Ingest budget **APPLY_BUDGET_MS = 4.0** per frame; `stats()` exposes `apply_budget_ms_last`, `apply_frames_over_budget`, `chunks_resident`, `chunks_pinned` | Program hub gate; measured not eyeball |
| L5 | JS **`wgpuWorldLoader.ts`** + **`useWgpuWorldResidency.ts`**: fetch concurrency **12**; `WorldResidency.set_viewport(bbox, deck_zoom)` wasm API; bytes → `ingest_chunk_gz(id, bytes)` | D2: JS fetch, Rust parse once |
| L6 | **`WorldSpatialIndex`** in Rust: chunk-granular `insert_chunk` / `remove_chunk`; entries map to global row handles or string ids `${id}:${row}`; class filter callback or visible-class bitmask at query time | Replaces worker rbush for wgpu path |
| L7 | Pick parity uses **world meters** radius (contract N2 **12 px** converted via `r_world = unproject(px+12) − unproject(px)` only in tests — engine hook may accept `radius_m` directly) | `worldSpatialIndex.ts` semantics |
| L8 | Building GPU: new batch lane(s) on existing pipelines — **OBB fill** via instanced axis-aligned quads in **world-oriented space** (rotation in instance struct) + **outline** via `Polyline` (near-black `[30,30,34,α]` casing) | First world visuals; matches Deck building fill/outline intent |
| L9 | Wgpu draw order (W3 only): basemap → hillshade → **world-buildings** → **world-buildings-outline** → grid | W4 inserts sea/roads above basemap |
| L10 | Hydrate/render **`building` class only** (`HYDRATE_RENDER_CLASSES = ['building']`); skip viewport work when `!classVisible('building', deckZoom)` — mirror `chunkStore` early exit | T-090.5.3 scope; pier/dock ride building class |
| L11 | Building colors: dark fill from Deck `buildingLayer` default (document exact RGBA in verify log); outline 1 px world-scaled at zoom | Operator visual pass deferred; readback probe is byte-exact gate |
| L12 | **`world.residency.parity.test.ts`:** deterministic viewport script (≥20 steps pan/zoom) → chunk-id set Class **S** vs `createChunkStore` fake client; LRU eviction order log Class **S** | Program hub gate |
| L13 | **`world.pick.parity.test.ts`:** ≥10k random (x,y,radius,class-filter) probes — result set Class **S** vs `createWorldSpatialIndex` rbush oracle on a fixed multi-chunk fixture | Program hub gate |
| L14 | **GPU readback probe:** pinned camera + known building OBB center → fill pixel RGBA byte-exact (margin-forced integer pixel coords, spike pattern) | Class **GPU-R** |
| L15 | `RenderEngine::stats()` T-151.0/1 fields untouched; additive world keys only (`world_building_instances`, `world_chunks_drawn`, …) | Regression isolation |
| L16 | Commit prefix `T-151.3:`; tag `T-151.3`; verify log `.ai/artifacts/t151_3_verify_log.md` | House convention |

## Pinned numbers (exact assertions)

| Quantity | Value | Source |
|---|---|---|
| Chunk size | **512 m** | manifest `objects.chunkSizeM` |
| LRU floor | **64** | `chunkStore.ts` `LRU_MIN_CHUNKS` |
| LRU formula | **max(64, 3 × pinned)** | plan §6 |
| Apply budget | **4 ms**/frame | `APPLY_BUDGET_MS` |
| Fetch concurrency | **12** | `worldObjectsCore` `DEFAULT_FETCH_CONCURRENCY` |
| Preload margin | **max(5 % viewport span, 512 m)** | `chunkMath.preloadMarginM` |
| Building LOD gate | **−2.5** | `BUILDING_FOOTPRINT_MIN_ZOOM` |
| World pick radius | **12 px** (tests convert to meters) | contract N2 |
| Pick probe count | **≥10 000** | program hub W3 |
| Viewport script steps | **≥20** | residency parity |
| Vitest baseline | **343** (post T-151.2) | `t151_2_verify_log.md` |
| Merged wasm baseline | **3,858,591 B** | `t151_2_verify_log.md` |
| Census (unchanged) | **391 / 508 291 / 275** | W2 parity must stay green |

## Tasks

1. **Residency core (L1–L4):** `WorldResidency` + native tests (LRU, pinned immunity, budget).
2. **Chunk math port (L2):** Rust `chunk_math` module + vitest Class **S** vs TS oracle (reuse or
   mirror `chunkMath.test.ts` if present).
3. **World index (L6–L7, L13):** chunk-keyed index + wasm pick API + pick parity test.
4. **JS loader + hook (L5, L10):** `wgpuWorldLoader.ts`, `useWgpuWorldResidency.ts`.
5. **Engine building lanes (L8–L9, L11, L14):** instance struct + upload + draw + readback API.
6. **`WgpuTacticalMap` (L9–L10):** viewport subscription → residency → engine upload.
7. **Residency parity test (L12)** + keep **`world.parity.test.ts`** green (W2 regression).
8. **Verify + log (L16):** all gates; wasm size delta; T-151.0/1/2 regression.

## Verify (all exit 0; run from worktree root)

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings
cargo test -p map-engine-core --all-features
cargo test -p map-engine-render
cargo build --workspace
make wasm
cd apps/website/frontend
npm test                                            # ≥343 + new W3 tests green
npm run build
npm run lint
! grep -l map_engine_wasm_bg dist/assets/index-*.js
```

**New automated tests (minimum):**

- `crates/map-engine-core/src/world/residency.rs` — native LRU + budget tests.
- `crates/map-engine-core/src/world/index.rs` — native pick tests vs brute force.
- `features/tactical-map/wgpu/chunkMathRust.test.ts` or `_wasm/world.residency.parity.test.ts` —
  chunk-id set Class **S** vs `chunkStore` harness.
- `features/_wasm/world.pick.parity.test.ts` — ≥10k probes Class **S** vs rbush.
- `features/_wasm/world.parity.test.ts` — **unchanged green** (W2 regression).

**T-151.0 / T-151.1 / T-151.2 regression:**

- Spike self-check + 20M stress; basemap lanes; 275-chunk world parser parity.

## Manual acceptance

- **S1:** `/missions/:id/edit?engine=wgpu` @ deckZoom ≥ −2.5 — building footprints visible on
  Everon (dark fills + thin outlines); pan/zoom updates chunks without tab freeze.
- **S2:** Same viewport @ `?engine=` off (Deck) — unchanged (oracle path).
- **S3:** HUD / stats shows `world_building_instances` > 0 when buildings in view; apply budget
  frames_over_budget = 0 on scripted pan path in verify log.
- **S4:** Readback JSON for building center pixel pasted in verify log (byte-exact).

## Documentation sync (Cursor, after merge)

Registry slice `T-151.3 → shipped` + `shipped_at`; program hub W3 status; verify-log link;
`./scripts/ticket sync && ./scripts/ticket check`.

## Claude Code prompt — T-151.3 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE at tbd-reforger-wgpu-spike/ (NOT main).

Implement **T-151.3** — chunk residency + world spatial index + first world instances (W3).

═══ PREFLIGHT ═══
  cd /var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike
  test "$(git rev-parse --show-toplevel)" = "$(pwd)"
  git status --porcelain            # must be empty @ a51e9dcb+ (tag T-151.2)
  # Do NOT checkout or create branches; do NOT run ./scripts/ticket run
  git lfs pull && make map-assets-link
  cd apps/website/frontend && npm ci && cd ../../..
  make wasm

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t151_3_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t151_3_world_residency.md
  3. docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md   (W3 gates)
  4. .ai/artifacts/t145_world_zerocopy_kickoff.md                          (Piece 2 framing)
  5. apps/website/frontend/src/features/tactical-map/worldmap/{chunkStore,chunkMath}.ts
  6. apps/website/frontend/src/features/tactical-map/state/worldSpatialIndex.ts
  7. apps/website/frontend/src/features/tactical-map/worldmap/buildingLayer.ts
  8. crates/map-engine-core/src/world/* + spatial/point_index.rs
  9. crates/map-engine-render/src/{engine.rs,scene.rs,lanes.rs}
  10. apps/website/frontend/src/features/tactical-map/WgpuTacticalMap.tsx
  11. apps/website/frontend/src/features/_wasm/{world.parity.test.ts,slotIndex.parity.test.ts}

═══ PROBLEM ═══
  W2 parse is proven but nothing streams or draws world objects on wgpu. W3 adds Rust chunk
  residency (LRU + 4 ms budget), a chunk-keyed PointIndex, building OBB GPU lanes on WgpuTacticalMap,
  and Class S parity vs chunkStore/rbush — without retiring the Deck worker path.

═══ SHIPPED (do not reopen) ═══
  T-151.2 @ a51e9dcb — world/ parser, 275-chunk Class R/S, wasm 3,858,591 B, vitest 343.
  T-151.1 @ 3ab81587 — basemap stack on wgpu.
  T-151.0 @ f019512d — merged wasm, batch list, spike self-check.

═══ LOCKED (full table: spec §Locked decisions L1–L16) ═══
  - WorldResidency: multi-chunk map, viewport chunk-id set, LRU max(64, 3×pinned), 4 ms ingest budget
  - JS wgpuWorldLoader (12 concurrent fetch) → wasm ingest_chunk_gz
  - Chunk-keyed WorldSpatialIndex; pick ids `${chunkId}:${row}`
  - Building fill + outline on wgpu only; class building @ zoom ≥ −2.5
  - Draw order: basemap → hillshade → buildings → outline → grid
  - world.residency.parity.test.ts (chunk sets + eviction order Class S)
  - world.pick.parity.test.ts (≥10k probes Class S vs rbush)
  - GPU readback building center pixel byte-exact
  - world.parity.test.ts (W2) stays green; Deck chunkStore/worker untouched

═══ DO ═══
  1. world/residency.rs + world/index.rs + chunk_math port + native tests (L1–L4, L6)
  2. Wasm WorldResidency API + pick methods (L5–L7)
  3. wgpuWorldLoader.ts + useWgpuWorldResidency.ts (L5, L10)
  4. RenderEngine building instance lanes + readback probe (L8–L9, L11, L14–L15)
  5. WgpuTacticalMap viewport → residency → GPU upload (L9–L10)
  6. world.residency.parity.test.ts + world.pick.parity.test.ts (L12–L13)
  7. T-151.0/1/2 regression; write .ai/artifacts/t151_3_verify_log.md
  8. Commit prefix T-151.3: · tag T-151.3

═══ DO NOT ═══
  - Edit docs/**, .ai/tickets/registry.json, docs/TICKET_*.md, CLAUDE.md status markers
  - Touch main, retire worker, flip Deck chunkStore, or delete worldmap Deck layers
  - Draw trees/roads/sea/forest/slots (W4–W6); wire editor picks (W7)
  - Break world.parity.test.ts, spike self-check, basemap lanes, or prior stats() fields
  - git checkout -b / create ticket/T-151.x branches
  - ./scripts/ticket run

═══ VERIFY (all exit 0) ═══
  cargo fmt --check
  cargo clippy --all-targets -- -D warnings
  cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings
  cargo test -p map-engine-core --all-features
  cargo test -p map-engine-render
  cargo build --workspace
  make wasm
  cd apps/website/frontend && npm test && npm run build && npm run lint
  ! grep -l map_engine_wasm_bg dist/assets/index-*.js

═══ MANUAL ═══
  S1: ?engine=wgpu buildings visible @ zoom ≥ −2.5, pan smooth
  S2: Deck path unchanged (?engine= off)
  S3: stats apply budget + world_building_instances in verify log
  S4: readback building center pixel JSON byte-exact

═══ RETURN ═══
  - Commit SHA + tag T-151.3
  - .ai/artifacts/t151_3_verify_log.md (all gate outputs + parity scripts + readback JSON)
  - Automated verify output (PASS)
  - Manual notes for S1–S4
  - **Ready for Cursor doc sync.**
```
