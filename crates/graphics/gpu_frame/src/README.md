# GPU frame source

The source of `gpu_frame`: four modules that together turn a caller's geometry and frame
packets into WebGPU draws, and the crate root that declares them. The GPU-free layouts,
geometry, glyphs and WGSL source they build on are `render_primitives`
(`crates/graphics/render_primitives/`); the device, queue, surface and acquire are `gpu_device`
(`crates/graphics/gpu_device/`).

## Contents

```text
crates/graphics/gpu_frame/src/
├── draw/        vertex and index uploads, the compute cull, the encoder
├── error.rs     `Error` and `Result`: a cell atlas whose pixels have the wrong length
├── frame/       the GPU frame vocabulary: packet, batches, buffers, glyph runs, atlases, submit
├── frame_pump/  the shared `requestAnimationFrame` pump and its `FrameTarget` trait
├── lib.rs       the crate root: module header and `mod` lines, with no feature gates
├── pipeline/    the render pipeline constructors and `create_render_shader`
├── prelude.rs   the frame vocabulary, the pump and the error type
└── tests/       the error message test
```

## How it works

A caller builds its geometry with `render_primitives` (`draw` and `text`), uploads it into
buffers, and records each layer as a `DrawBatch` keyed by an opaque `LaneId`. Each frame it wraps
the sorted batches in a `frame::FramePacket` and runs `draw::encode::encode` between
`gpu_device`'s acquire and `frame::present::submit`;
`render_primitives::frame::damage::RenderDamage` decides whether the frame submits at all. The
pipelines the packet indexes come from `pipeline`, all compiled against
`render_primitives::shaders::SHADER_WGSL`. `frame_pump` calls the caller's `FrameTarget` once
per animation frame.

```text
caller geometry ─▶ draw / text ─▶ buffers ─▶ DrawBatch (lane, pipeline, payload)
                                                   │
FrameTarget::render_frame ◀── frame_pump::RafPump  ▼
      │                                     frame::FramePacket
      └─▶ gpu_device acquire ─▶ draw::encode::encode ─▶ present::submit
```

`lib.rs` declares every module without a gate. `wgpu`, `wasm-bindgen` and `web-sys` are
dependencies of the `wasm32` target only, so each file that needs a GPU type carries its own
`#[cfg(target_arch = "wasm32")]`. The native build therefore keeps the pump's `tick` and the
compute cull's source check, which the native unit tests cover, while the browser build adds the
uploads, the encoder, the pipelines, the atlases, the submit step, the compute cull and
`RafPump::start`.

The modules share three conventions: positions are anchor-relative metres cast to f32 after the
caller's anchor is subtracted; colours are linear 0 to 1 on a non-sRGB target; and bind group 0 is
the camera, 1 a textured quad's texture, 2 an atlas.

## Boundaries

- Depends on: `bytemuck`, `render_primitives` and `thiserror`; `wgpu`, `wasm-bindgen` and `web-sys` in the
  WebAssembly build.
- Used by: `renderer_core`, the map rendering crates (`crates/map_rendering/`) and the
  single-page app's frame pumps (`crates/frontend/shell/frontend_application`).
- Rules: no module names a thing in the world being drawn; a module that names a GPU type gates
  itself on `wasm32`.
