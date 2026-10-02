**Status:** live

# Graphics engine documentation

The documentation of `graphics_engine`, the web platform's `wgpu` renderer, which knows no
map concept and draws the [Mission Creator](/documentation/glossary/g_to_m.md#mission-creator)'s map
for the map engine. Developers and AI agents read it below the crate's code READMEs, for the frame
flow across modules, the design, the open work and the decisions.

## Contents

```text
documentation/legacy/graphics_engine/
└── graphics_engine_overview.md  the crate's modules, one frame, sprite culling, design and open work
```

## Code

- [Graphics engine](/legacy/graphics_engine/) — the crate the overview describes; its
  [README](/legacy/graphics_engine/README.md) gives the commands and public surface, and
  its [source README](/legacy/graphics_engine/src/README.md) the frame flow.
- [Render primitives](/crates/graphics/render_primitives/) — the GPU-free layouts, geometry,
  glyph packing and WGSL source the engine builds on.

## Boundaries

- Depends on: the code of `legacy/graphics_engine/` and its caller in
  `legacy/map_engine/src/frame/`, which every claim is checked against; the
  [feature doc template](/documentation/standards/templates/feature_doc.md); the ticket
  registry in `.ai/tickets/` for open work.
- Used by: the graphics engine's code README, which links the overview under Related
  documentation; the [website documentation](/documentation/apps/README.md) index; the
  [engine boundary rules](/documentation/standards/engine_boundary_rules.md) and the
  [map engine documentation](/documentation/legacy/map_engine/README.md).
- Rules: the overview describes the committed code; the layer rules this crate lives under are
  the engine boundary rules standard, which the overview links rather than restates.

## Related documentation

- [Engine boundary rules](/documentation/standards/engine_boundary_rules.md) — the one-way
  arrow and the no-map-noun rule `cargo xtask verify engine-layers` enforces.
- [Map engine documentation](/documentation/legacy/map_engine/README.md) — the crate that
  builds each frame and calls this one.
