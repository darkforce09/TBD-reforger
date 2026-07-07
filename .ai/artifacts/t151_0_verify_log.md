# T-151.0 verify log — wasm packaging merge + engine batch list + editor dual mount

- **Worktree** (`git rev-parse --show-toplevel`): `/var/home/Samuel/Projects/TBD-Reforger/tbd-reforger-wgpu-spike`
- **Baseline HEAD** (parent of the T-151.0 commit): `16d19d288d6a02d99aa5170d2075c58b02b5dbe9`
  (spike `94261dd6` is an ancestor — `git merge-base --is-ancestor 94261dd6 HEAD` ✔)
- **Branch:** `t-151-wgpu-spike` (never `main`; no slice branch created)
- **Date:** 2026-07-08
- **Toolchain:** rustc/cargo 1.95.0, wasm-pack 0.15.0, wasm32-unknown-unknown installed, node v26.4.0

## Summary

All automated gates exit 0. One wasm module now carries `RenderEngine` + `MissionDoc` +
`OrthoCameraJs` in a single linear memory; the `--target web` product is removed end to end; the
engine draws iterate an ordered `Vec<Batch>` with byte-identical behavior; `WgpuTacticalMap` mounts
in the editor behind the engine flag with the L10 shared-memory proof. Browser GPU manuals S1–S3
are operator-run (no headless WebGPU/GPU browser available in this environment) — precise repro +
expected values below.

## L3 decision (start-fn collision)

**Not needed.** `map-engine-render` keeps its `#[wasm_bindgen(start)]` panic hook
(`engine.rs:42`). `map-engine-wasm` has no `start`, so linking the two crates produced no
wasm-bindgen duplicate-start error: `cargo check -p map-engine-wasm --target wasm32-unknown-unknown`
and `make wasm` both succeeded with the `(start)` attribute intact. No `init_panic_hook()`
conversion was required.

## Merged wasm byte size

```
baseline  (make wasm, pre-merge):  931424  apps/website/frontend/src/wasm/pkg/map_engine_wasm_bg.wasm
merged    (post-merge, pre-batch): 3657508 apps/website/frontend/src/wasm/pkg/map_engine_wasm_bg.wasm
merged    (post-batch-refactor):   3658383 apps/website/frontend/src/wasm/pkg/map_engine_wasm_bg.wasm
```

Delta baseline→merged = +2,726,959 bytes (≈ 2.6 MB engine payload — within the expected ~2.8 MB).
The batch-list refactor adds +875 bytes (the `PipelineKind`/`Batch` seam) — negligible.

`.d.ts` classes present (`grep -E "export class (RenderEngine|MissionDoc|OrthoCameraJs) "`):
```
export class MissionDoc {
export class OrthoCameraJs {
export class RenderEngine {
```
Full `RenderEngine` surface emitted: `create`, `render`, `self_check`, `seed_stress`,
`clear_stress`, `stats`, `backend`, `pan`, `zoom_at`, `resize`, `set_view`, `visible_bounds`,
`target_x`/`target_y`/`zoom` getters, `free`, `[Symbol.dispose]`.

## Automated gates (spec §Verify — all exit 0)

### `cargo fmt --check`
```
FMT OK   (exit 0)
```

### `cargo clippy --all-targets -- -D warnings`
```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 12.81s
clippy-all EXIT 0
```

### `cargo clippy -p map-engine-render --target wasm32-unknown-unknown -- -D warnings`
```
    Checking map-engine-render v0.1.0 (…/crates/map-engine-render)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.20s
clippy-render EXIT 0
```

### `cargo test -p map-engine-core --all-features`  (66 = 56 + 5 + 5)
```
running 56 tests
test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.28s
running 5 tests
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
running 5 tests
test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
running 0 tests
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
core-test EXIT 0
```

### `cargo test -p map-engine-render`  (4)
```
test scene::tests::stress_chunk_first_instances_pinned ... ok
test scene::tests::stress_chunk_is_deterministic_and_chunk_independent ... ok
test scene::tests::stress_chunk_domain_bounds ... ok
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
render-test EXIT 0
```

### `cargo build --workspace`
```
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.11s
build EXIT 0
```

### `make wasm`
```
[INFO]: ✨   Done in 36.34s
[INFO]: 📦   Your wasm pkg is ready to publish at apps/website/frontend/src/wasm/pkg.
wasm EXIT 0
```

### `npm test`  (vitest — 317, unchanged; `deviceSize.test.ts` move kept the count)
```
 Test Files  39 passed (39)
      Tests  317 passed (317)
   Duration  3.44s
```

### `npm run build`
```
✓ built in 578ms
build EXIT 0
```

### `npm run lint`
```
> eslint .
lint EXIT 0
```

### Entry-chunk isolation: `! grep -l map_engine_wasm_bg dist/assets/index-*.js`
```
PASS: wasm NOT in entry chunk (grep exit 1)
chunks that reference the wasm:
  map_engine_wasm-dxJ23z94.js        (the wasm module's own lazy chunk)
  worldObjects.worker-zWRK5Wzp.js    (the pre-existing world-objects worker)
WgpuTacticalMap code-splits into its own chunk: WgpuTacticalMap-QIBClmbI.js
```
The merged wasm loads only via lazy route/worker chunks; the entry `index-*.js` does not reference
it. Gate holds.

