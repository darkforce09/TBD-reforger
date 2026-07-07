# T-151.0 — wasm packaging merge + engine batch list + editor dual mount

**Program:** [`t151_wgpu_engine_program.md`](t151_wgpu_engine_program.md) (W0) ·
**Executor:** claude-code · **Branch:** `t-151-wgpu-spike` (worktree
`tbd-reforger-wgpu-spike/`; do **not** touch `main`) · **Baseline:** `94261dd6` (spike
complete, all gates green — verify log
[`t151_wgpu_spike_verify_log.md`](../../../.ai/artifacts/t151_wgpu_spike_verify_log.md)).

## In one sentence

Merge the `--target web` render pkg into the single bundler wasm module (one linear memory
shared by `MissionDoc` and `RenderEngine`), refactor the engine's hardcoded draws into an
ordered batch list, and mount a minimal `WgpuTacticalMap` inside the Mission Creator editor
shell behind an engine flag — with every shipped spike gate re-run green on the merged module.

## Problem

The spike engine lives in its own wasm-pack `--target web` package with its own linear memory,
so `MissionDoc.slot_xy_ptr` (doc core) and the engine cannot share buffers — the zero-copy
doc→GPU path (program D1) is impossible across two wasm instances. The engine also hardcodes
its two draw calls (stress pool + calibration), leaving no seam for the W1+ layer stack, and
it is reachable only at `/_spike/wgpu`, not inside the editor shell where the migration
(program D3) must be verified slice by slice.

## Goal

1. **One wasm module (D1):** `map-engine-render` compiled into `map-engine-wasm` (bundler
   target, existing `make wasm` output at `apps/website/frontend/src/wasm/pkg/`);
   `RenderEngine` importable from `@/wasm/pkg/map_engine_wasm`; the `--target web` pkg,
   its Makefile target, and its ignore entries removed.
2. **Batch list:** engine internals refactored to an ordered `Vec<Batch>` (pipeline kind +
   buffers + instance range + visibility). Behavior identical this slice — stress chunks then
   calibration, same clear color, same self-check — the refactor is the seam, not a feature.
3. **Dual mount (D3):** new `WgpuTacticalMap` rendered by `MissionCreatorPage` instead of the
   Deck `TacticalMap` when `VITE_MC_ENGINE=wgpu` or `?engine=wgpu`; accepts the same props
   (unused ones ignored this slice); shows the calibration scene + a small HUD (backend, fps,
   shared-memory check) full-bleed in the chromeless editor route.

## Out of scope (later slices — do not build)

- Basemap/satellite/hillshade/grid rendering (T-151.1), world parsing (T-151.2), any new
  pipeline beyond the existing `quad-instanced` (T-151.4/5), slot rendering from the doc
  (T-151.6), interaction beyond the existing pan/wheel (T-151.7), culling (T-151.8).
- Deleting any Deck.gl code (T-151.9). The Deck `TacticalMap` path must keep working
  unchanged with the flag off.
- Binary chunk wire, worker changes, registry/docs edits (Cursor-owned).

## Locked decisions

