# GPU frame

The `gpu_frame` crate: the GPU half of the renderer's frame. It defines the frame vocabulary a
caller describes one frame in (packet, batches, buffers, glyph runs, cell atlases) and supplies
the vertex and index uploads, the frame encoder, the compute sprite cull, the render pipeline
constructors, the swapchain submit and the shared `requestAnimationFrame` pump. The GPU-free
half it builds on (byte layouts, geometry, glyph packing, frame ids, damage tracking and the WGSL
source) is `render_primitives`; the device and queue it draws with come from `gpu_device`. It
knows no map concept.

## Contents

```text
crates/graphics/gpu_frame/
├── Cargo.toml  the package: `bytemuck`, `render_primitives`, `thiserror`; `wgpu`, `wasm-bindgen`, `web-sys` on wasm32; tier 1
└── src/        the frame vocabulary, the draw path, the pipelines and the frame pump
```

## How it works

A caller uploads its geometry with `draw::lines` and `draw::polygons` (or writes instance
buffers), records each layer as a `frame::DrawBatch` keyed by an opaque lane id, and each frame
wraps the sorted batches in a `frame::FramePacket`, encodes it with `draw::encode::encode`
between `gpu_device`'s acquire and `frame::present::submit`, and lets `frame_pump::RafPump` call it
once per animation frame. The pipelines the packet indexes come from `pipeline`, all compiled
against `render_primitives::shaders::SHADER_WGSL`. The source README draws the flow.

## Getting started

Run from the repository root:

```bash
cargo test -p gpu_frame                                              # pump and cull source tests
cargo clippy -p gpu_frame --target wasm32-unknown-unknown -- -D warnings  # the browser half
cargo xtask verify crate-anatomy                                     # lib.rs, prelude, README and manifest shape
```

## Configuration

None: no feature and no environment variable. The browser half is selected by the `wasm32`
target.

## Public surface

- `frame`: `FramePacket`, `DrawBatch`, `DrawPayload`, `IndirectDraw`, `TextRun`, the buffer
  types, the atlas creators and `present::{submit, TimestampResolve, frame_ms_ema}`.
- `draw`: `lines`, `polygons`, `encode::encode` and `cull::compute`.
- `pipeline`: `create_render_shader` and the pipeline constructors.
- `frame_pump`: `FrameTarget` and `RafPump`.
- `Error` and `Result` at the root: why a cell atlas could not be built.
- `prelude`: the frame vocabulary, the pump and the error type.

## Boundaries

- Depends on: `bytemuck`, `render_primitives` and `thiserror`; `wgpu`, `wasm-bindgen` and
  `web-sys` in the WebAssembly build.
- Used by: `renderer_core`; the map rendering crates `map_renderer`, `symbology_layers_gpu`,
  `world_layers_gpu` and `map_render_diagnostics`, which build and encode the frame packet; and
  the single-page app (`apps/frontend`, WebAssembly build only), whose frame pumps run on
  `frame_pump::RafPump`.
- Rules: graphics tier 1 (`cargo xtask verify crate-tiers`); declares no map noun; fields and
  variants name geometry and GPU handles only; a file that names a GPU or browser type gates
  itself on `wasm32`, so `cargo test -p gpu_frame` runs natively.

## Related documentation

- [GPU rendering overview](/documentation/crates/graphics/gpu_rendering_overview.md) — the
  graphics crates and one frame across them.
