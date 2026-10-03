# Graphics crates

The map-agnostic renderer crates: building blocks that know nothing about what is being drawn,
only geometry, byte layouts, glyphs and shader text.

## Contents

```text
crates/graphics/
├── gpu_device/         `gpu_device`: the GPU context of a canvas, pooled lane buffers, readback guards, frame timer
├── gpu_frame/          `gpu_frame`: frame vocabulary, draw encoding, compute sprite cull, pipelines, frame pump
├── renderer_core/      `renderer_core`: lane sink, layer context, frame hooks, render statistics, packet binding ids
└── render_primitives/  `render_primitives`: GPU-free layouts, geometry, glyph packing and the WGSL source
```

## Boundaries

- Depends on: external crates and lower-tier workspace crates only.
- Used by: the map rendering crates (`crates/map_rendering/`), whose `map_renderer` implements
  `renderer_core`'s contracts and whose typed layers draw through `gpu_frame`; the paper doll
  renderer (`crates/paper_doll/paper_doll_renderer`), over `gpu_device`'s GPU context; the
  streaming and overlay crates and the single-page app (`apps/frontend`), over
  `render_primitives`' byte layouts and `gpu_frame`'s frame pump.
- Rules: a graphics crate declares `category = "crates/graphics"` (`cargo xtask verify
  crate-tiers`), and no name or document in it names a thing in the world being drawn.
