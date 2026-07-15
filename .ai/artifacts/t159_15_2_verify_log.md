# T-159.15.2 — verify log (Mission Creator camera pan + pointer foundation)

**Shipped:** tag `T-159.15.2` · **branch** `t-159-leptos-ui` · **worktree**
`.ai/artifacts/worktrees/TBD-T-159` · **baseline** `a425936d` (T-159.15.1) · **Date:** 2026-07-15

## What shipped

React-parity **MMB/RMB drag-pan** + **mid-pan wheel rebase** on the Leptos Mission Creator editor,
continuing the T-159.15.0/.15.1 boundary collapse (engine owned directly as Rust, D5). Pan maps DOM
pointer events → `engine.pan(dx_px, dy_px)`; Rust owns all ortho math (no JS unproject, no
`unproject_xy` — audit X-05). Slot pick / marquee / entity drag need `WasmMissionDoc` → **T-159.16+**.

- **Pan model — incremental `engine.pan`** (`camera.pan`: `target -= dx/scale; target += dy/scale`
  at the LIVE scale), mirroring the `WgpuCanvas` oracle (`engine.pan(clientX-lastX, …); lastX=clientX`),
  **not** the Deck `useSelectTool` frozen-viewport/JS-unproject path (forbidden by the language gate).
  `pan_px: Rc<Cell<Option<(f64,f64)>>>` holds the last client px while an MMB/RMB pan is in flight.
- **P1** MMB(1)+RMB(2) pan; LMB(0) untouched. **P3** `contextmenu` `preventDefault`. **P4** pointer
  capture on down, release on up/cancel. **P7** `disable_frame_timing()` + per-frame `poll()` kept;
  no GpuTimer (→ T-160). **P6** `window.__editorCam()` bridge exposes `{tx,ty,z,backend}` from the
  `&self` getters `target_x()/target_y()/zoom()/backend()` for the smoke.
- **P5 mid-pan rebase** is satisfied by construction + literally. The incremental model has **no
  frozen zoom to go stale**, so a mid-pan `zoom_at` needs no re-anchor — the next `engine.pan` delta
  divides by the LIVE post-zoom scale. Single-pointer invariant: a `pointermove` precedes any
  `wheel`, so `wheel.client == last_px`; the guarded one-liner `pan_px.set((wheel.client))` is a
  provable no-op that also defensively re-syncs the start px (and satisfies P5's wording verbatim).

## Gates (all green)

| Gate | Backend | Result |
|------|---------|--------|
| `cargo check -p website-leptos` (native shell) | — | clean (pre-existing dead-code warnings only) |
| `cargo check -p website-leptos --target wasm32-unknown-unknown` | — | clean |
| `trunk build --release` (from `apps/website-leptos`) | — | success → `dist/` |
| `smoke_editor.mjs` (15.1 wheel regression) | webgpu | `pass:true` — `viewChangedOnWheel`, no panic |
| `selfcheck_editor.mjs` (`?force=webgl`) | webgl2 | `pass:true` — calibration + texture byte-exact |
| **`smoke_pan_editor.mjs`** (NEW) | webgpu | `pass:true` — `panMoved && zoomChanged && panContinued`, no "already mapped" |

## Pan smoke — runtime matches the derived math to the decimal

Viewport 1440×900 dsf=1 → center (720,450); `scale = 2^z`; bounds `[0,12800]` (targets never clamped).

| Snapshot | Action | `tx` (runtime) | Derivation |
|----------|--------|----------------|------------|
| `cam0` | initial | **6400**, z −2 | `set_view(6400,6400,−2)` |
| `cam1` | Test A: RMB drag 720→520 (−200 px @ 0.25) | **7200** | 6400 + 200/0.25 = **+800** |
| `camB1` | Test B: move 720→680 (−40 px @ 0.25) | **7360** | 7200 + 40/0.25 = **+160** |
| `camB2` | wheel `deltaY −600` @ (680,450): z −2 → **−0.8** | 7269.644 | off-center zoom shifts target (cursor-anchored) |
| `camB3` | move 680→620 (−60 px @ 2⁻⁰·⁸≈0.5743), **no re-press** | 7374.110 | 7269.644 + 60/0.5743 = **+104.47** |

`panMoved` = |7200−6400| = 800 · `zoomChanged` = |−0.8−(−2)| = 1.2 · `panContinued` =
|7374.110−7269.644| = 104.47 — all ≫ 1e-6. `panContinued` proves the pan resumes after a mid-pan
zoom with **no re-press** (P5). `ty` stayed 6400 throughout (all gestures horizontal). No panics.

## Files

- `apps/website-leptos/src/mission_editor.rs` — `pan_px` Cell; `pointerdown/move/up/cancel` +
  `contextmenu` listeners; wheel rebase one-liner; `register_editor_cam` bridge (+ call).
- `apps/website-leptos/Cargo.toml` — web-sys `"PointerEvent"` + `"MouseEvent"` (`PointerEvent`
  transitively enables MouseEvent/UiEvent/Event; `set/has/release_pointer_capture` under `Element`).
- `.ai/artifacts/t159_gates/driver/smoke_pan_editor.mjs` — new Class-R pan smoke.
- `crates/map-engine-render/src/engine.rs` — **untouched** (`pan`/`zoom_at`/`set_view`/`target_*`
  getters already present; no `unproject_xy` resurrection).

## Deferred to T-159.16+ (LOCKED P9)

Slot pick, marquee, entity drag-move, clusters, `WasmMissionDoc` host, basemap/world loaders, Eden
chrome, LMB gestures. Frozen-viewport unproject for pick lands with the doc host + gesture machine.

## Next

**T-159.16** — `WasmMissionDoc` host (slot pick / spatial index). Ready for Cursor doc sync.
