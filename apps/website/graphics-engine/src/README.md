# Graphics engine source

The source of `website-graphics-engine`: eight modules that together turn a caller's geometry and
frame packets into WebGPU draws, and the one library root that declares them.

## Contents

```text
apps/website/graphics-engine/src/
├── device/    pooled per-lane GPU buffers and readback fences
├── draw/      triangulation, composition, instance layouts, uploads, culling, the encoder
├── frame/     the frame vocabulary: packet, batches, ids, camera, present, damage, atlases
├── layout/    one list of every byte layout shared with callers, as re-exports
├── lib.rs     the library root; declares the eight modules, with no feature gates
├── loop/      the shared `requestAnimationFrame` pump and its `FrameTarget` trait
├── pipeline/  the render pipeline constructors and `create_map_shader`
├── shaders/   `SHADER_WGSL`: every vertex, fragment and compute entry point
└── text/      the bitmap font, the ASCII atlas bake, glyph layout and sprite bit-packing
```

## How it works

A caller builds its geometry with `draw` and `text`, uploads it into buffers, and records each
layer as a `DrawBatch` keyed by an opaque `LaneId`. Each frame it wraps the sorted batches in a
`frame::FramePacket` and runs `draw::encode::encode` between `frame::present::acquire` and
`frame::present::submit`; `frame::RenderDamage` decides whether the frame submits at all. The
pipelines the packet indexes come from `pipeline`, all compiled against `shaders`. `loop` calls
the caller's `FrameTarget` once per animation frame.

```text
caller geometry ─▶ draw / text ─▶ buffers ─▶ DrawBatch (lane, pipeline, payload)
                                                   │
FrameTarget::render_frame ◀── loop::RafPump        ▼
      │                                     frame::FramePacket
      └─▶ present::acquire ─▶ draw::encode::encode ─▶ present::submit
```

`lib.rs` declares every module without a gate. `wgpu`, `wasm-bindgen`, `js-sys` and `web-sys` are
dependencies of the `wasm32` target only, so each file that needs a GPU type carries its own
`#[cfg(target_arch = "wasm32")]`. The native build therefore keeps the CPU half (triangulation,
composition, instance layouts, text, damage, the cull oracle and the pump's `tick`), which the
native unit tests cover, while the browser build adds the uploads, the encoder, the pipelines, the
atlases, the present step, the compute cull and `RafPump::start`.

The modules share three conventions: positions are anchor-relative metres cast to f32 after the
caller's anchor is subtracted; colours are linear 0 to 1 on a non-sRGB target; and bind group 0 is
the camera, 1 a textured quad's texture, 2 an atlas.

## Public surface

- `frame`: `FramePacket`, `DrawBatch`, `DrawPayload`, `IndirectDraw`, `TextRun`, the buffer types,
  the three ids, `CameraUniform`, the atlas creators, `damage` and `present`.
- `draw`: triangulation, composition, grid lines, the uploads, `encode::encode` and `cull`.
- `layout`: the shared byte layouts, re-exported in one list.
- `pipeline`: `create_map_shader` and the pipeline constructors.
- `r#loop`: `FrameTarget` and `RafPump`.
- `device::buffers`: `LanePool` and `ReadbackLane`.
- `text`: the atlas bake, glyph metrics, layout and packing.
- `shaders::SHADER_WGSL`: the shader source.

## Boundaries

- Depends on: `bytemuck` and `earcutr`; `wgpu`, `wasm-bindgen`, `js-sys` and `web-sys` in the
  WebAssembly build. Nothing in the workspace.
- Used by: `website-map-engine` alone. Its `frame` module
  (`apps/website/map-engine/src/frame/mod.rs`) is the only file that names this crate's `frame`
  module, and with `apps/website/map-engine/src/frame/pump.rs` the only files that name `device`,
  `pipeline` or `r#loop`; its `world`, `overlay`, `spatial` and `diagnostics` modules import
  `draw`, `layout` and `text` directly. The frontend reaches this crate only through the map
  engine's re-exports.
- Rules: no module imports `website_map_engine` (`cargo xtask verify engine-layers`, rule 1); no
  declared name contains `terrain`, `symbology`, `mission`, `orbat` or `arma` (rule 2); a module
  that names a GPU type gates itself on `wasm32`, so `cargo test -p website-graphics-engine` runs
  natively.
