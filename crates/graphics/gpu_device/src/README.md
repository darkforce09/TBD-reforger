# GPU device source

The source of `gpu_device`: the GPU context of one browser canvas, the long-lived GPU buffer
bookkeeping and the frame timer, and the crate root that declares them.

## Contents

```text
crates/graphics/gpu_device/src/
├── buffers/    per-lane pooled vertex buffers and one-at-a-time readback guards
├── context/    `GpuContext`, the swapchain acquire, the web display handle and the surface policy
├── error.rs    `Error` and `Result`: the stable-coded failures of create, resize and acquire
├── lib.rs      the crate root: module header, `mod` lines and the root re-exports
├── prelude.rs  the pool, readback, context, acquire and timer names most callers import
└── timing/     `GpuTimer`: the timestamp queries of one render pass and their readback
```

## How it works

A renderer creates its GPU with `context::gpu_context::GpuContext::create(canvas, force_webgl,
label, want_timestamps)`, which builds the instance over `context::web_display::WebDisplay`,
the surface over the canvas, the adapter, the device and queue and the linear surface
configuration, applying the checks of `context::surface_policy`. Each frame it takes an image
from `GpuContext::acquire`, encodes into it and submits it through `gpu_frame`; a resize goes
through `GpuContext::resize` after the renderer's own rounding and clamping. When timestamps are
on, the renderer creates a `timing::gpu_timer::GpuTimer` and hands its query set and buffers to
the submit.

`buffers::pool::LanePool` sizes and reuses the persistent vertex buffers a caller writes lane by
lane, and `buffers::readback::ReadbackLane` guards the `map_async` cycle of a readback buffer;
the timer keeps its readback in one.

Every file that names a `wgpu` or `web-sys` type compiles for `wasm32` only: `context/
gpu_context.rs`, `context/frame_acquire.rs`, `context/web_display.rs`, `timing/gpu_timer.rs` and
`LanePool::write_gpu`. The native build keeps the pool arithmetic, the readback guard, the
surface policy and the error type, which the native tests cover.

## Public surface

- `context::gpu_context::GpuContext` (also at the root): `create`, `resize`, `acquire` and the
  accessors of the instance, surface, adapter facts, device, queue and surface configuration.
- `context::frame_acquire`: `Acquired` and `acquire_frame` (also at the root).
- `context::web_display`: `WebDisplay` and `instance_descriptor` (also at the root).
- `context::surface_policy`: `BackendKind` (also at the root), `checked_canvas_size`,
  `checked_surface_size`, `first_linear_format`, `request_timestamps`.
- `buffers::pool`: `LanePool`, `WriteOutcome` and `grow_capacity`; `buffers::readback`:
  `ReadbackLane`.
- `timing::gpu_timer::GpuTimer` (also at the root); `Error` and `Result` at the root.

## Boundaries

- Depends on: `thiserror`; `wgpu` and `web-sys` (`HtmlCanvasElement`) in the WebAssembly build.
- Used by: `map_renderer`, whose render engine holds a `GpuContext` and a `GpuTimer`;
  `paper_doll_renderer`, whose doll renderer holds a `GpuContext`; and `symbology_layers_gpu`,
  whose slot symbology writes through `buffers::pool::LanePool`.
- Rules: no name or document here names a thing in the world being drawn; a file that names a
  GPU or browser type gates itself on `wasm32`.
