# T-151.2 — world parser in Rust (`world/` module; W2 Piece 1)

**Status:** **shipped** @ `a51e9dcb` (tag **T-151.2**, 2026-07-08) · verify log
[`t151_2_verify_log.md`](../../../.ai/artifacts/t151_2_verify_log.md) · **Program:**
[`t151_wgpu_engine_program.md`](t151_wgpu_engine_program.md) (W2) · **Executor:** claude-code ·
**Worktree:** `tbd-reforger-wgpu-spike/` (absolute:
`/var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike`; do **not** touch `main`) ·
**Baseline:** `3ab81587` (tag **T-151.1** — verify log
[`t151_1_verify_log.md`](../../../.ai/artifacts/t151_1_verify_log.md)).

## In one sentence

Port the Everon world-object **parse** from `worldObjectsCore.ts` into a Rust `world/` module with
a wasm `WorldStore`, proving byte/set parity against the JS oracle on all **275** real chunk files
before W3 residency and GPU instance draws.

## Problem

T-151.1 made the wgpu editor visually verifiable (basemap stack). World data — **391** prefabs,
**508,291** instances in **275** gzipped chunk JSON files, **888** road segments, **36** forest
regions, **625** TBDD density grids — is still parsed only in JavaScript inside the Comlink worker
(`worldObjectsCore.ts`). Decision **D2** retires that worker once Rust owns parse + residency; **W2
is Piece 1 only:** move parse + SoA materialization into `map-engine-core`, expose zero-copy column
views through wasm, and lock correctness with a differential parity harness. The app **must not**
flip onto Rust parsing this slice — the JS path stays live; W3 wires residency + first GPU draws.

## Goal

1. **`crates/map-engine-core/src/world/`:** `WorldStore` (manifest + prefab table + chunk registry)
   and `WorldChunk` SoA matching `parseChunk` in `worldObjectsCore.ts:571–617`.
2. **Parsers:** chunk instance JSON (`[prefabId, x, y, z, rotationDeg]` rows), `prefabs.json.gz`,
   manifest `objects` block, `roads.json.gz` + Class-T port of `extractRoadCenterline` /
   `parseRoadsPayload`, `forest-regions.json.gz`. TBDD decode reuses existing `decode_tbdd` (already
   parity-pinned in `forest.parity.test.ts`).
3. **Geometry helpers:** `obb_corners` (from `buildingLayer.ts`) and road centerline recovery — Class
   **T** ≤ 1 ULP vs TS oracles.
4. **Wasm surface:** `WorldStore` with `load_manifest`, `load_prefabs`, `parse_chunk`, `load_roads`,
   `load_forest_regions`; per-chunk `*_ptr` / `*_len` getters; exact aggregate counters in `stats()`.
5. **Parity harness:** new `features/_wasm/world.parity.test.ts` — **all 275** Everon chunk files
   Class **R** on SoA columns + Class **S** on per-class row sets; pinned census totals exact.

## Out of scope (later slices — do not build)

- Chunk residency, LRU, viewport diff, fetch concurrency, rbush / `PointIndex` (W3).
- GPU world instance buffers, building OBB draws, road/forest vector layers (W3–W4).
- Flipping `worldObjectsCore` / the worker onto Rust (W3+; optional feature flag **not** required
  in W2).
- Deleting the worker, Deck world layers, or `worldmap/*` (T-151.9).
- Binary chunk wire re-export (D4 deferred).
- Registry/docs edits (Cursor-owned).

## Locked decisions

