# Graphics engine source

The source of `graphics_engine`: five modules that together turn a caller's geometry and frame
packets into WebGPU draws, and the one library root that declares them. The GPU-free layouts,
geometry, glyphs and WGSL source they build on are `render_primitives`
(`crates/graphics/render_primitives/`).

## Contents

```text
legacy/graphics_engine/src/
├── device/    pooled per-lane GPU buffers and readback fences
├── draw/      vertex and index uploads, the compute cull, the encoder
├── frame/     the GPU frame vocabulary: packet, batches, buffers, present, atlases
├── lib.rs     the library root; declares the five modules, with no feature gates
├── loop/      the shared `requestAnimationFrame` pump and its `FrameTarget` trait
└── pipeline/  the render pipeline constructors and `create_map_shader`
```

## How it works

A caller builds its geometry with `render_primitives` (`draw` and `text`), uploads it into buffers, and records each
layer as a `DrawBatch` keyed by an opaque `LaneId`. Each frame it wraps the sorted batches in a
`frame::FramePacket` and runs `draw::encode::encode` between `frame::present::acquire` and
`frame::present::submit`; `render_primitives::frame::damage::RenderDamage` decides whether the
frame submits at all. The
pipelines the packet indexes come from `pipeline`, all compiled against
`render_primitives::shaders::SHADER_WGSL`. `loop` calls
the caller's `FrameTarget` once per animation frame.

```text
caller geometry ─▶ draw / text ─▶ buffers ─▶ DrawBatch (lane, pipeline, payload)
                                                   │
FrameTarget::render_frame ◀── loop::RafPump        ▼
      │                                     frame::FramePacket
      └─▶ present::acquire ─▶ draw::encode::encode ─▶ present::submit
```

`lib.rs` declares every module without a gate. `wgpu`, `wasm-bindgen` and `web-sys` are
dependencies of the `wasm32` target only, so each file that needs a GPU type carries its own
`#[cfg(target_arch = "wasm32")]`. The native build therefore keeps the pump's `tick`, the buffer
pools and the compute cull's source check, which the native unit tests cover, while the browser build adds the uploads, the encoder, the pipelines, the
atlases, the present step, the compute cull and `RafPump::start`.

The modules share three conventions: positions are anchor-relative metres cast to f32 after the
caller's anchor is subtracted; colours are linear 0 to 1 on a non-sRGB target; and bind group 0 is
the camera, 1 a textured quad's texture, 2 an atlas.

## Public surface

- `frame`: `FramePacket`, `DrawBatch`, `DrawPayload`, `IndirectDraw`, `TextRun`, the buffer types,
  the atlas creators and `present`.
- `draw`: the uploads, `encode::encode` and `cull::compute`.
- `pipeline`: `create_map_shader` and the pipeline constructors.
- `r#loop`: `FrameTarget` and `RafPump`.
- `device::buffers`: `LanePool` and `ReadbackLane`.

## Boundaries

- Depends on: `bytemuck` and `render_primitives`; `wgpu`, `wasm-bindgen` and
  `web-sys` in the WebAssembly build.
- Used by: `map_engine` alone. Its `frame` module
  (`legacy/map_engine/src/frame/mod.rs`) is the only file that names this crate's `frame`
  module, and with `legacy/map_engine/src/frame/pump.rs` the only files that name `device`,
  `pipeline` or `r#loop`; its `world`, `overlay`, `spatial` and `diagnostics` modules import
  `draw::{lines, polygons, encode}` directly, and the GPU-free layouts and text from
  `render_primitives`. The frontend reaches this crate only through the map engine's re-exports.
- Rules: no module imports `map_engine` (`cargo xtask verify engine-layers`, rule 1); no
  declared name contains `terrain`, `symbology`, `mission`, `orbat` or `arma` (rule 2); a module
  that names a GPU type gates itself on `wasm32`, so `cargo test -p graphics_engine` runs
  natively.
