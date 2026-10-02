# Render primitives source

The source of `render_primitives`: the byte layouts, CPU geometry, glyph tables and shader text
the renderer builds on, and the crate root that declares them.

## Contents

```text
crates/graphics/render_primitives/src/
├── color_normalization.rs  `norm` and `u8_rgba_to_f32`: RGBA8 to linear 0 to 1 floats
├── draw/                   instance layouts, line geometry, triangulation, fills, grid, cull reference
├── frame/                  the GPU-free frame vocabulary: ids, damage tracking, camera uniform
├── lib.rs                  the crate root: module header and `mod` lines
├── prelude.rs              the layouts, ids and result types most callers import
├── shaders/                `SHADER_WGSL`: every vertex, fragment and compute entry point
└── text/                   the bitmap font, the ASCII atlas bake, glyph layout and sprite packing
```

## How it works

`lib.rs` declares six public modules and no item of its own. `color_normalization` is the one
place an RGBA8 colour becomes linear floats: `draw::grid` uses `norm`, `draw::compose` uses
`u8_rgba_to_f32`, which also folds a layer alpha into the alpha channel. `text::pack` and
`text::scale` are the one place glyph and sprite bytes are packed and glyph sizes are anchored to
zoom. `shaders` embeds `shader.wgsl` by a relative `include_str!`; the cull oracle and the layout
tests read that text to keep the CPU side and the WGSL source in agreement.

## Boundaries

- Depends on: `bytemuck` (plain-old-data layouts) and `earcutr` (triangulation).
- Used by: the graphics engine, through the crate root.
- Rules: nothing here names a GPU handle or a browser type, so every module compiles and is
  tested natively; nothing here names a thing in the world being drawn.
