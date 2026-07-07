# T-151.0 — Claude Code handoff (wasm packaging merge + batch list + editor dual mount)

**Shipped:** @ `f019512d` (tag **T-151.0**, 2026-07-08) — verify log
[`t151_0_verify_log.md`](t151_0_verify_log.md). Cursor doc-sync pass follows.

**Spec (wins on conflict):**
[`t151_0_wasm_merge_dual_mount.md`](../../docs/specs/Mission_Creator_Architecture/t151_0_wasm_merge_dual_mount.md)
· **Program hub:**
[`t151_wgpu_engine_program.md`](../../docs/specs/Mission_Creator_Architecture/t151_wgpu_engine_program.md)
· **Working tree:** the standing worktree at `tbd-reforger-wgpu-spike/` (absolute:
`/var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike`) @ `94261dd6` or later —
**never `main`**. Do **not** run `./scripts/ticket run` (no nested worktrees). Do **not**
create or checkout slice branches — commit linearly on the worktree's current HEAD.

## Operator report

The T-151 spike shipped and every gate is green (verify log
[`t151_wgpu_spike_verify_log.md`](t151_wgpu_spike_verify_log.md)): deck.gl-parity
`OrthoCamera` (ULP 0 across a 300-case two-oracle corpus), a wgpu `RenderEngine` on
`SurfaceTarget::Canvas` with WebGPU→WebGL2 fallback, a chunked instance pool measured at
20,000,000 instances (58–70 fps across three browser/backend combos, `gpu_frame_ms`
13.9–14.4 ms on Chrome/webgpu), byte-exact readback self-check on both backends, and a
64-byte-per-frame navigation invariant. The program now wires the real Mission Creator onto
this engine; T-151.0 is the foundation slice.

## What you are building

Three things, behavior-preserving everywhere else:

1. **One wasm module (program D1).** Today there are two wasm packages with two separate
   linear memories: the bundler pkg (`make wasm` →
   `apps/website/frontend/src/wasm/pkg/map_engine_wasm*`, carrying `MissionDoc`,
   `OrthoCameraJs`, DEM/geometry, `SlotIndex`, `ClusterIndex`) and the spike's web-target pkg
   (`make wasm-render` → `src/wasm/render/map_engine_render*`, carrying `RenderEngine`).
   Zero-copy doc→GPU requires ONE memory: make `map-engine-render` a dependency of
   `map-engine-wasm`, re-export `RenderEngine` (cfg wasm32), delete the web-target product
   end to end. After `make wasm`, `import { RenderEngine, MissionDoc } from
   '@/wasm/pkg/map_engine_wasm'` must both work — from the same instance.
2. **Batch list seam.** `crates/map-engine-render/src/engine.rs` currently hardcodes two draw
   groups in `render()` (stress chunks loop, then the calibration buffer). Refactor to an
   ordered `Vec<Batch>` (`kind: PipelineKind::QuadInstanced`, buffer, count, visible) that the
   render pass iterates. Same draw order, same clear, same `stats()` JSON fields, `self_check`
   untouched — the spike gates are the regression harness proving "no behavior change".
3. **Editor dual mount (program D3).** New
   `apps/website/frontend/src/features/tactical-map/WgpuTacticalMap.tsx` accepting the
   existing `TacticalMapProps` (most ignored this slice), mounted by `MissionCreatorPage`
   when `VITE_MC_ENGINE === 'wgpu'` or `?engine=wgpu`. It renders the calibration scene
   full-bleed with a HUD showing backend, fps, and the **shared-memory numeric proof**
   (spec L10): `new MissionDoc()` → `seed_random(1000, 12800, 12800, 0x12345678)` →
   `refresh()` → `new Float32Array(wasmBg.memory.buffer, doc.slot_xy_ptr, 2000)` → count how
   many of the 2000 floats are finite and in [0, 12800] → render
   `shared-memory: PASS (2000/2000 in [0,12800])` or FAIL with the first offending index.
   Free the doc in the same effect cleanup that frees the engine.

## Do not

- Edit `docs/**`, `.ai/tickets/registry.json`, generated `docs/TICKET_*.md`, or CLAUDE.md
  sync markers (Cursor-owned).
- Touch `main`, the Deck `TacticalMap` render path, `worldmap/**`, `workers/**`, or any Deck
  layer module — the flag-off editor must be bit-identical in behavior.
- Add any rendering feature beyond the existing scene (no basemap, world objects, slots,
  new pipelines — those are T-151.1+).
- Change `stats()` field names, `self_check` probe expectations, camera math, or any spike
  gate expectation.
- Delete the `crates/map-engine-render` crate — it stays as a library crate; only its
  standalone wasm-pack product goes away.
- `git checkout -b`, create `ticket/T-151.x` branches, or run `./scripts/ticket run`.

## Execution order (strict)

1. Cargo merge → native + wasm32 compile clean.
2. `make wasm` → merged pkg; confirm `.d.ts` exports; record `wc -c` of the `_bg.wasm`.
3. Retire the web-target path (Makefile, `.gitignore`, eslint `globalIgnores`, delete
   `src/wasm/render/`).
4. Move the TS glue + test; repoint the spike page; verify the spike page still passes its
   self-check on the merged pkg (this is your mid-slice checkpoint).
5. Batch-list refactor; re-run spike gates.
6. `WgpuTacticalMap` + `MissionCreatorPage` flag + shared-memory HUD.
7. Full verify; write `.ai/artifacts/t151_0_verify_log.md`; commit `T-151.0: …`; tag `T-151.0`.

