# Render primitives

The `render_primitives` crate: the building blocks of the renderer that hold no GPU handle. The
per-instance vertex layouts and the line vertex, colour normalisation, ear-clipping
triangulation, fill and hairline composition, the procedural grid, the CPU reference of sprite
culling, the frame ids, damage tracking and camera uniform, the bitmap font with its atlas bake,
glyph layout and sprite packing, and the WGSL source. The GPU crates (`gpu_frame` and the map
rendering crates) upload, bind and draw what these produce.

## Contents

```text
crates/graphics/render_primitives/
├── Cargo.toml  the package: `bytemuck` and `earcutr`, layout tier 0, every target
└── src/        the layouts, geometry, frame vocabulary, text, shader source and prelude
```

## How it works

A caller describes geometry in f64 world metres with an anchor; `draw::geometry::rel` folds it to
small f32 offsets before any byte is written. `draw::triangulate` and `draw::compose` turn rings
into indexed coloured meshes and segment lists into hairlines, `draw::grid` builds the grid's
line vertices, and `draw::instances` fixes the byte layouts of the three instanced draws. Colours
pass through `color_normalization`, the crate's one home for RGBA8 to linear floats.

Text is drawn as sprites: `text::atlas` bakes the built-in font into a 16 by 6 cell atlas,
`text::metrics` maps characters to cells, `text::layout` lays an already-chosen string out into
glyph instances and `text::pack` writes the 20-byte sprite records and the text uniform block,
the crate's one home for glyph packing.

`frame` holds the opaque ids a frame's batches carry, `RenderDamage` (whether a frame submits at
all) and the camera uniform. `shaders::SHADER_WGSL` is the one WGSL source; the tests read it as
text to pin every layout and uniform block against it, so a drift fails on the native target
without a GPU. `draw::cull::oracle` is the CPU reference the GPU compute cull must match.

## Getting started

Run from the repository root:

```bash
cargo test -p render_primitives   # the layout, geometry, text, damage, cull and shader tests
cargo xtask verify crate-anatomy  # lib.rs, prelude, README and manifest shape
```

## Configuration

None: no feature, no environment variable. Every target builds the whole crate.

## Public surface

- `color_normalization`: `norm` and `u8_rgba_to_f32`.
- `draw`: `compose`, `cull::oracle`, `geometry`, `grid`, `instances` and `triangulate`.
- `frame`: `camera::CameraUniform`, `damage::{RenderDamage, FrameDecision}` and
  `ids::{LaneId, PipelineId, BindGroupId}`.
- `text`: `atlas`, `font`, `layout`, `metrics`, `pack` and `scale`.
- `shaders::SHADER_WGSL`.
- `prelude`: the layouts, ids, meshes and glyph types most callers import.

## Boundaries

- Depends on: `bytemuck` and `earcutr`.
- Used by: `gpu_frame`, which builds its uploads, pipelines, encoder and compute cull on this
  crate; `renderer_core`; the map rendering crates (`crates/map_rendering/`), whose uploads,
  typed layers and readback checks import it directly; the streaming, overlay, terrain and world
  object crates, whose draw buffers, label packers and mesh composers pack its layouts; and the
  single-page app's building viewer (`apps/frontend`), which triangulates with it.
- Rules: graphics tier 0 with no workspace dependency (`cargo xtask verify crate-tiers`); no GPU
  handle and no browser API, so every test runs natively; no name or document names a thing in
  the world being drawn; every byte layout matches `shaders/shader.wgsl`, pinned by tests.

## Related documentation

- [Graphics engine overview](/documentation/crates/graphics/gpu_rendering_overview.md)
  — one frame across the renderer's modules.
- [Crate boundary rules](/documentation/standards/crate_boundary_rules.md) — the dependency
  directions between the workspace crates.
