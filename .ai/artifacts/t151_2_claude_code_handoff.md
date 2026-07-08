# T-151.2 — Claude Code handoff (world parser in Rust; W2 Piece 1)

**Spec (wins on conflict):**
[`t151_2_world_parser.md`](../../docs/specs/Mission_Creator_Architecture/t151_2_world_parser.md)
· **Program hub:**
[`t151_wgpu_engine_program.md`](../../docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md)
· **Working tree:** the standing worktree at `tbd-reforger-wgpu-spike/` (absolute:
`/var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike`) @ `3ab81587` (tag **T-151.1**)
or later — **never `main`**. Do **not** run `./scripts/ticket run`. Do **not** create or
checkout slice branches — commit linearly on the worktree's current HEAD.

## Operator report

T-151.1 shipped @ `3ab81587` (verify log
[`t151_1_verify_log.md`](t151_1_verify_log.md)): wgpu basemap stack (TBDS + hillshade + grid),
vitest **334**, merged wasm **3,723,192 B**, GPU readback gates byte-exact via headless CDP.
T-151.0 spike + editor dual mount remain regression harnesses.

W2 is **parse-only** — the operator should see **no** visible change in the editor. Success is
automated: `world.parity.test.ts` green on all **275** Everon chunks with census totals exact.

## What you are building

Three deliverables (Piece 1 from
[`t145_world_zerocopy_kickoff.md`](t145_world_zerocopy_kickoff.md)):

1. **Rust `world/` module (L1–L9):** `WorldStore` + `WorldChunk` SoA; ports of
   `parseChunk`, `narrowPrefabRows`, `buildPrefabMaps`, `renderClassForPrefab`; roads
   (`extractRoadCenterline`, `parseRoadsPayload`); `obbCorners`; forest-regions parse; manifest
   counters including `has_oversized`.
2. **Wasm `WorldStore` (L10):** zero-copy `*_ptr`/`*_len` getters on chunk columns (mirror
   `MissionDoc.slot_xy_ptr` pattern in `map-engine-wasm/src/lib.rs`); `stats()` aggregate counters.
3. **Parity harness (L11–L12):** export `parseChunkOracle` from `worldObjectsCore.ts`; add
   `features/_wasm/world.parity.test.ts` sweeping `packages/map-assets/everon/objects/chunks/*.json.gz`.

## Do not

- Edit `docs/**`, `.ai/tickets/registry.json`, generated ticket views, CLAUDE sync markers.
- Flip `worldObjectsCore` / the Comlink worker onto Rust (W3+).
- Add chunk residency, LRU, spatial index, or GPU world instance rendering (W3–W4).
- Touch Deck world layers, `chunkStore.ts` apply queue, or delete the worker.
- Break T-151.0 spike self-check / 20M stress or T-151.1 basemap lanes.
- `git checkout -b`, create `ticket/T-151.x` branches, or run `./scripts/ticket run`.

## Execution order (strict)

1. Core `world/` types + classify + chunk parse → native golden tests (tbd-schema golden chunk).
2. Prefab + manifest parse → prefab_count **391**, has_oversized flag.
3. Roads + OBB Rust ports → native tests from `roadLayer.test.ts` / `buildingLayer.test.ts`.
4. Forest regions parse → region count **36**.
5. Wasm `WorldStore` bindings + `make wasm`.
6. Export `parseChunkOracle`; write `world.parity.test.ts`; run 275-chunk sweep.
7. Full verify; `.ai/artifacts/t151_2_verify_log.md`; commit `T-151.2:`; tag `T-151.2`.

## Preflight

```bash
cd /var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike
test "$(git rev-parse --show-toplevel)" = "$(pwd)"
git status --porcelain             # empty @ 3ab81587+
# Do NOT checkout or create branches; do NOT run ./scripts/ticket run
git lfs pull && make map-assets-link
cd apps/website/frontend && npm ci && cd ../../..
make wasm
```

Toolchain: same as T-151.0/1 (rustc 1.95, wasm-pack 0.15, node 26). Map assets must be linked —
275 chunk files under `packages/map-assets/everon/objects/chunks/`.

## Key files (surveyed — trust these locations)

| Concern | Path |
|---|---|
| JS parse oracle (parseChunk) | `apps/website/frontend/src/features/tactical-map/workers/worldObjectsCore.ts:571–617` |
| Classify + prefab narrow | same file `:48–66`, `:291–404` |
| Worker tests / fixtures | `workers/worldObjectsCore.test.ts` |
| Road centerline oracle | `worldmap/roadLayer.ts:72–124` + `roadLayer.test.ts` |
| OBB oracle | `worldmap/buildingLayer.ts:47–60` + `buildingLayer.test.ts` |
| Everon manifest | `packages/map-assets/everon/manifest.json` |
| Chunk files (275) | `packages/map-assets/everon/objects/chunks/*.json.gz` |
| Prefabs | `packages/map-assets/everon/objects/prefabs.json.gz` |
| Roads | `packages/map-assets/everon/objects/roads.json.gz` |
| Forest regions | `packages/map-assets/everon/objects/forest-regions.json.gz` |
| TBDD grids (625) | `packages/map-assets/everon/objects/density/*.bin` |
| Golden chunk (unit test) | `packages/tbd-schema/golden/map-objects/map-object-chunk-sample.json` |
| Parity harness utils | `features/_wasm/parity.ts` |
| TBDD wasm (exists) | `crates/map-engine-core/src/geometry/tbdd.rs` |
| Slot SoA ptr pattern | `crates/map-engine-core/src/doc/soa.rs`, `map-engine-wasm/src/lib.rs` |
| Zero-copy kickoff | `.ai/artifacts/t145_world_zerocopy_kickoff.md` |

## Gotchas

- **f32 store order:** JSON numbers → `as f32` in JS; Rust must use `f32::from()` / `as f32` on
  the same values — Class **R** compares raw f32 bits (`f32BytesEqual`).
- **rowsByClass:** keys are render class strings (`building`, `tree`, …); values are row index
  arrays built only for classified instances (code ≠ 255).
- **Chunk id format:** `"cx_cy"` string; split on `_` for cx/cy integers.
- **Gunzip:** real chunk files are `.json.gz`; parity test reads raw bytes; Rust parser should
  accept gzip (sniff magic) like `bytesToJson` in worldObjectsCore.
- **275-chunk test runtime:** use vitest with adequate timeout; log wall time in verify log. Run
  from `apps/website/frontend` with cwd-relative path to `packages/map-assets` (same as
  `worldObjectsCore.test.ts:35`).
- **Road count:** `parseRoadsPayload` drops degenerate segments — assert **888** centerlined
  segments, not raw export row count.
- **Do not import deck.gl in Rust tests** — keep road/building geometry as pure f64/f32 math ports.
- **Wasm size:** expect growth from WorldStore; record delta from **3,723,192 B** baseline.
- **Prettier + eslint** on touched TS; `make wasm` before `npm test`.

## Verify commands

Spec §Verify verbatim. Minimum new tests: native `world/*` tests + `world.parity.test.ts`.
Vitest baseline **334** + new tests — record final count in verify log.

## Return to operator / Cursor

- Commit SHA + tag `T-151.2`
- `.ai/artifacts/t151_2_verify_log.md` — gate outputs, 275-chunk sweep timing, census asserts,
  wasm byte size, sample chunk id spot-check
- **Ready for Cursor doc sync.**

## Handoff vs spec vs prompt

Spec = decisions + gates (L1–L13). This handoff = context + file map + gotchas. The prompt
(spec §Claude Code prompt) = the send-off. On conflict: spec wins.