| # | Decision | Rationale |
|---|---|---|
| L1 | New module tree under `map-engine-core/src/world/`: `mod.rs`, `store.rs`, `chunk.rs`, `prefab.rs`, `manifest.rs`, `classify.rs`, `roads.rs`, `regions.rs`, `obb.rs` | Program hub W2 layout; mirrors `doc/` pattern |
| L2 | `WorldChunk` SoA columns **exactly** as JS `parseChunk`: `positions: Vec<f32>` (2×count xy-pairs), `prefab_idx: Vec<u16>`, `rotations: Vec<f32>`, `z: Vec<f32>`, `cls_codes: Vec<u8>` (255 = unclassified), `rows_by_class: HashMap<RenderClass, Vec<u32>>` gathered row indices | Class **R** memcmp gate; same `as f32` store order |
| L3 | `render_class_for_prefab(kind, class)` ports `renderClassForPrefab` verbatim (`worldObjectsCore.ts:48–66`): building, water pier/dock→building, tree, vegetation, rock→rockLarge, prop/utility→prop, else null→255 | Class **S** row-set gate depends on identical classification |
| L4 | Instance row narrow: reject unless `prefabId` is number and x/y finite; z/rot default 0 when non-finite — same as `narrowInstanceRow` | Oracle parity |
| L5 | Prefab narrow mirrors `narrowPrefabRows` / `narrowSpatial` / `narrowRender`; `build_prefab_maps` mirrors `buildPrefabMaps` including `has_oversized` when `max(halfX,halfY) ≥ 64` | Manifest lite fields for W3 |
| L6 | Chunk JSON: accept gunzip bytes in wasm (`parse_chunk_bytes(id, bytes)`); Rust gunzip or require JS to gunzip first — **prefer Rust gunzip** (sniff `0x1f 0x8b`) matching `bytesToJson` | Worker parity; tests read `.json.gz` from disk |
| L7 | Roads: port `extractRoadCenterline` + `parseRoadsPayload` (`roadLayer.ts:72–124`); `CENTERLINE_DEDUPE_M = 0.05`; width sanity clamp `(0.3, 40)` else style-table fallback | Class **T** on vertices/width; segment count **888** after filter |
| L8 | OBB: port `obbCorners` (`buildingLayer.ts:47–60`); rotation 0° = north (+y), clockwise-positive; Class **T** ≤ 1 ULP vs TS on pinned cases from `buildingLayer.test.ts` | W3 building instances depend on this |
| L9 | Forest regions: parse `forest-regions.json.gz`; region count **36** exact; store id + bbox/polygon fields needed by W4 (structural Class **S** vs TS loader) | Program pinned inventory |
| L10 | Wasm `WorldStore` (separate from `MissionDoc` / `RenderEngine`): `load_manifest_json`, `load_prefabs_gz`, `parse_chunk_gz(id, bytes)`, `load_roads_gz`, `load_forest_regions_gz`; getters `chunk_positions_ptr/len`, `chunk_prefab_idx_ptr/len`, `chunk_rotations_ptr/len`, `chunk_z_ptr/len`, `chunk_cls_codes_ptr/len`, `chunk_rows_by_class_json` (or per-class ptr API — document choice in verify log); `stats()` returns `{prefab_count, instance_count_total, chunk_count_loaded, road_segment_count, forest_region_count, has_oversized}` | D1 shared memory prep; zero-copy views like `slot_xy_ptr` |
| L11 | Export **`parseChunkOracle(id, raw, prefabById)`** from `worldObjectsCore.ts` (test-only re-export of existing internal logic, no worker behavior change) for vitest differential harness | Avoid duplicating 200 lines of oracle in the test file |
| L12 | **`world.parity.test.ts`:** iterate `packages/map-assets/everon/objects/chunks/*.json.gz` (**275** files); for each: JS oracle vs wasm `parse_chunk_gz`; assert `f32BytesEqual` on all SoA arrays, `intArrayEqual` on each `rowsByClass` index list; manifest-level asserts **391 / 508291 / 275 / 888 / 36**; TBDD grid file count **625** via manifest path scan (decode optional smoke on ≥3 grids) | Program hub gates |
| L13 | Commit prefix `T-151.2:`; tag `T-151.2`; verify log `.ai/artifacts/t151_2_verify_log.md` | House convention |

## Pinned numbers (exact assertions)

| Quantity | Value | Source |
|---|---|---|
| Prefabs | **391** | `manifest.json` `objects.prefabCount` |
| World instances | **508,291** | `objects.instanceCount` |
| Chunk files | **275** | `objects/chunks/*.json.gz` |
| Road segments (centerlined) | **888** | program hub + `roads.json.gz` after `parseRoadsPayload` |
| Forest regions | **36** | `forest-regions.json.gz` |
| TBDD density grids | **625** × 1,172 B | `objects/density/{cx}_{cy}.bin` |
| Chunk size | **512 m** | `objects.chunkSizeM` |
| Render classes (instance) | **5** | `RENDER_CLASS_CODES` |
| NO_CLASS sentinel | **255** | `worldObjectsCore.ts:268` |
| Oversized half-extent gate | **64 m** | `OVERSIZED_HALF_EXTENT_M` |
| Vitest baseline | **334** (post T-151.1) | `t151_1_verify_log.md` |
| Merged wasm baseline | **3,723,192 B** | `t151_1_verify_log.md` |

## Tasks

1. **Core module (L1–L5):** `world/` parsers + `WorldStore` + native unit tests (classify, narrow,
   golden chunk from `packages/tbd-schema/golden/map-objects/`).
2. **Roads + OBB (L7–L8):** Rust ports + native tests mirroring `roadLayer.test.ts` /
   `buildingLayer.test.ts`.
3. **Regions + manifest (L5, L9):** forest-regions parse; manifest counters.
4. **Wasm bindings (L10):** `WorldStore` in `map-engine-wasm/src/lib.rs`; ptr/len getters documented.
5. **JS oracle export (L11):** `parseChunkOracle` for vitest only.
6. **Parity harness (L12):** `world.parity.test.ts` full 275-chunk sweep (+ prefabs/roads/regions
   subtests).