## Preflight

```bash
cd /var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike
test "$(git rev-parse --show-toplevel)" = "$(pwd)"
git status --porcelain             # empty @ baseline SHA
# Do NOT checkout or create branches; do NOT run ./scripts/ticket run
git lfs pull && make map-assets-link
cd apps/website/frontend && npm ci && cd ../../..
make wasm                          # baseline builds BEFORE changes
```

Toolchain (verified working on this machine): rustc/cargo 1.95.0, wasm-pack 0.15.0,
`wasm32-unknown-unknown` installed, node v26.4.0. `make …` cargo/wasm-pack steps may need
to run outside any sandbox (cargo registry + wasm-pack tool cache write to `~/.cargo`).

## Key files (surveyed — trust these locations)

| Concern | Path |
|---|---|
| Workspace members | `Cargo.toml` (root) |
| Bundler wasm shim (gains the dep + re-export) | `crates/map-engine-wasm/{Cargo.toml, src/lib.rs}` |
| Render crate (crate-type → `["rlib"]`) | `crates/map-engine-render/Cargo.toml` |
| Engine draws to refactor | `crates/map-engine-render/src/engine.rs` (`render()`, `seed_stress`, `clear_stress`) |
| Probe (do not change expectations) | `crates/map-engine-render/src/probe.rs` |
| Makefile targets `wasm` / `wasm-render` / `wasm-ci` | `Makefile` |
| Web-pkg ignore entries to remove | `.gitignore` (`src/wasm/render/`), `apps/website/frontend/eslint.config.js` `globalIgnores` |
| TS glue to move (keep mutex + deviceSize; drop init memoization) | `apps/website/frontend/src/features/_spike/wgpu/wasmRender.ts` → `features/tactical-map/wgpu/wasmRender.ts` |
| Glue test moves with it | `features/_spike/wgpu/deviceSize.test.ts` → `features/tactical-map/wgpu/deviceSize.test.ts` |
| Spike page (repoint imports; regression harness) | `features/_spike/wgpu/WgpuCanvas.tsx` |
| Lifecycle invariants I2–I7 to reuse verbatim | comments + structure in `WgpuCanvas.tsx` |
| Zero-copy view pattern to copy | `features/_spike/DocCoreSpikePage.tsx` lines 11–14, 42–47 (`import * as wasmBg from '@/wasm/pkg/map_engine_wasm_bg.wasm'`) |
| Editor page that gains the flag switch | `apps/website/frontend/src/features/mission-creator/MissionCreatorPage.tsx` (its `<TacticalMap …>` usage defines the props to accept) |
| Props contract type | `apps/website/frontend/src/features/tactical-map/TacticalMap.tsx` (`TacticalMapProps`) |

## Gotchas (learned the hard way — carry these)

- **wasm-bindgen start collision:** `map-engine-render` has `#[wasm_bindgen(start)]` (panic
  hook). `map-engine-wasm` currently has none, so linking should be fine — but if
  wasm-bindgen errors on a duplicate start, apply spec L3 (convert to an explicit
  `init_panic_hook()` export called from the glue) and note it in the verify log.
- **wasm handles in React are effect-local, never `useMemo`** — StrictMode double-invokes
  effects and `.free()` is not idempotent (memory `wasm-react-lifecycle`). `WgpuCanvas.tsx`
  I2–I7 are the reference implementation; the creation mutex in `wasmRender.ts` guarantees at
  most one live engine per canvas across the StrictMode interleave.
- **Bundler target needs no `init()`** — vite-plugin-wasm + top-level instantiation handle it
  (see `vite.config.ts` comments). Delete the web-target init memoization; ESM gives you the
  module singleton.
- **`make wasm` before any frontend test/build** — the pkg is gitignored; vitest imports it.
- **Vitest count baseline is 317** (312 pre-spike + orthoCamera.parity 2 + deviceSize 3).
  Moving `deviceSize.test.ts` must not change the count.
- **Entry-chunk isolation gate:** `! grep -l map_engine_wasm_bg dist/assets/index-*.js` —
  the wasm must be referenced only from lazy route chunks (it already is today via the
  code-split editor + spike routes; your changes must not pull it into the entry).
- **Prettier** on every touched TS file (`npx prettier --write <files>`); eslint bans
  `console.log` (warn/error allowed), `any`, non-null `!`.
- Harmless local noise: `npm warn Unknown env config "devdir"`, rpm-ostree/dconf lines in
  shell output — ignore.
- **20M stress on the merged pkg** allocates 640 MB GPU — expected numbers are in the spike
  verify log's cross-backend table; record yours, deviation is data, not failure.

## Verify commands

Spec §Verify verbatim (all exit 0), then the browser manuals S1–S3. Vitest/browser gate
philosophy: PASS/FAIL comes from printed JSON and integer counters, never appearance.

## Return to operator / Cursor

- Commit SHA + tag `T-151.0`
- `.ai/artifacts/t151_0_verify_log.md` — every gate's verbatim output, the merged
  `map_engine_wasm_bg.wasm` byte size, S1 self-check JSONs (both backends), S2 HUD line, S3
  smoke notes
- **Ready for Cursor doc sync.**

## Handoff vs spec vs prompt

Spec = decisions + gates (L1–L11). This handoff = context + file map + gotchas. The prompt
(spec §Claude Code prompt, extractable via `./scripts/ticket prompt T-151 --slice T-151.0`) =
the send-off. On conflict: spec wins.
