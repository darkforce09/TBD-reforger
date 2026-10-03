# GPU device

The `gpu_device` crate: the GPU of one browser canvas and the long-lived GPU bookkeeping around
it. `GpuContext` is the one bootstrap a renderer creates its GPU with (instance, surface,
adapter facts, device, queue and linear surface configuration, with resize and swapchain
acquire); beside it sit the web display handle, the pooled per-lane vertex buffers, the
one-mapping-at-a-time readback guard and the timestamp-query frame timer. It knows no map
concept: what a renderer draws is the renderer's business.

## Contents

```text
crates/graphics/gpu_device/
├── Cargo.toml  the package: `thiserror`; `wgpu` and `web-sys` on wasm32; layout tier 0, wasm32
└── src/        the context, buffers, timing, error type and prelude
```

## How it works

`GpuContext::create(canvas, force_webgl, label, want_timestamps)` prefers WebGPU and falls back
to WebGL2 (or uses WebGL2 alone when forced), requests the device under the caller's label with
timestamp queries when wanted and supported, and configures the surface with its first non-sRGB
format and `Fifo`. A renderer keeps the context, builds its own shaders and pipelines on its
device, takes each frame's image from `acquire` and resizes with `resize` after its own rounding
and clamping. Every failure is an `Error` whose message starts with a stable code. The source
README describes the modules.

## Getting started

Run from the repository root:

```bash
cargo test -p gpu_device                                              # pool, readback and surface policy tests
cargo clippy -p gpu_device --target wasm32-unknown-unknown -- -D warnings  # the browser half
cargo xtask verify crate-anatomy                                      # lib.rs, prelude, README and manifest shape
```

## Configuration

None: no feature and no environment variable. The browser half is selected by the `wasm32`
target.

## Public surface

- `GpuContext`, `Acquired`, `acquire_frame`, `WebDisplay`, `instance_descriptor` and `GpuTimer`
  (WebAssembly), `BackendKind`, `Error` and `Result`, at the root.
- `context`: `gpu_context`, `frame_acquire`, `web_display` and `surface_policy`.
- `buffers`: `pool::{LanePool, WriteOutcome, grow_capacity}` and `readback::ReadbackLane`.
- `timing::gpu_timer::GpuTimer`.
- `prelude`: the names most callers import.

## Boundaries

- Depends on: `thiserror`; `wgpu` and `web-sys` in the WebAssembly build. No workspace crate.
- Used by: the map renderer (`crates/map_rendering/map_renderer/`) and the paper doll renderer
  (`crates/paper_doll/paper_doll_renderer/`), which both create their GPU through `GpuContext`
  (the map renderer also times frames with `GpuTimer`); `symbology_layers_gpu`, whose slot
  symbology writes its sprite lanes through a `LanePool`.
- Rules: graphics tier 0 (`cargo xtask verify crate-tiers`); declares no map noun; a file that
  names a GPU or browser type gates itself on `wasm32`, so `cargo test -p gpu_device` runs
  natively.

## Related documentation

- [GPU rendering overview](/documentation/crates/graphics/gpu_rendering_overview.md) — the
  graphics crates and one frame across them.