### Dev-server note — `WgpuTacticalMap` is lazy-loaded (dep-scan fix)

`MissionCreatorPage` imports `WgpuTacticalMap` via `React.lazy`, not a static import. A static
import made Vite's esbuild dependency **scan** (eager) hit `require.resolve` on
`@/wasm/pkg/map_engine_wasm_bg.wasm` and fail (`Cannot find module …`), because the esbuild scan
does not apply the `@/` tsconfig alias to the raw-wasm import. `DocCoreSpikePage` uses the identical
raw-wasm import without issue precisely because it is only reached through a dynamic import
(React.lazy), which the scan defers to runtime (where Vite's resolver handles `@/`). Lazy-loading
`WgpuTacticalMap` matches that pattern, fixes `make web`, and keeps the engine chunk (+ its wasm)
out of the flag-off editor graph entirely — it code-splits into `WgpuTacticalMap-*.js`, fetched only
when the engine flag is on. `npm run build`, `npm run lint`, `npm test` (317), and the entry-chunk
isolation gate all re-ran green after the change.

## Behavior-identical guarantees (L7 batch refactor)

- `render()` iterates `self.batches` (stress chunks first, calibration last) — the same draw order
  as the pre-batch-list engine (`for chunk in &self.stress { … } ; calibration draw(0..2)`), same
  `LoadOp::Clear(CLEAR_COLOR)`, same one `TIMESTAMP_QUERY` set, same `uniform_bytes_last_frame = 64`.
- `stats()` JSON field names + values unchanged: `instances` = `stress_instances`, `chunks` =
  `batches.len() - 1`, `gpu_bytes = stress_bytes + 64 + 32 + 64`, `staging_peak_bytes`, `gen_ms`,
  `upload_ms`, `uniform_bytes_last_frame`, `gpu_frame_ms`.
- `probe.rs` / `self_check` are untouched — they still clone the kept `calibration_buf` +
  `unit_quad_buf` fields directly. The calibration `Batch` holds a cheap `Arc` clone of the same
  buffer, so `clear_stress` never destroys it.

## Changed files (this slice)

Rust: `crates/map-engine-render/Cargo.toml` (crate-type → `["rlib"]`),
`crates/map-engine-render/src/engine.rs` (batch list), `crates/map-engine-wasm/Cargo.toml`
(+ render dep), `crates/map-engine-wasm/src/lib.rs` (`cfg(wasm32) pub use RenderEngine`),
`Cargo.lock`.
Build/ignore: `Makefile` (dropped `wasm-render`), `.gitignore`, `apps/website/frontend/eslint.config.js`.
TS: `features/_spike/wgpu/wasmRender.ts` → **moved** to `features/tactical-map/wgpu/wasmRender.ts`
(init memoization deleted; creation mutex + `deviceSize` + `WHEEL_ZOOM_PER_PX` kept);
`features/_spike/wgpu/deviceSize.test.ts` → **moved** to `features/tactical-map/wgpu/deviceSize.test.ts`;
`features/_spike/wgpu/WgpuCanvas.tsx` (import repoint only); **new**
`features/tactical-map/WgpuTacticalMap.tsx`; `features/mission-creator/MissionCreatorPage.tsx`
(engine flag switch). Generated `apps/website/frontend/src/wasm/render/` deleted.

Cursor-owned files (`docs/**`, `.ai/tickets/registry.json`, `docs/TICKET_*.md`, the handoff) were
**not** touched — any modifications to them in the working tree pre-date this slice and are left
uncommitted for Cursor's doc-sync pass.

## Manual acceptance S1–S3 (operator — browser GPU, numeric)

This environment has no browser automation, chrome/chromium binary, or headless WebGPU/GL, so the
GPU readback (`self_check`) and live-canvas checks are operator-run, exactly as for the shipped
spike. Run `make web`, open the URLs, read the printed JSON/counters (never appearance).

- **S1 — `/_spike/wgpu` on the merged pkg (regression harness for the merge):**
  - Click **Run self-check** on the auto-detected backend → expect `"pass": true`. Paste the JSON.
  - Open `/_spike/wgpu?force=webgl`, **Run self-check** → expect `"pass": true`. Paste the JSON.
  - Click **20M** → expect `stats()` `instances == 20000000`, `staging_peak_bytes == 67108864`,
    `uniform_bytes_last_frame == 64`. Note fps + `gpu_frame_ms` (shipped family: webgl2 58–70 fps,
    webgpu ~67 fps / ~14 ms — deviation is data, not failure).
  - _Result:_ ⏳ operator-pending.

- **S2 — `/missions/:id/edit?engine=wgpu`:** the calibration scene renders in the editor shell; the
  HUD reads `backend` + `shared-memory: PASS (2000/2000 in [0,12800])` (L10). The shared-memory
  proof is pure wasm (no GPU) and runs at mount regardless of backend.
  - _Result:_ ⏳ operator-pending.

- **S3 — `/missions/:id/edit` (no flag):** Deck editor unchanged — load a mission, click-select a
  slot, drag it, undo; all behave as before the merge (flag-off path is untouched).
  - _Result:_ ⏳ operator-pending.
