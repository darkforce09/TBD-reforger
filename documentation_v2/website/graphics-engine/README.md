**Status:** live

# Graphics engine documentation

The documentation of `website-graphics-engine`, the web platform's `wgpu` renderer, which knows no
map concept and draws the [Mission Creator](/documentation_v2/glossary/g_to_m.md#mission-creator)'s map
for the map engine. Developers and AI agents read it below the crate's code READMEs, for the frame
flow across modules, the design, the open work and the decisions.

## Contents

```text
documentation_v2/website/graphics-engine/
└── graphics_engine_overview.md  the crate's modules, one frame, sprite culling, design and open work
```

## Code

- [Graphics engine](/apps/website/graphics-engine/) — the crate the overview describes; its
  [README](/apps/website/graphics-engine/README.md) gives the commands and public surface, and
  its [source README](/apps/website/graphics-engine/src/README.md) the frame flow.

## Boundaries

- Depends on: the code of `apps/website/graphics-engine/` and its caller in
  `apps/website/map-engine/src/frame/`, which every claim is checked against; the
  [feature doc template](/documentation_v2/standards/templates/feature_doc.md); the ticket
  registry in `.ai/tickets/` for open work.
- Used by: the graphics engine's code README, which links the overview under Related
  documentation; the [website documentation](/documentation_v2/website/README.md) index; the
  [engine boundary rules](/documentation_v2/standards/engine_boundary_rules.md) and the
  [map engine documentation](/documentation_v2/website/map-engine/README.md).
- Rules: the overview describes the committed code; the layer rules this crate lives under are
  the engine boundary rules standard, which the overview links rather than restates.

## Related documentation

- [Engine boundary rules](/documentation_v2/standards/engine_boundary_rules.md) — the one-way
  arrow and the no-map-noun rule `cargo xtask verify engine-layers` enforces.
- [Map engine documentation](/documentation_v2/website/map-engine/README.md) — the crate that
  builds each frame and calls this one.