| # | Decision | Rationale |
|---|---|---|
| L1 | `map-engine-wasm/Cargo.toml` gains `map-engine-render = { path = "../map-engine-render" }`; `lib.rs` gains `#[cfg(target_arch = "wasm32")] pub use map_engine_render::RenderEngine;` | Guarantees linkage so wasm-bindgen exports the class; native builds untouched (render's GPU deps are wasm32-gated) |
| L2 | `map-engine-render` `crate-type` becomes `["rlib"]` | It is no longer wasm-packed directly; cdylib is dead weight |
| L3 | The render crate keeps its `#[wasm_bindgen(start)]` panic hook **unless** wasm-bindgen rejects a duplicate start across linked crates — in that case convert it to an explicit `init_panic_hook()` export called from the TS glue, and say so in the verify log | Single-start constraint is a wasm-bindgen rule; the fallback is behavior-equivalent |
| L4 | Delete `make wasm-render`; keep the render crate's fmt/clippy/test lines in `wasm-ci`; remove `apps/website/frontend/src/wasm/render/` + its `.gitignore` and eslint `globalIgnores` entries | One build product |
| L5 | TS glue: `features/_spike/wgpu/wasmRender.ts` moves to `features/tactical-map/wgpu/wasmRender.ts` (shared by spike page + editor mount); the web-target `init()` memoization is **deleted** (bundler target auto-instantiates; ESM guarantees the singleton); the **creation mutex (I3) and `deviceSize` stay** | I-invariants I1 is now the module system's job; I3–I7 remain load-bearing |
| L6 | `deviceSize.test.ts` moves with the glue (`features/tactical-map/wgpu/deviceSize.test.ts`), assertions unchanged | Tests follow the code |
| L7 | Batch struct shape: `{ kind: PipelineKind (one variant: QuadInstanced), instances: wgpu::Buffer, count: u32, visible: bool }` in draw order stress→calibration; `stats()` JSON fields unchanged; `self_check()` untouched | Seam without behavior change — the spike gates must pass byte-identically |
| L8 | Engine flag: `const useWgpu = import.meta.env.VITE_MC_ENGINE === 'wgpu' \|\| new URLSearchParams(window.location.search).get('engine') === 'wgpu'` evaluated in `MissionCreatorPage`; Deck path untouched when false | D3 dual mount |
| L9 | `WgpuTacticalMap.tsx` lives at `features/tactical-map/WgpuTacticalMap.tsx`, accepts `TacticalMapProps` (import the existing type; prefix-underscore unused ones), reuses lifecycle invariants I2–I7 from `WgpuCanvas.tsx` verbatim (effect-local handle, disposed-after-await free, rAF cancel before free, error banner, fresh-canvas retry) | One lifecycle discipline |
| L10 | Shared-memory proof is **numeric**: the mount (and the spike page) creates a `MissionDoc`, calls `seed_random(1000, 12800, 12800, 0x12345678)`, `refresh()`, builds `new Float32Array(wasmBg.memory.buffer, doc.slot_xy_ptr, 2000)` from the **same** `map_engine_wasm_bg.wasm` module namespace the engine came from, and asserts all 2000 values are finite ∧ ≥ 0 ∧ ≤ 12800 — HUD renders `shared-memory: PASS (2000/2000 in [0,12800])` or FAIL with the first offending index | "One memory" becomes a checked predicate, not an architecture claim |
| L11 | Commit prefix `T-151.0:`; tag `T-151.0`; verify log `.ai/artifacts/t151_0_verify_log.md` | House convention |

## Tasks

1. **Cargo merge (L1, L2):** dependency + re-export; `cargo build --workspace` and
   `cargo check -p map-engine-wasm --target wasm32-unknown-unknown` compile; resolve the
   start-fn collision per L3 only if the build forces it.
2. **`make wasm`** produces the merged pkg; confirm `RenderEngine` + `OrthoCameraJs` +
   `MissionDoc` all present in `src/wasm/pkg/map_engine_wasm.d.ts`. Record
   `wc -c map_engine_wasm_bg.wasm` (expected ≈ old pkg + ~2.8 MB engine payload).
3. **Makefile/ignores (L4):** remove the web-target path end to end.
4. **TS glue move (L5, L6):** repoint `features/_spike/wgpu/WgpuCanvas.tsx` imports at the
   merged pkg via the moved glue; delete `src/wasm/render/` references; spike page must behave
   identically (it is the regression harness for the merge).
5. **Batch refactor (L7):** `engine.rs` draws iterate the batch list; stress
   `seed_stress`/`clear_stress` mutate batches; calibration is a permanent batch.
6. **Dual mount (L8, L9, L10):** `WgpuTacticalMap.tsx` + the `MissionCreatorPage` switch +
   the shared-memory HUD check.
7. **Verify + log (L11):** run the full gate list below; write the verify log with every
   command's output verbatim; commit + tag.

## Verify (all exit 0; run from the worktree root)

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings
cargo test -p map-engine-core --all-features        # 66 (56+5+5) — unchanged
cargo test -p map-engine-render                     # 4 — unchanged
cargo build --workspace
make wasm                                           # merged pkg
cd apps/website/frontend
npm test                                            # ≥317; orthoCamera.parity + deviceSize green
npm run build
npm run lint
# Entry-chunk isolation: the wasm module must load only via lazy route chunks.
! grep -l map_engine_wasm_bg dist/assets/index-*.js
```

Browser (numeric, both paths):

- `/_spike/wgpu` on the merged pkg: `Run self-check` → `pass: true` on the detected backend
  **and** `?force=webgl`; 20M stress re-recorded (`instances == 20000000`,
  `staging_peak_bytes == 67108864`, `uniform_bytes_last_frame == 64`, fps + `gpu_frame_ms`
  noted — expected within the shipped family: webgl2 58–70 fps, webgpu ~67 fps / ~14 ms).
- `/missions/:id/edit?engine=wgpu`: calibration scene renders in the editor shell; HUD shows
  backend + `shared-memory: PASS (2000/2000 in [0,12800])` (L10).
- `/missions/:id/edit` (no flag): Deck editor unchanged (load a mission, click-select a slot,
  drag it, undo — all behave as before the merge).

## Manual acceptance

- **S1:** spike page self-check `pass: true` on webgpu and forced webgl2 (paste both JSONs).
- **S2:** editor mount HUD shared-memory line reads PASS with the exact `2000/2000` count.
- **S3:** Deck editor smoke (flag off) — select/drag/undo unaffected.

## Documentation sync (Cursor, after merge)

Registry slice `T-151.0 → shipped` + `shipped_at`; program hub status line; verify-log link;
`./scripts/ticket sync && ./scripts/ticket check`.

## Claude Code prompt — T-151.0 (copy-paste)

Authority: this spec + handoff. **Do not edit docs/registry.**

```
Read CLAUDE.md first. Work in the WORKTREE at tbd-reforger-wgpu-spike/ on branch
t-151-wgpu-spike (NOT main — the worktree instruction supersedes the commit-to-main rule).

Implement **T-151.0** — wasm packaging merge + engine batch list + editor dual mount.

═══ PREFLIGHT ═══
  cd tbd-reforger-wgpu-spike
  git status --porcelain            # must be empty, branch t-151-wgpu-spike @ 94261dd6+
  git lfs pull && make map-assets-link
  cd apps/website/frontend && npm ci && cd ../../..
  make wasm                         # baseline pkg builds before you change anything

═══ READ (in order — spec wins on conflict) ═══
  1. .ai/artifacts/t151_0_claude_code_handoff.md
  2. docs/specs/Mission_Creator_Architecture/t151_0_wasm_merge_dual_mount.md
  3. docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md   (program context)
  4. crates/map-engine-render/src/engine.rs + apps/website/frontend/src/features/_spike/wgpu/*

═══ PROBLEM ═══
  The wgpu engine lives in its own --target web wasm pkg with its own linear memory, so
  MissionDoc and RenderEngine cannot share buffers (program D1 blocked). Draws are hardcoded
  (no batch seam for W1+ layers), and the engine is unreachable from the editor shell.

═══ SHIPPED (do not reopen) ═══
  T-151 spike @ 94261dd6 — OrthoCamera ULP-0 parity, chunked pool @ 20M, byte-exact
  self_check on webgpu + webgl2. All of it must stay green.

═══ LOCKED (full table: spec §Locked decisions L1–L11) ═══
  - One wasm module: map-engine-wasm depends on map-engine-render; cfg(wasm32) pub use
    RenderEngine; render crate-type ["rlib"]; --target web pkg deleted end to end
  - TS glue moves to features/tactical-map/wgpu/ (init memoization deleted; creation mutex,
    deviceSize + its test kept)
  - Batch list refactor is behavior-IDENTICAL (stats fields, self_check, draw order)
  - Engine flag: VITE_MC_ENGINE === 'wgpu' || ?engine=wgpu in MissionCreatorPage; Deck path
    untouched when off
  - WgpuTacticalMap reuses lifecycle invariants I2–I7 from WgpuCanvas verbatim
  - Shared-memory proof per L10: seed_random(1000, 12800, 12800, 0x12345678) → Float32Array
    over slot_xy_ptr → assert 2000/2000 finite ∧ in [0, 12800] → HUD PASS/FAIL line

═══ DO ═══
  1. Cargo merge (L1/L2); resolve a wasm-bindgen duplicate-start error per L3 only if it occurs
  2. make wasm; confirm RenderEngine + MissionDoc + OrthoCameraJs in map_engine_wasm.d.ts;
     record wc -c on map_engine_wasm_bg.wasm
  3. Remove make wasm-render + src/wasm/render/ + its .gitignore + eslint ignore entries (L4)
  4. Move glue + test (L5/L6); repoint WgpuCanvas.tsx; spike page must behave identically
  5. Refactor engine draws to the ordered Vec<Batch> (L7) — no behavior change
  6. WgpuTacticalMap.tsx + MissionCreatorPage flag switch + shared-memory HUD (L8/L9/L10)
  7. Write .ai/artifacts/t151_0_verify_log.md with every gate's verbatim output
  8. Commit prefix T-151.0: · tag T-151.0

═══ DO NOT ═══
  - Edit docs/**, .ai/tickets/registry.json, docs/TICKET_*.md, CLAUDE.md status markers
  - Touch main, the Deck TacticalMap render path, worldmap/*, workers/*, or any layer code
  - Add basemap/world/slot rendering (T-151.1+), new pipelines, or new engine features
  - Change stats() field names, self_check probes, or any spike gate expectation

═══ VERIFY (all exit 0) ═══
  cargo fmt --check
  cargo clippy --all-targets -- -D warnings
  cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings
  cargo test -p map-engine-core --all-features        # 66
  cargo test -p map-engine-render                     # 4
  cargo build --workspace
  make wasm
  cd apps/website/frontend && npm test && npm run build && npm run lint
  ! grep -l map_engine_wasm_bg dist/assets/index-*.js

═══ MANUAL ═══
  S1: /_spike/wgpu self-check pass:true on detected backend AND ?force=webgl (paste JSONs);
      20M stress: instances==20000000, staging_peak==67108864, uniform_bytes==64, fps noted
  S2: /missions/:id/edit?engine=wgpu HUD reads shared-memory: PASS (2000/2000 in [0,12800])
  S3: /missions/:id/edit without flag — Deck editor select/drag/undo unaffected

═══ RETURN ═══
  - Commit SHA + tag T-151.0
  - .ai/artifacts/t151_0_verify_log.md (all gate outputs + merged wasm byte size)
  - Automated verify output (PASS)
  - Manual notes for S1–S3 (self-check JSONs pasted)
  - **Ready for Cursor doc sync.**
```