7. **Verify + log (L13):** all gates; record wasm byte size delta; T-151.0/1 regression green.

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
npm test                                            # ≥334 + world.parity green
npm run build
npm run lint
! grep -l map_engine_wasm_bg dist/assets/index-*.js
```

**New automated tests (minimum):**

- `crates/map-engine-core/src/world/*.rs` — native tests: classify, chunk golden, road centerline,
  obb corners (Class R/T).
- `features/_wasm/world.parity.test.ts` — **275/275** chunks Class R+S; census totals; roads **888**;
  regions **36**; OBB/road Class T samples ≤ 1 ULP.

**T-151.0 / T-151.1 regression (must stay green):**

- Spike `/_spike/wgpu` self-check + 20M stress; basemap lanes; vitest **334** baseline tests unchanged.

## Manual acceptance

- **S1:** `world.parity.test.ts` runtime recorded in verify log (275-chunk sweep duration acceptable
  on operator machine — target < 120 s cold, document actual).
- **S2:** Spot-check one chunk id in verify log: `positions` length = `2 × count`, `cls_codes`
  distribution matches JS oracle JSON snippet.
- **S3:** No change to Deck editor world-object visuals (`?engine=` off) — parse-only slice.

## Documentation sync (Cursor, after merge)

Registry slice `T-151.2 → shipped` + `shipped_at`; program hub W2 status; verify-log link;
`./scripts/ticket sync && ./scripts/ticket check`.

## Claude Code prompt — T-151.2 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE at tbd-reforger-wgpu-spike/ (NOT main).

Implement **T-151.2** — world parser in Rust (`world/` module; W2 Piece 1).

═══ PREFLIGHT ═══
  cd /var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike
  test "$(git rev-parse --show-toplevel)" = "$(pwd)"
  git status --porcelain            # must be empty @ 3ab81587+ (tag T-151.1)
  # Do NOT checkout or create branches; do NOT run ./scripts/ticket run
  git lfs pull && make map-assets-link
  cd apps/website/frontend && npm ci && cd ../../..
  make wasm

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t151_2_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t151_2_world_parser.md
  3. docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md   (W2 gates)
  4. .ai/artifacts/t145_world_zerocopy_kickoff.md                          (Piece 1 framing)
  5. apps/website/frontend/src/features/tactical-map/workers/worldObjectsCore.ts  (parseChunk oracle)
  6. apps/website/frontend/src/features/tactical-map/worldmap/{roadLayer,buildingLayer}.ts
  7. crates/map-engine-core/src/doc/soa.rs + map-engine-wasm/src/lib.rs      (ptr/len pattern)
  8. apps/website/frontend/src/features/_wasm/{parity.ts,forest.parity.test.ts}

═══ PROBLEM ═══
  ~508k world instances are parsed only in JS (worldObjectsCore worker). W2 moves chunk/prefab/
  manifest/roads/regions parse into Rust with wasm zero-copy column views, proven byte-exact against
  the JS oracle on all 275 Everon chunks — without flipping the live worker or drawing world objects
  yet (W3).

═══ SHIPPED (do not reopen) ═══
  T-151.1 @ 3ab81587 — basemap TBDS/hillshade/grid on wgpu; vitest 334; wasm 3,723,192 B.
  T-151.0 @ f019512d — merged wasm, batch list, editor dual mount, spike self-check.

═══ LOCKED (full table: spec §Locked decisions L1–L13) ═══
  - world/ module: WorldStore + WorldChunk SoA matching parseChunk columns
  - render_class_for_prefab + narrow* + build_prefab_maps verbatim ports
  - roads: extractRoadCenterline + parseRoadsPayload (888 segments)
  - obb_corners Class T ≤1 ULP (buildingLayer oracle)
  - forest regions count 36; TBDD via existing decode_tbdd
  - Wasm WorldStore ptr/len getters + stats counters
  - Export parseChunkOracle from worldObjectsCore for vitest only
  - world.parity.test.ts: 275/275 chunks Class R + row-set Class S
  - Do NOT flip worker to Rust; no GPU world draws; no residency/LRU

═══ DO ═══
  1. crates/map-engine-core/src/world/* — parsers, store, native tests (L1–L9)
  2. map-engine-wasm WorldStore bindings — ptr/len + stats (L10)
  3. Export parseChunkOracle in worldObjectsCore.ts (L11)
  4. features/_wasm/world.parity.test.ts — full Everon chunk sweep + census asserts (L12)
  5. T-151.0/1 regression green; record wasm size delta
  6. Write .ai/artifacts/t151_2_verify_log.md with every gate's verbatim output
  7. Commit prefix T-151.2: · tag T-151.2

═══ DO NOT ═══
  - Edit docs/**, .ai/tickets/registry.json, docs/TICKET_*.md, CLAUDE.md status markers
  - Touch main, RenderEngine world draws, chunkStore residency, world worker flip, Deck world layers
  - Break T-151.0 spike self-check, T-151.1 basemap lanes, or rename/remove prior stats() fields
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
  S1: world.parity 275-chunk sweep duration in verify log
  S2: one chunk id spot-check JSON in verify log
  S3: Deck editor world visuals unchanged (?engine= off)

═══ RETURN ═══
  - Commit SHA + tag T-151.2
  - .ai/artifacts/t151_2_verify_log.md (all gate outputs + parity timing + census asserts)
  - Automated verify output (PASS)
  - Manual notes for S1–S3
  - **Ready for Cursor doc sync.**
```
