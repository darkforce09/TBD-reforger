# Frame pump

The browser's `requestAnimationFrame` loop, written once and shared by every canvas the app
mounts. It drives any type that implements its `FrameTarget` trait, so the graphics engine never
names the renderer it runs.

## Contents

```text
apps/website/graphics-engine/src/loop/
├── mod.rs   the module tree; re-exports `FrameTarget` and `RafPump`
├── pump.rs  `FrameTarget` and `RafPump`: one frame's order, the contention rule and disposal
└── tests/   unit tests for the frame order, skipped frames, disposal and the per-frame hook
```

## How it works

The module is declared as `r#loop` in `src/lib.rs`, because `loop` is a Rust keyword. `RafPump`
holds the target as `Rc<RefCell<Option<T>>>`: shared, because resize and pointer handlers reach
the same target between frames, and optional, because the target is `None` until the
asynchronous GPU boot finishes. An `Arc<AtomicBool>` stops the loop.

`RafPump::tick` does one frame without touching the DOM:

```text
disposed? ── yes ──▶ return false (the loop drops its closure)
   │ no
try_borrow_mut ── busy ──▶ skip the frame, return true
   │ ok
target present? ── no ──▶ return true
   │ yes
render_frame ─▶ poll_device ─▶ frames += 1 ─▶ after_frame hook ─▶ return true
```

A busy target skips its frame rather than panicking, so a panic inside one render cannot turn
into a second "already borrowed" panic on the next frame and hide the first. Skipped frames
neither advance the count nor call the hook. `start`, in the WebAssembly build only, moves the
pump into a closure that reschedules itself through `window.request_animation_frame` until `tick`
returns false, and then drops itself; a missing `window` ends the loop.

## Boundaries

- Depends on: `std`; `wasm-bindgen` and `web-sys` in the WebAssembly build.
- Used by: `website-map-engine`, which re-exports `FrameTarget` and `RafPump` in
  `apps/website/map-engine/src/frame/pump.rs` and implements `FrameTarget` for `RenderEngine`
  there; through that re-export, the frontend starts a pump for the
  [Mission Creator](/documentation_v2/glossary.md#mission-creator)'s viewport
  (`apps/website/frontend/src/v2/apps/editor/bridge/viewport.rs`) and for the debug building
  viewer and world line-of-sight views (`apps/website/frontend/src/v2/apps/debug/`).
- Rules: a frame renders, then polls, then counts, then calls the hook
  (`a_frame_renders_then_polls`, `the_hook_runs_after_the_frame_and_sees_the_running_count`); a
  contended or unbooted target skips the frame and keeps the loop alive
  (`a_contended_target_skips_the_frame_and_keeps_the_loop_alive`,
  `a_target_that_has_not_booted_yet_is_skipped_without_stopping`); disposal renders nothing
  (`disposal_ends_the_loop_and_renders_nothing`); the map engine names `r#loop` only in
  `apps/website/map-engine/src/frame/mod.rs` and `apps/website/map-engine/src/frame/pump.rs`
  (`cargo xtask verify engine-layers`, rule 3b).
