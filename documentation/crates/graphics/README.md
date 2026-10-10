**Status:** live

# Graphics crate documentation

The documentation of the map-agnostic graphics crates under `crates/graphics/`: `gpu_device`, the
GPU of a browser canvas, and `gpu_frame`, the frame vocabulary, draw path, pipelines and frame
pump, which together draw the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s
map for the map renderer and the Arsenal's paper doll, over the GPU-free `render_primitives`.
Developers and AI agents read it below the crates' code READMEs, for the frame flow across crates,
the design, the open work and the decisions.

## Contents

```text
documentation/crates/graphics/
└── gpu_rendering_overview.md  the graphics crates, one frame, sprite culling, design and open work
```

## Code

- [GPU device](/crates/graphics/gpu_device/README.md) — the GPU context, pooled buffers,
  readback guard and frame timer.
- [GPU frame](/crates/graphics/gpu_frame/README.md) — the frame vocabulary, draw path, pipelines
  and frame pump; its [source README](/crates/graphics/gpu_frame/src/README.md) draws the frame
  flow.
- [Renderer core](/crates/graphics/renderer_core/README.md) — the contracts a renderer and its
  typed layers meet at: lane sink, layer context, frame hooks, render statistics and the frame
  packet's binding ids.
- [Render primitives](/crates/graphics/render_primitives/README.md) — the GPU-free layouts,
  geometry, glyph packing and WGSL source both build on.

## Boundaries

- Depends on: the code of `crates/graphics/` and its callers in `crates/map_rendering/` and
  `crates/paper_doll/`,
  which every claim is checked against; the
  [feature doc template](/documentation/standards/templates/feature_doc.md); the ticket manager
  (`ttm`) for open work.
- Used by: the graphics crates' code READMEs, which link the overview under Related
  documentation; the [library crate documentation](/documentation/crates/README.md) index; the
  [crate boundary rules](/documentation/standards/crate_boundary_rules.md) and the
  [map rendering documentation](/documentation/crates/map_rendering/README.md).
- Rules: the overview describes the committed code; the layer rules these crates live under are
  the crate boundary rules standard, which the overview links rather than restates.

## Related documentation

- [Crate boundary rules](/documentation/standards/crate_boundary_rules.md) — the one-way
  arrow and the no-map-noun rule.
- [Map rendering documentation](/documentation/crates/map_rendering/README.md) — the renderer
  that builds each frame and calls these.
